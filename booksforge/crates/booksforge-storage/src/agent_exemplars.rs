//! Self-learning exemplar memory (Item 5 of FEATURE_HARDENING_PLAN).
//!
//! Persists accepted-quality paragraphs from successful agent runs.
//! Subsequent drafter calls load the top-K exemplars for the same
//! agent and inject them into the prompt as in-context examples,
//! compounding quality across runs.
//!
//! Untyped `sqlx::query()` is used here (rather than the typed
//! `sqlx::query!` macro the rest of `booksforge-storage` favours)
//! because the offline-cache `.sqlx/` JSON for the new
//! `agent_exemplars` table doesn't exist yet — regenerating it
//! requires a live SQLite with all migrations applied. The cost of
//! the trade-off is one fewer compile-time guarantee on three INSERT
//! / SELECT statements; the benefit is shipping in-session without
//! a sqlx-prepare detour. Migrate to typed macros after the offline
//! cache is regenerated for v9.

use chrono::{DateTime, Utc};
use sqlx::Row as _;
use ulid::Ulid;

use crate::{DbPool, StorageError};

/// One exemplar — a paragraph (or 2-3 paragraph chunk) the
/// drafter wrote in a previous run that scored above the
/// quality threshold.
#[derive(Debug, Clone)]
pub struct AgentExemplar {
    pub id: Ulid,
    pub project_id: Ulid,
    pub agent_id: String,
    pub snippet: String,
    pub snippet_word_count: i64,
    pub quality_score: f64,
    pub voice_profile_match: f64,
    pub tags: Vec<String>,
    pub source_run_id: Option<Ulid>,
    pub created_at: DateTime<Utc>,
}

/// Insert one exemplar. Returns the inserted row's ULID.
///
/// Idempotent on `(project_id, agent_id, snippet)` — calling twice
/// with the same prose for the same agent in the same project
/// inserts twice (each call gets its own ULID). Dedupe at the
/// caller if needed.
pub async fn insert_exemplar(
    pool: &DbPool,
    project_id: Ulid,
    agent_id: &str,
    snippet: &str,
    quality_score: f64,
    voice_profile_match: f64,
    tags: &[String],
    source_run_id: Option<Ulid>,
) -> Result<Ulid, StorageError> {
    let id = Ulid::new();
    let snippet_word_count = snippet.split_whitespace().count() as i64;
    let tags_json = serde_json::to_string(tags)?;
    let created_at = Utc::now();

    sqlx::query(
        r#"
        INSERT INTO agent_exemplars (
            id, project_id, agent_id, snippet, snippet_word_count,
            quality_score, voice_profile_match, tags,
            source_run_id, created_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(id.to_string())
    .bind(project_id.to_string())
    .bind(agent_id)
    .bind(snippet)
    .bind(snippet_word_count)
    .bind(quality_score)
    .bind(voice_profile_match)
    .bind(tags_json)
    .bind(source_run_id.map(|u| u.to_string()))
    .bind(created_at.to_rfc3339())
    .execute(pool)
    .await?;

    Ok(id)
}

/// Fetch the top-K exemplars for `agent_id` sorted by
/// `quality_score DESC, created_at DESC`. When `project_id` is
/// `Some(_)`, results are scoped to that project; when `None`,
/// returns exemplars across all projects (the default for
/// genre-pack-style cross-project learning).
pub async fn fetch_top_exemplars(
    pool: &DbPool,
    agent_id: &str,
    project_id: Option<Ulid>,
    limit: i64,
) -> Result<Vec<AgentExemplar>, StorageError> {
    let rows = if let Some(pid) = project_id {
        sqlx::query(
            r#"
            SELECT id, project_id, agent_id, snippet, snippet_word_count,
                   quality_score, voice_profile_match, tags,
                   source_run_id, created_at
            FROM   agent_exemplars
            WHERE  agent_id = ? AND project_id = ?
            ORDER  BY quality_score DESC, created_at DESC
            LIMIT  ?
            "#,
        )
        .bind(agent_id)
        .bind(pid.to_string())
        .bind(limit)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query(
            r#"
            SELECT id, project_id, agent_id, snippet, snippet_word_count,
                   quality_score, voice_profile_match, tags,
                   source_run_id, created_at
            FROM   agent_exemplars
            WHERE  agent_id = ?
            ORDER  BY quality_score DESC, created_at DESC
            LIMIT  ?
            "#,
        )
        .bind(agent_id)
        .bind(limit)
        .fetch_all(pool)
        .await?
    };

    let mut out: Vec<AgentExemplar> = Vec::with_capacity(rows.len());
    for r in rows {
        let id_s: String = r.try_get("id")?;
        let pid_s: String = r.try_get("project_id")?;
        let snippet: String = r.try_get("snippet")?;
        let snippet_word_count: i64 = r.try_get("snippet_word_count")?;
        let quality_score: f64 = r.try_get("quality_score")?;
        let voice_profile_match: f64 = r.try_get("voice_profile_match")?;
        let tags_s: String = r.try_get("tags")?;
        let source_run_id_s: Option<String> = r.try_get("source_run_id").ok();
        let created_s: String = r.try_get("created_at")?;
        let agent_id: String = r.try_get("agent_id")?;
        out.push(AgentExemplar {
            id: Ulid::from_string(&id_s).map_err(|e| StorageError::ConstraintViolation {
                detail: format!("invalid id ulid: {e}"),
            })?,
            project_id: Ulid::from_string(&pid_s).map_err(|e| {
                StorageError::ConstraintViolation {
                    detail: format!("invalid project_id ulid: {e}"),
                }
            })?,
            agent_id,
            snippet,
            snippet_word_count,
            quality_score,
            voice_profile_match,
            tags: serde_json::from_str(&tags_s).unwrap_or_default(),
            source_run_id: source_run_id_s
                .as_deref()
                .and_then(|s| Ulid::from_string(s).ok()),
            created_at: DateTime::parse_from_rfc3339(&created_s)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
        });
    }
    Ok(out)
}

/// Render the top-K exemplars as a prompt-ready block. Each
/// exemplar appears as `### Example N (quality X.X / 10)` followed
/// by the snippet. Returns the empty string when the exemplar list
/// is empty (so the drafter prompt template can render an
/// always-safe `{{ exemplars_block }}`).
pub fn render_exemplars_block(exemplars: &[AgentExemplar]) -> String {
    if exemplars.is_empty() {
        return String::new();
    }
    let mut out = String::new();
    out.push_str("# In-context exemplars from previous accepted prose\n\n");
    out.push_str(
        "These paragraphs were judged high-quality on previous runs of this \
         agent. Match their craft register — the level of sensory specificity, \
         the cadence of sentence variation, the type of metaphor — without \
         copying their content. Each example shows what HOUSE STYLE looks like \
         for this project; the prose you write must be original but rhythmically \
         and texturally adjacent.\n\n",
    );
    for (i, ex) in exemplars.iter().enumerate() {
        out.push_str(&format!(
            "### Example {} (quality {:.1} / 10):\n{}\n\n",
            i + 1,
            ex.quality_score,
            ex.snippet,
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{open_pool, run_migrations};

    async fn fresh_pool() -> (DbPool, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let pool = open_pool(&dir.path().join("test.db")).await.unwrap();
        run_migrations(&pool).await.unwrap();
        (pool, dir)
    }

    fn exemplar_text() -> &'static str {
        "The iron key scraped against the tumblers. Elara held her breath. \
         The lock gave with a heavy click. Red wax. It looked like a dried wound."
    }

    #[tokio::test]
    async fn insert_then_fetch_round_trip() {
        let (pool, _dir) = fresh_pool().await;
        let project = Ulid::new();
        let id = insert_exemplar(
            &pool,
            project,
            "scene-drafter-fic",
            exemplar_text(),
            8.5,
            0.92,
            &["interior".to_string(), "sensory".to_string()],
            None,
        )
        .await
        .unwrap();
        assert!(!id.to_string().is_empty());

        let got = fetch_top_exemplars(&pool, "scene-drafter-fic", Some(project), 5)
            .await
            .unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].snippet, exemplar_text());
        assert!((got[0].quality_score - 8.5).abs() < 0.01);
        assert_eq!(got[0].tags, vec!["interior", "sensory"]);
    }

    #[tokio::test]
    async fn fetch_orders_by_quality_then_recency() {
        let (pool, _dir) = fresh_pool().await;
        let project = Ulid::new();
        // Three exemplars, varying quality.
        let _ = insert_exemplar(
            &pool,
            project,
            "scene-drafter-fic",
            "low",
            5.0,
            0.5,
            &[],
            None,
        )
        .await
        .unwrap();
        let _ = insert_exemplar(
            &pool,
            project,
            "scene-drafter-fic",
            "high",
            9.0,
            0.9,
            &[],
            None,
        )
        .await
        .unwrap();
        let _ = insert_exemplar(
            &pool,
            project,
            "scene-drafter-fic",
            "mid",
            7.5,
            0.7,
            &[],
            None,
        )
        .await
        .unwrap();
        let got = fetch_top_exemplars(&pool, "scene-drafter-fic", Some(project), 10)
            .await
            .unwrap();
        assert_eq!(got.len(), 3);
        assert_eq!(got[0].snippet, "high");
        assert_eq!(got[1].snippet, "mid");
        assert_eq!(got[2].snippet, "low");
    }

    #[tokio::test]
    async fn fetch_top_k_caps_results() {
        let (pool, _dir) = fresh_pool().await;
        let project = Ulid::new();
        for i in 0..5 {
            let _ = insert_exemplar(
                &pool,
                project,
                "scene-drafter-fic",
                &format!("snippet {i}"),
                7.0 + i as f64 * 0.1,
                0.8,
                &[],
                None,
            )
            .await
            .unwrap();
        }
        let got = fetch_top_exemplars(&pool, "scene-drafter-fic", Some(project), 3)
            .await
            .unwrap();
        assert_eq!(got.len(), 3);
    }

    #[tokio::test]
    async fn fetch_filters_by_agent_id() {
        let (pool, _dir) = fresh_pool().await;
        let project = Ulid::new();
        let _ = insert_exemplar(
            &pool,
            project,
            "scene-drafter-fic",
            "draft",
            8.0,
            0.8,
            &[],
            None,
        )
        .await
        .unwrap();
        let _ = insert_exemplar(&pool, project, "world-bible", "world", 8.0, 0.8, &[], None)
            .await
            .unwrap();
        let drafter = fetch_top_exemplars(&pool, "scene-drafter-fic", Some(project), 10)
            .await
            .unwrap();
        let world = fetch_top_exemplars(&pool, "world-bible", Some(project), 10)
            .await
            .unwrap();
        assert_eq!(drafter.len(), 1);
        assert_eq!(world.len(), 1);
        assert_eq!(drafter[0].snippet, "draft");
        assert_eq!(world[0].snippet, "world");
    }

    #[tokio::test]
    async fn fetch_across_projects_when_project_id_none() {
        let (pool, _dir) = fresh_pool().await;
        let project_a = Ulid::new();
        let project_b = Ulid::new();
        let _ = insert_exemplar(
            &pool,
            project_a,
            "scene-drafter-fic",
            "A",
            8.0,
            0.8,
            &[],
            None,
        )
        .await
        .unwrap();
        let _ = insert_exemplar(
            &pool,
            project_b,
            "scene-drafter-fic",
            "B",
            8.0,
            0.8,
            &[],
            None,
        )
        .await
        .unwrap();
        let all = fetch_top_exemplars(&pool, "scene-drafter-fic", None, 10)
            .await
            .unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn render_block_empty_for_empty_list() {
        assert_eq!(render_exemplars_block(&[]), "");
    }

    #[test]
    fn render_block_includes_quality_score() {
        let ex = AgentExemplar {
            id: Ulid::new(),
            project_id: Ulid::new(),
            agent_id: "scene-drafter-fic".into(),
            snippet: "The brass key was cold.".into(),
            snippet_word_count: 5,
            quality_score: 8.7,
            voice_profile_match: 0.91,
            tags: vec!["sensory".into()],
            source_run_id: None,
            created_at: Utc::now(),
        };
        let block = render_exemplars_block(std::slice::from_ref(&ex));
        assert!(block.contains("In-context exemplars"));
        assert!(block.contains("Example 1 (quality 8.7 / 10)"));
        assert!(block.contains("The brass key was cold."));
        assert!(block.contains("HOUSE STYLE"));
    }
}
