-- Migration v9 — `agent_exemplars` ledger for the self-learning
-- exemplar memory subsystem (Item 5 of FEATURE_HARDENING_PLAN).
--
-- One row per accepted-quality paragraph. The drafter prompt for
-- subsequent runs loads the top-K exemplars (by quality score, then
-- recency) for the same agent and renders them into the prompt as
-- in-context examples. After ~50 successful runs the drafter has
-- internalised its own house style derived from its own best work
-- — the compounding-quality mechanism.
--
-- Schema decisions:
--   - `agent_id` is a stable string ('scene-drafter-fic',
--     'character-bible-card', etc.) matching `agent_runs.agent_id`.
--     Indexed because every read is `WHERE agent_id = ?`.
--   - `quality_score` is a 0.0-10.0 float computed by the
--     paragraph-quality scorer in `booksforge-voice`. Indexed in
--     descending order with `agent_id` so top-K reads are a single
--     index seek.
--   - `voice_profile_match` is the stylometric distance between this
--     paragraph and the agent's declared `VoiceTarget` (0.0-1.0).
--     Used as a secondary sort.
--   - `tags` is a JSON array of categorical labels
--     (e.g. ["dialogue-heavy", "interior", "action", "sensory"])
--     so future readers can filter by scene shape.
--   - `created_at` is ISO-8601 UTC. Used as the tiebreaker when
--     scores are equal (most-recent wins).

CREATE TABLE IF NOT EXISTS agent_exemplars (
    id                  TEXT PRIMARY KEY NOT NULL,           -- ULID
    project_id          TEXT NOT NULL,                       -- ULID; future filter "exemplars from this book only"
    agent_id            TEXT NOT NULL,                       -- e.g. 'scene-drafter-fic'
    snippet             TEXT NOT NULL,                       -- 1-3 paragraph chunk of accepted prose
    snippet_word_count  INTEGER NOT NULL,
    quality_score       REAL    NOT NULL CHECK (quality_score >= 0.0 AND quality_score <= 10.0),
    voice_profile_match REAL    NOT NULL CHECK (voice_profile_match >= 0.0 AND voice_profile_match <= 1.0),
    tags                TEXT    NOT NULL DEFAULT '[]',       -- JSON array
    source_run_id       TEXT,                                -- ULID of the run this came from (NULL for bootstrap)
    created_at          TEXT    NOT NULL                     -- ISO-8601 UTC
);

CREATE INDEX IF NOT EXISTS idx_agent_exemplars_agent_quality
    ON agent_exemplars(agent_id, quality_score DESC, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_agent_exemplars_project
    ON agent_exemplars(project_id);
