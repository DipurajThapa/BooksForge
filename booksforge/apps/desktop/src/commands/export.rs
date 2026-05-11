//! Manuscript export — Tauri commands.
//!
//! M0 ships the Markdown profile only.  The full Pandoc / EPUB-3 pipeline
//! lands in M5; until then writers can still take their work out of
//! BooksForge as a single `.md` file that Pandoc, Word, or any GitHub
//! preview will happily open.
//!
//! Steps:
//!   1. List every non-deleted node from storage (returns LexoRank-sorted).
//!   2. Bulk-load every saved scene's `pm_doc`.
//!   3. Convert each `pm_doc` to plain text via the export crate's
//!      Markdown converter.
//!   4. Render the whole manuscript as Markdown — H1 title, H2 parts,
//!      H3 chapters with sequential numbering, scene bodies in document
//!      order.
//!   5. Atomically write to the user-chosen path.
//!   6. Persist an `exports` ledger row keyed by blake3 of the bytes.

use std::collections::BTreeMap;
use std::path::Path;

use booksforge_domain::{ExportProfile, FormatProfile};
use booksforge_epubcheck::{java_on_path, run_epubcheck};
use booksforge_export::{
    manuscript_to_html_chapters, manuscript_to_markdown, pm_doc_to_html, pm_doc_to_markdown,
    ManuscriptInput,
};
use booksforge_export_epub::{EpubMetadata, EpubPackageInput};
use booksforge_export_pandoc::{pandoc_on_path, run_pandoc, PandocInput};
use booksforge_export_typst::{run_typst, typst_on_path, TypstInput, TypstTrim};
use booksforge_ipc::{
    BooksForgeError, ExportDependencyReport, ExportDependencyStatus, ExportHistoryEntry,
    ExportMarkdownInput, ExportMarkdownResult, ExportRunInput, ExportRunResult,
};
use booksforge_storage::StorageRepository;
use chrono::Utc;
use tauri::State;
use ulid::Ulid;

use crate::state::AppState;

#[tauri::command]
pub async fn export_markdown(
    input: ExportMarkdownInput,
    state: State<'_, AppState>,
) -> Result<ExportMarkdownResult, BooksForgeError> {
    let project = {
        let guard = state.open_project.lock().await;
        guard.as_ref().cloned()
    }
    .ok_or_else(|| BooksForgeError::internal("no project is open".to_owned()))?;

    let storage = &project.storage;

    // 1. Tree.
    let nodes = storage
        .list_nodes()
        .await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;
    if nodes.is_empty() {
        return Err(BooksForgeError::validation(
            "project has no scenes yet — generate or write some content before exporting"
                .to_owned(),
        ));
    }

    // 2. All scene content in one round-trip.
    let scene_rows = storage
        .list_all_scene_content()
        .await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;

    // 3. Convert each pm_doc to plain Markdown body.
    let mut scene_texts: BTreeMap<Ulid, String> = BTreeMap::new();
    for row in scene_rows {
        let text = pm_doc_to_markdown(&row.pm_doc);
        if !text.trim().is_empty() {
            scene_texts.insert(row.node_id, text);
        }
    }

    // 4. Render.
    let manuscript = ManuscriptInput {
        nodes,
        scene_texts,
        title: project.title.clone(),
        author: project.author.clone(),
    };
    let (rendered, stats) = manuscript_to_markdown(&manuscript);

    // 5. Validate + atomically write.
    let path = Path::new(&input.output_path);
    let parent = path.parent().ok_or_else(|| {
        BooksForgeError::validation("output_path must include a directory".to_owned())
    })?;
    if !parent.as_os_str().is_empty() && !parent.exists() {
        return Err(BooksForgeError::validation(format!(
            "output directory does not exist: {}",
            parent.display()
        )));
    }

    // Write to a sibling .tmp then rename — never leave a half-written file.
    let tmp = path.with_extension({
        let mut ext = path
            .extension()
            .map(|e| e.to_string_lossy().into_owned())
            .unwrap_or_default();
        ext.push_str(".tmp");
        ext
    });
    tokio::fs::write(&tmp, rendered.as_bytes())
        .await
        .map_err(|e| BooksForgeError::internal(format!("write failed: {e}")))?;
    tokio::fs::rename(&tmp, path)
        .await
        .map_err(|e| BooksForgeError::internal(format!("rename failed: {e}")))?;

    // 6. Ledger row.
    let hash = blake3::hash(rendered.as_bytes()).to_hex().to_string();
    let record = booksforge_domain::ExportRecord {
        id: Ulid::new(),
        profile: ExportProfile::Markdown,
        output_path: input.output_path.clone(),
        hash: hash.clone(),
        created_at: Utc::now(),
    };
    storage
        .export_insert(&record)
        .await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;

    Ok(ExportMarkdownResult {
        export_id: record.id.to_string(),
        output_path: input.output_path,
        bytes: stats.bytes,
        part_count: stats.part_count,
        chapter_count: stats.chapter_count,
        scene_count: stats.scene_count,
        word_count: stats.word_count,
        hash,
    })
}

// ── export_run ────────────────────────────────────────────────────────────────

/// Unified export entry-point (Phase 6 / BACKLOG H1+H2+H3+H4).  Routes to
/// the correct backend based on `profile`:
///
///   - `markdown`            → in-process Markdown renderer (no sidecar)
///   - `generic_epub` /
///     `kdp_ebook`           → in-process EPUB-3 packager + opt-in EPUBCheck
///   - `docx` / `trade_pdf*` → Pandoc subprocess
///
/// Persists an `exports` ledger row for every successful run.  EPUBCheck
/// is opt-in: if Java + the JAR aren't present, the export still
/// succeeds and `validation_message` carries a "local-only" hint.
#[tauri::command]
pub async fn export_run(
    input: ExportRunInput,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<ExportRunResult, BooksForgeError> {
    let project = {
        let guard = state.open_project.lock().await;
        guard.as_ref().cloned()
    }
    .ok_or_else(|| BooksForgeError::internal("no project is open".to_owned()))?;

    let profile = ExportProfile::from_str(&input.profile).ok_or_else(|| {
        BooksForgeError::validation(format!("unknown export profile: {}", input.profile))
    })?;

    // Resolve the genre-aware typography profile.  Empty / unknown
    // values fall back to the default (FictionTradeStandard).
    let format_profile = input
        .format_profile
        .as_deref()
        .and_then(FormatProfile::from_str)
        .unwrap_or_default();

    // Resolve the bundled-font directory from the Tauri resource
    // path (BACKLOG §H8.2 follow-up).  In dev, this is
    // `apps/desktop/resources/fonts/`; in shipped builds, the
    // platform-native resource location.  When the directory is
    // missing (unbundled dev build), fall back to `None` so the
    // packagers use the Google Fonts CDN / system install.
    let font_bundle_dir: Option<String> = resolve_font_bundle_dir(&app);

    // Atomic-write helper used by all branches.
    let outcome = match profile {
        ExportProfile::Markdown => export_markdown_inline(&project, &input.output_path).await?,
        ExportProfile::GenericEpub | ExportProfile::KdpEbook => {
            export_epub_inline(
                &project,
                &input.output_path,
                profile,
                format_profile,
                font_bundle_dir.as_deref(),
            )
            .await?
        }
        ExportProfile::Docx => {
            export_via_pandoc(
                &project,
                &input.output_path,
                profile,
                format_profile,
                font_bundle_dir.as_deref(),
            )
            .await?
        }
        ExportProfile::TradePdf5x8 | ExportProfile::TradePdf6x9 => {
            // BACKLOG §A11 — prefer typst (Apache-2.0, no LaTeX dependency,
            // ships as a single ~30 MB binary) over pandoc-via-LaTeX,
            // which fails out-of-the-box on macOS without a TeX install.
            // Pandoc-via-LaTeX remains the fallback when typst is missing.
            if typst_on_path().is_some() {
                export_via_typst(&project, &input.output_path, profile).await?
            } else {
                export_via_pandoc(
                    &project,
                    &input.output_path,
                    profile,
                    format_profile,
                    font_bundle_dir.as_deref(),
                )
                .await?
            }
        }
    };

    // Persist export row.
    let record = booksforge_domain::ExportRecord {
        id: Ulid::new(),
        profile,
        output_path: outcome.output_path.clone(),
        hash: outcome.hash.clone(),
        created_at: Utc::now(),
    };
    project
        .storage
        .export_insert(&record)
        .await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;

    // For EPUB profiles, attempt EPUBCheck validation.  KDP profile
    // additionally runs the structural KDP checks (G3) against the
    // built bytes — cover image, file size band, nav health, image
    // size band — and folds those findings into the same message.
    let (validation_ok, validation_message, error_count, warning_count) = if matches!(
        profile,
        ExportProfile::GenericEpub | ExportProfile::KdpEbook
    ) {
        let (mut ok, mut msg, mut errs, mut warns) =
            run_epubcheck_if_available(&outcome.output_path).await;
        if matches!(profile, ExportProfile::KdpEbook) {
            let (kdp_ok, kdp_msg, kdp_errs, kdp_warns) =
                run_kdp_checks_for(&outcome.output_path).await;
            ok = ok && kdp_ok;
            errs = errs.saturating_add(kdp_errs);
            warns = warns.saturating_add(kdp_warns);
            msg = match (msg, kdp_msg) {
                (Some(a), Some(b)) => Some(format!("{a}  ·  {b}")),
                (Some(a), None) => Some(a),
                (None, Some(b)) => Some(b),
                (None, None) => None,
            };
        }
        (ok, msg, errs, warns)
    } else {
        (true, None, 0u32, 0u32)
    };

    let bytes = tokio::fs::metadata(&outcome.output_path)
        .await
        .map(|m| m.len())
        .unwrap_or(0);

    Ok(ExportRunResult {
        export_id: record.id.to_string(),
        profile: profile.as_str().to_owned(),
        output_path: outcome.output_path,
        bytes,
        hash: outcome.hash,
        validation_ok,
        validation_message,
        error_count,
        warning_count,
    })
}

async fn export_markdown_inline(
    project: &std::sync::Arc<crate::state::OpenProject>,
    output_path: &str,
) -> Result<booksforge_export::ExportOutcome, BooksForgeError> {
    let manuscript = build_manuscript_input(project, /* html */ false).await?;
    let (rendered, _stats) = manuscript_to_markdown(&manuscript);
    write_atomic(output_path, rendered.as_bytes()).await?;
    Ok(booksforge_export::ExportOutcome {
        profile: ExportProfile::Markdown,
        output_path: output_path.to_owned(),
        hash: blake3::hash(rendered.as_bytes()).to_hex().to_string(),
    })
}

pub(crate) async fn export_epub_inline(
    project: &std::sync::Arc<crate::state::OpenProject>,
    output_path: &str,
    profile: ExportProfile,
    format_profile: FormatProfile,
    font_bundle_dir: Option<&str>,
) -> Result<booksforge_export::ExportOutcome, BooksForgeError> {
    let manuscript = build_manuscript_input(project, /* html */ true).await?;
    let html_chapters = manuscript_to_html_chapters(&manuscript);
    if html_chapters.is_empty() {
        return Err(BooksForgeError::validation(
            "manuscript has no chapters — write some content before exporting an EPUB".to_owned(),
        ));
    }
    let chapters: Vec<booksforge_export_epub::HtmlChapter> = html_chapters
        .iter()
        .map(|c| booksforge_export_epub::HtmlChapter {
            node_id: c.node_id.to_string(),
            title: c.title.clone(),
            html_body: c.html_body.clone(),
        })
        .collect();
    let metadata = EpubMetadata {
        title: project.title.clone(),
        authors: vec![project.author.clone()],
        language: "en".to_owned(),
        publisher: None,
        description: None,
        isbn: None,
        book_id: project.project_id.to_string(),
        dedication: None,
        epigraph: None,
        copyright_notice: None,
    };
    let outcome = booksforge_export_epub::build_epub(EpubPackageInput {
        chapters,
        metadata,
        profile,
        output_path: output_path.to_owned(),
        format_profile,
        font_bundle_dir: font_bundle_dir.map(|s| s.to_owned()),
    })
    .await
    .map_err(|e| BooksForgeError::internal(format!("EPUB build failed: {e}")))?;
    Ok(outcome)
}

pub(crate) async fn export_via_pandoc(
    project: &std::sync::Arc<crate::state::OpenProject>,
    output_path: &str,
    profile: ExportProfile,
    format_profile: FormatProfile,
    font_bundle_dir: Option<&str>,
) -> Result<booksforge_export::ExportOutcome, BooksForgeError> {
    let pandoc = pandoc_on_path().ok_or_else(|| {
        BooksForgeError::validation(
            "Pandoc not found on PATH.  Install Pandoc 3.x to use DOCX / PDF export.".to_owned(),
        )
    })?;
    let manuscript = build_manuscript_input(project, /* html */ false).await?;
    let (markdown, _stats) = manuscript_to_markdown(&manuscript);

    // Per-bundle DOCX reference template — drop a `reference.docx`
    // file in `<bundle>/exports/templates/` and the next DOCX export
    // picks it up automatically.  Authors who don't supply one get
    // Pandoc's defaults (still readable).  Resolution order:
    //   1. Env override `BOOKSFORGE_DOCX_TEMPLATE` (developer flow).
    //   2. Project-bundle path `<bundle>/exports/templates/reference.docx`.
    //   3. None — Pandoc default styling.
    // DOCX styling resolution (BACKLOG §H8.2 follow-up):
    //   1. User-supplied reference doc (env / per-profile / per-genre /
    //      generic) — `resolve_docx_template`.
    //   2. Auto-generated reference doc from `FormatProfile` — written
    //      to `<bundle>/exports/templates/.generated-<profile>.docx`
    //      so the file path passed to Pandoc is stable across runs.
    //
    // Either resolution yields a valid `.docx` reference; Pandoc uses
    // it to style the manuscript output.
    let docx_template = if matches!(profile, ExportProfile::Docx) {
        if let Some(p) = resolve_docx_template(project, format_profile) {
            Some(p)
        } else {
            Some(write_generated_docx_template(project, format_profile).await?)
        }
    } else {
        None
    };

    let outcome = run_pandoc(PandocInput {
        pandoc_binary: pandoc,
        markdown_source: markdown,
        docx_template,
        profile,
        output_path: output_path.to_owned(),
        format_profile,
        font_bundle_dir: font_bundle_dir.map(|s| s.to_owned()),
    })
    .await
    .map_err(|e| BooksForgeError::internal(format!("Pandoc export failed: {e}")))?;
    Ok(outcome)
}

/// PDF export via the `typst` sidecar (BACKLOG §A11).
///
/// Used in preference to pandoc-via-LaTeX for the `TradePdf*` profiles
/// when the typst binary is on PATH. Typst ships as a single ~30 MB
/// binary, has no LaTeX dependency, and is what the BF-E2E test
/// successfully used to render `manuscript.pdf` in the audit run.
pub(crate) async fn export_via_typst(
    project: &std::sync::Arc<crate::state::OpenProject>,
    output_path: &str,
    profile: ExportProfile,
) -> Result<booksforge_export::ExportOutcome, BooksForgeError> {
    let typst = typst_on_path().ok_or_else(|| {
        BooksForgeError::validation(
            "Typst not found on PATH. Install typst 0.14+ or fall back to Pandoc + LaTeX."
                .to_owned(),
        )
    })?;
    let manuscript = build_manuscript_input(project, /* html */ false).await?;
    let (markdown, _stats) = manuscript_to_markdown(&manuscript);
    let trim = match profile {
        ExportProfile::TradePdf5x8 => TypstTrim::Trade5x8,
        // TradePdf6x9 (and any other PDF trim that routes here) defaults
        // to the 6x9 trade profile.
        _ => TypstTrim::Trade6x9,
    };
    let outcome = run_typst(TypstInput {
        typst_binary: typst,
        markdown_source: markdown,
        output_path: output_path.to_owned(),
        trim,
        title: project.title.clone(),
        author: project.author.clone(),
    })
    .await
    .map_err(|e| BooksForgeError::internal(format!("Typst export failed: {e}")))?;
    Ok(booksforge_export::ExportOutcome {
        // Override profile so the audit row reflects what the caller asked for
        // (the typst crate hard-codes 6x9 in its own outcome shape).
        profile,
        output_path: outcome.output_path,
        hash: outcome.hash,
    })
}

/// Resolve the bundled Google Font directory at runtime (BACKLOG
/// §H8.2 follow-up).
///
/// Lookup order:
///   1. Env override `BOOKSFORGE_FONT_BUNDLE_DIR` — developer flow,
///      lets you point at a checked-in copy without re-bundling.
///   2. Tauri resource path (`<resource_dir>/resources/fonts`) — the
///      shipped layout configured in `tauri.conf.json`.
///   3. `None` — packagers fall back to the Google Fonts CDN
///      (EPUB) or the writer's system install (PDF).
fn resolve_font_bundle_dir(app: &tauri::AppHandle) -> Option<String> {
    use tauri::Manager as _;
    if let Ok(env_path) = std::env::var("BOOKSFORGE_FONT_BUNDLE_DIR") {
        let p = std::path::Path::new(&env_path);
        if p.is_dir() {
            return Some(env_path);
        }
    }
    let resource_dir = app.path().resource_dir().ok()?;
    let candidate = resource_dir.join("resources").join("fonts");
    if candidate.is_dir() {
        return Some(candidate.to_string_lossy().into_owned());
    }
    // Older bundlers flatten resources/* directly under the resource
    // dir.  Try that fallback too.
    let flat = resource_dir.join("fonts");
    if flat.is_dir() {
        return Some(flat.to_string_lossy().into_owned());
    }
    None
}

/// Auto-generate a `reference.docx` from a `FormatProfile` and persist
/// it under `<bundle>/exports/templates/.generated-<profile>.docx`.
///
/// Persisting it (rather than using a temp file) means:
///   - Pandoc's `--reference-doc=<path>` argument points at a stable
///     location the writer can inspect / edit / replace if they want
///     to override our defaults.
///   - The file is byte-deterministic for a given `FormatProfile`, so
///     we skip the rewrite when the bytes already match — keeps mtime
///     stable for git-tracking the bundle.
async fn write_generated_docx_template(
    project: &std::sync::Arc<crate::state::OpenProject>,
    format_profile: FormatProfile,
) -> Result<String, BooksForgeError> {
    let bytes = booksforge_export_pandoc::build_reference_docx(format_profile);
    let dir = project.bundle.exports().join("templates");
    if !dir.exists() {
        tokio::fs::create_dir_all(&dir)
            .await
            .map_err(|e| BooksForgeError::internal(format!("create templates dir: {e}")))?;
    }
    let path = dir.join(format!(".generated-{}.docx", format_profile.as_str()));

    // Skip the write when the existing file matches the generated bytes
    // — keeps mtime stable and avoids needless disk churn.
    let needs_write = match tokio::fs::read(&path).await {
        Ok(existing) => existing != bytes,
        Err(_) => true,
    };
    if needs_write {
        let tmp = path.with_extension("docx.tmp");
        tokio::fs::write(&tmp, &bytes)
            .await
            .map_err(|e| BooksForgeError::internal(format!("write generated docx: {e}")))?;
        tokio::fs::rename(&tmp, &path)
            .await
            .map_err(|e| BooksForgeError::internal(format!("rename generated docx: {e}")))?;
    }
    Ok(path.to_string_lossy().into_owned())
}

/// Resolve a DOCX reference template path for a given format profile.
///
/// Lookup order (BACKLOG §H8.2 — DOCX auto-styling routing):
///   1. Env override `BOOKSFORGE_DOCX_TEMPLATE` (developer flow,
///      bypasses the per-profile pick).
///   2. Per-format-profile bundle path
///      `<bundle>/exports/templates/reference-<profile_str>.docx`
///      (e.g. `reference-romance_regency.docx`).
///   3. Per-genre fallback
///      `<bundle>/exports/templates/reference-<genre>.docx`
///      (e.g. `reference-romance.docx`).
///   4. Generic `<bundle>/exports/templates/reference.docx` (legacy).
///   5. `None` — Pandoc default styling.
///
/// The programmatic `styles.xml` generator from `FormatProfile`
/// (which would mean "no reference doc needed") is tracked as a
/// follow-up; this lookup chain lets writers ship genre-specific
/// references in their bundle today and have BooksForge pick the
/// right one automatically.
fn resolve_docx_template(
    project: &std::sync::Arc<crate::state::OpenProject>,
    format_profile: FormatProfile,
) -> Option<String> {
    if let Ok(env_path) = std::env::var("BOOKSFORGE_DOCX_TEMPLATE") {
        if std::path::Path::new(&env_path).is_file() {
            return Some(env_path);
        }
    }
    let templates = project.bundle.exports().join("templates");

    // (2) Per-format-profile.
    let by_profile = templates.join(format!("reference-{}.docx", format_profile.as_str()));
    if by_profile.is_file() {
        return Some(by_profile.to_string_lossy().into_owned());
    }

    // (3) Per-genre fallback.
    let by_genre = templates.join(format!(
        "reference-{}.docx",
        format_profile.genre().as_str()
    ));
    if by_genre.is_file() {
        return Some(by_genre.to_string_lossy().into_owned());
    }

    // (4) Generic reference.
    let generic = templates.join("reference.docx");
    if generic.is_file() {
        return Some(generic.to_string_lossy().into_owned());
    }
    None
}

/// Build the in-memory manuscript input.  When `html=true`, scene
/// bodies are rendered as HTML fragments; otherwise plain Markdown.
pub(crate) async fn build_manuscript_input(
    project: &std::sync::Arc<crate::state::OpenProject>,
    html: bool,
) -> Result<ManuscriptInput, BooksForgeError> {
    let nodes = project
        .storage
        .list_nodes()
        .await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;
    if nodes.is_empty() {
        return Err(BooksForgeError::validation(
            "project has no scenes yet — generate or write some content before exporting"
                .to_owned(),
        ));
    }
    let scene_rows = project
        .storage
        .list_all_scene_content()
        .await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;
    let mut scene_texts: BTreeMap<Ulid, String> = BTreeMap::new();
    for row in scene_rows {
        let text = if html {
            pm_doc_to_html(&row.pm_doc)
        } else {
            pm_doc_to_markdown(&row.pm_doc)
        };
        if !text.trim().is_empty() {
            scene_texts.insert(row.node_id, text);
        }
    }
    Ok(ManuscriptInput {
        nodes,
        scene_texts,
        title: project.title.clone(),
        author: project.author.clone(),
    })
}

async fn write_atomic(output_path: &str, bytes: &[u8]) -> Result<(), BooksForgeError> {
    let path = Path::new(output_path);
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            return Err(BooksForgeError::validation(format!(
                "output directory does not exist: {}",
                parent.display()
            )));
        }
    }
    let tmp = format!("{output_path}.tmp");
    tokio::fs::write(&tmp, bytes)
        .await
        .map_err(|e| BooksForgeError::internal(format!("write failed: {e}")))?;
    tokio::fs::rename(&tmp, path)
        .await
        .map_err(|e| BooksForgeError::internal(format!("rename failed: {e}")))?;
    Ok(())
}

/// Locate the EPUBCheck JAR. Tries the explicit env override first,
/// then a small set of conventional sidecar install paths so the
/// out-of-the-box experience works after `brew install epubcheck`
/// (macOS) or the equivalent on Linux/Windows. Returns `None` when
/// no JAR can be located — the caller should treat that as
/// "validation skipped, export still succeeds."
fn resolve_epubcheck_jar() -> Option<String> {
    if let Ok(j) = std::env::var("BOOKSFORGE_EPUBCHECK_JAR") {
        if std::path::Path::new(&j).is_file() {
            return Some(j);
        }
    }
    // Sidecar default install paths in priority order. Limited to JAR
    // files installed by mainstream package managers — never scans
    // arbitrary filesystem locations.
    const DEFAULTS: &[&str] = &[
        // Homebrew on Apple Silicon (most common dev setup).
        "/opt/homebrew/opt/epubcheck/libexec/epubcheck.jar",
        // Homebrew on Intel macOS.
        "/usr/local/opt/epubcheck/libexec/epubcheck.jar",
        // Common Linux package manager destinations.
        "/usr/share/epubcheck/epubcheck.jar",
        "/usr/local/share/epubcheck/epubcheck.jar",
    ];
    for p in DEFAULTS {
        if std::path::Path::new(p).is_file() {
            return Some((*p).to_owned());
        }
    }
    None
}

/// Best-effort EPUBCheck.  Returns `(validation_ok, message, errors,
/// warnings)` — `validation_ok = true` when EPUBCheck passed OR when
/// it isn't installed (the export itself is reliable; validation is
/// extra assurance).
pub(crate) async fn run_epubcheck_if_available(
    epub_path: &str,
) -> (bool, Option<String>, u32, u32) {
    // Look for EPUBCheck JAR via env override (BOOKSFORGE_EPUBCHECK_JAR)
    // first, then fall back to the conventional Homebrew install
    // location on macOS so a `brew install epubcheck` works out of the
    // box without requiring users to set an env var. Bundled JAR shipping
    // is BACKLOG §M4 (deferred to release engineering).
    let jar = resolve_epubcheck_jar();
    let java = java_on_path();
    let (jar, java) = match (jar, java) {
        (Some(j), Some(jv)) => (j, jv),
        _ => {
            return (
                true,
                Some(
                    "EPUBCheck not configured — export succeeded but was not validated. \
             Install via `brew install epubcheck` or set BOOKSFORGE_EPUBCHECK_JAR + install Java."
                        .into(),
                ),
                0,
                0,
            )
        }
    };
    match run_epubcheck(epub_path, &jar, &java).await {
        Ok(report) => {
            let errors = report.error_count() as u32;
            let warnings = report.warning_count() as u32;
            let ok = report.is_valid();
            let msg = if ok && warnings == 0 {
                Some(format!("Validated by EPUBCheck {}", report.checker_version))
            } else if ok {
                Some(format!(
                    "EPUBCheck {} found {warnings} warning(s)",
                    report.checker_version
                ))
            } else {
                Some(format!(
                    "EPUBCheck {} flagged {errors} error(s) and {warnings} warning(s)",
                    report.checker_version
                ))
            };
            (ok, msg, errors, warnings)
        }
        Err(e) => (true, Some(format!("EPUBCheck unavailable: {e}")), 0, 0),
    }
}

// ── export_check_dependencies ────────────────────────────────────────────────

/// Probe the export pipeline's external dependencies (Pandoc, Java,
/// EPUBCheck JAR) and return their status.  Pure read-only — no
/// side-effects, no persistence.  Powers the export panel's "needs
/// pandoc" / "needs java" badges so users know what to install before
/// they try a profile that won't work.
///
/// Discovery rules per binary:
///
///   - **Pandoc** — env `BOOKSFORGE_PANDOC_BIN`, then PATH.
///   - **Java**   — env `JAVA_HOME/bin/java`, env `BOOKSFORGE_JAVA_BIN`,
///                  then PATH (we don't dictate Java for the user).
///   - **EPUBCheck JAR** — env `BOOKSFORGE_EPUBCHECK_JAR` only (no
///                  conventional install path; users opt in explicitly).
#[tauri::command]
pub async fn export_check_dependencies() -> Result<ExportDependencyReport, BooksForgeError> {
    use booksforge_epubcheck::java_on_path;
    use booksforge_export_pandoc::{pandoc_on_path, probe_pandoc};

    let mut items = Vec::new();

    // ── Pandoc ──
    let pandoc_path = std::env::var("BOOKSFORGE_PANDOC_BIN")
        .ok()
        .filter(|p| std::path::Path::new(p).is_file())
        .or_else(pandoc_on_path);
    let pandoc_version = if let Some(ref p) = pandoc_path {
        probe_pandoc(p).await.ok().unwrap_or_default()
    } else {
        String::new()
    };
    items.push(ExportDependencyStatus {
        id:           "pandoc".into(),
        name:         "Pandoc".into(),
        found:        pandoc_path.is_some(),
        path:         pandoc_path.unwrap_or_default(),
        version:      pandoc_version,
        unlocks:      vec!["docx".into(), "trade_pdf_5x8".into(), "trade_pdf_6x9".into()],
        install_hint: "Download from https://pandoc.org/installing.html or your OS package manager (brew install pandoc / choco install pandoc / apt install pandoc).".into(),
    });

    // ── Typst (BACKLOG §A11) ──
    // Preferred PDF engine for Trade PDF profiles when present. Falls
    // back to Pandoc + LaTeX when missing.
    {
        use booksforge_export_typst::{probe_typst, typst_on_path};
        let typst_path = std::env::var("BOOKSFORGE_TYPST_BIN")
            .ok()
            .filter(|p| std::path::Path::new(p).is_file())
            .or_else(typst_on_path);
        let typst_version = if let Some(ref p) = typst_path {
            probe_typst(p).await.ok().unwrap_or_default()
        } else {
            String::new()
        };
        items.push(ExportDependencyStatus {
            id:           "typst".into(),
            name:         "Typst".into(),
            found:        typst_path.is_some(),
            path:         typst_path.unwrap_or_default(),
            version:      typst_version,
            unlocks:      vec!["trade_pdf_5x8".into(), "trade_pdf_6x9".into()],
            install_hint: "Apache-2.0 single-binary PDF engine. brew install typst (macOS) / scoop install typst (Windows) / cargo install typst-cli (any). Replaces Pandoc + LaTeX for the Trade PDF profiles.".into(),
        });
    }

    // ── Java ──
    let java_path: Option<String> = std::env::var("BOOKSFORGE_JAVA_BIN")
        .ok()
        .filter(|p| std::path::Path::new(p).is_file())
        .or_else(|| {
            std::env::var("JAVA_HOME").ok().and_then(|home| {
                let candidate =
                    std::path::Path::new(&home)
                        .join("bin")
                        .join(if cfg!(target_os = "windows") {
                            "java.exe"
                        } else {
                            "java"
                        });
                if candidate.is_file() {
                    Some(candidate.to_string_lossy().into_owned())
                } else {
                    None
                }
            })
        })
        .or_else(java_on_path);
    let java_version = if let Some(ref p) = java_path {
        probe_java(p).await.unwrap_or_default()
    } else {
        String::new()
    };
    items.push(ExportDependencyStatus {
        id:           "java".into(),
        name:         "Java (JRE)".into(),
        found:        java_path.is_some(),
        path:         java_path.unwrap_or_default(),
        version:      java_version,
        unlocks:      vec!["epubcheck".into()],
        install_hint: "Install a Java 11+ runtime (Temurin / Zulu / system OpenJDK) — only needed to run EPUBCheck on EPUB exports.".into(),
    });

    // ── EPUBCheck JAR ──
    let jar = resolve_epubcheck_jar();
    items.push(ExportDependencyStatus {
        id:           "epubcheck".into(),
        name:         "EPUBCheck".into(),
        found:        jar.is_some(),
        path:         jar.unwrap_or_default(),
        version:      String::new(),
        unlocks:      vec!["epub_validation".into()],
        install_hint: "Install via `brew install epubcheck` (auto-detected) or set BOOKSFORGE_EPUBCHECK_JAR to the .jar path.  Optional — EPUB export still works without it.".into(),
    });

    Ok(ExportDependencyReport { items })
}

/// Read the built EPUB back from disk and run the KDP structural
/// checks against its bytes.  Returns `(ok, message, errors, warnings)`
/// — `ok = false` only if a KDP-Error finding fired (e.g. file
/// exceeds 650 MB or the archive is malformed).  Read failures are
/// surfaced as a soft warning so the export itself still counts as
/// successful.
async fn run_kdp_checks_for(epub_path: &str) -> (bool, Option<String>, u32, u32) {
    use booksforge_export_epub::{run_kdp_checks, KdpSeverity};
    let bytes = match tokio::fs::read(epub_path).await {
        Ok(b) => b,
        Err(e) => {
            return (
                true,
                Some(format!("KDP checks skipped — could not re-read EPUB: {e}")),
                0,
                0,
            )
        }
    };
    let findings = run_kdp_checks(&bytes);
    let errors: u32 = findings
        .iter()
        .filter(|f| f.severity == KdpSeverity::Error)
        .count() as u32;
    let warnings: u32 = findings
        .iter()
        .filter(|f| f.severity == KdpSeverity::Warning)
        .count() as u32;
    let ok = errors == 0;
    let msg = if findings.is_empty() {
        Some("KDP structural checks: clean.".to_owned())
    } else {
        // Render up to 3 findings inline; surface the rest as a count.
        let preview: Vec<String> = findings
            .iter()
            .take(3)
            .map(|f| format!("[{:?} {}] {}", f.severity, f.code, f.message))
            .collect();
        let extra = findings.len().saturating_sub(preview.len());
        if extra > 0 {
            Some(format!("KDP: {} (+{extra} more)", preview.join("  ·  ")))
        } else {
            Some(format!("KDP: {}", preview.join("  ·  ")))
        }
    };
    (ok, msg, errors, warnings)
}

async fn probe_java(java: &str) -> Option<String> {
    use tokio::process::Command;
    let out = Command::new(java).arg("-version").output().await.ok()?;
    // Java prints version to stderr, not stdout.
    let s = String::from_utf8_lossy(&out.stderr);
    s.lines().next().map(|l| l.trim().to_owned())
}

// ── publishing_targets_list ───────────────────────────────────────────────────

/// One publishing-target row for the UI picker. Mirrors
/// `booksforge_domain::TargetSpec` but flattens the parts the UI
/// needs into a JSON-friendly shape (no `&'static [...]` slices).
#[derive(Debug, Clone, serde::Serialize)]
pub struct PublishingTargetRow {
    pub id: String,
    pub label: String,
    pub blurb: String,
    pub user_briefing: String,
    pub artifact_formats: Vec<String>,
    pub allowed_trims: Vec<TrimRow>,
    pub identifier_scheme: String,
    pub toc_depth_max: u8,
    pub image_min_dpi: u32,
    pub cover_min_px: (u32, u32),
    pub cover_aspect_x100: u32,
    pub fonts_embedded_required: bool,
    pub pdfx_required: bool,
    pub accessibility_required: bool,
    pub epubcheck_required: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TrimRow {
    pub label: String,
    pub width_in: f32,
    pub height_in: f32,
}

/// List every supported `PublishingTarget` with its compliance spec
/// flattened for the UI. Read-only; no project context required.
#[tauri::command]
pub async fn publishing_targets_list() -> Result<Vec<PublishingTargetRow>, BooksForgeError> {
    use booksforge_domain::PublishingTarget;
    let mut out = Vec::with_capacity(PublishingTarget::all().len());
    for t in PublishingTarget::all() {
        let s = t.spec();
        out.push(PublishingTargetRow {
            id: t.as_str().to_owned(),
            label: s.label.to_owned(),
            blurb: s.blurb.to_owned(),
            user_briefing: s.user_briefing.to_owned(),
            artifact_formats: s
                .artifact_formats
                .iter()
                .map(|a| {
                    match a {
                        booksforge_domain::ArtifactFormat::PdfX1a => "pdf_x1a",
                        booksforge_domain::ArtifactFormat::Pdf => "pdf",
                        booksforge_domain::ArtifactFormat::Epub3 => "epub3",
                        booksforge_domain::ArtifactFormat::Epub2 => "epub2",
                        booksforge_domain::ArtifactFormat::Docx => "docx",
                        booksforge_domain::ArtifactFormat::Markdown => "markdown",
                    }
                    .to_owned()
                })
                .collect(),
            allowed_trims: s
                .allowed_trims
                .iter()
                .map(|(l, w, h)| TrimRow {
                    label: (*l).to_owned(),
                    width_in: *w,
                    height_in: *h,
                })
                .collect(),
            identifier_scheme: match s.identifier_scheme {
                booksforge_domain::IdentifierScheme::UrnIsbn => "urn_isbn".to_owned(),
                booksforge_domain::IdentifierScheme::UrnIsbnPreferred => {
                    "urn_isbn_preferred".to_owned()
                }
                booksforge_domain::IdentifierScheme::UrnBfProject => "urn_bf_project".to_owned(),
            },
            toc_depth_max: s.toc_depth_max,
            image_min_dpi: s.image_min_dpi,
            cover_min_px: s.cover_min_px,
            cover_aspect_x100: s.cover_aspect_x100,
            fonts_embedded_required: s.fonts_embedded_required,
            pdfx_required: s.pdfx_required,
            accessibility_required: s.accessibility_required,
            epubcheck_required: s.epubcheck_required,
        });
    }
    Ok(out)
}

// ── export_history ────────────────────────────────────────────────────────────

/// List previous exports for the open project, newest first.
#[tauri::command]
pub async fn export_history(
    state: State<'_, AppState>,
) -> Result<Vec<ExportHistoryEntry>, BooksForgeError> {
    let project = {
        let guard = state.open_project.lock().await;
        guard.as_ref().cloned()
    }
    .ok_or_else(|| BooksForgeError::internal("no project is open".to_owned()))?;
    let rows = project
        .storage
        .list_exports()
        .await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;
    Ok(rows
        .into_iter()
        .map(|r| ExportHistoryEntry {
            id: r.id.to_string(),
            profile: r.profile.as_str().to_owned(),
            output_path: r.output_path,
            hash: r.hash,
            created_at: r.created_at.to_rfc3339(),
        })
        .collect())
}
