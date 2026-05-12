//! Prepare-for-Publishing single-action workflow (Phase 7 of
//! `PRODUCT_ROADMAP_E2E.md`, closes UX recommendation R4).
//!
//! One command bundles every artefact each marketplace needs into a
//! per-platform subdirectory under `<bundle>/exports/<platform>/`,
//! along with a `READY_TO_UPLOAD.md` instruction file and a JSON
//! readiness checklist. The user clicks once; the system either hands
//! them a ready-to-upload package, or surfaces exactly what's missing
//! and how to fix it (HUMAN_REQUIRED items).
//!
//! Per-platform contents:
//!
//! ```text
//! <bundle>/exports/
//!   kdp/
//!     manuscript.epub                     ← ebook
//!     manuscript.pdf                      ← print interior (typst, 6×9)
//!     metadata.kdp.csv                    ← KDP metadata schema
//!     cover_brief.md                      ← cover-art brief (HUMAN_REQUIRED to commission)
//!     READY_TO_UPLOAD.md                  ← step-by-step instructions
//!     readiness.json                      ← per-item PASS/WARN/FAIL/HUMAN_REQUIRED
//!   google_play/
//!     manuscript.epub
//!     manuscript.pdf
//!     metadata.gp.json
//!     cover_brief.md
//!     READY_TO_UPLOAD.md
//!     readiness.json
//!   apple_books/
//!     manuscript.epub                     ← validated by EPUBCheck if available
//!     metadata.apple.json                 ← ONIX-flavoured fields
//!     cover_brief.md
//!     READY_TO_UPLOAD.md
//!     readiness.json
//! ```

use std::path::{Path, PathBuf};

use booksforge_domain::ExportProfile;
use booksforge_fs::manifest::BundleManifest;
use booksforge_ipc::{
    BooksForgeError, PlatformReadiness, PrepareForPublishingInput, PrepareForPublishingResult,
    PublishingMetadata, ReadinessItem,
};
use serde::Serialize;
use tauri::State;

use crate::commands::export::{
    export_epub_inline, export_via_pandoc, export_via_typst, run_epubcheck_if_available,
};
use crate::state::AppState;

// ── Tauri command ────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn prepare_for_publishing(
    input: PrepareForPublishingInput,
    state: State<'_, AppState>,
) -> Result<PrepareForPublishingResult, BooksForgeError> {
    let project = {
        let guard = state.open_project.lock().await;
        guard.as_ref().cloned()
    }
    .ok_or_else(|| BooksForgeError::internal("no project is open".to_owned()))?;
    let manifest = BundleManifest::read_from_bundle(&project.bundle)
        .await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;
    let project_id = manifest.project.id.clone();

    let platforms: Vec<&str> = if input.platforms.is_empty() {
        vec!["kdp", "google_play", "apple_books"]
    } else {
        input.platforms.iter().map(String::as_str).collect()
    };

    let bundle_root = project.bundle.root().to_owned();
    let exports_root = bundle_root.join("exports");
    tokio::fs::create_dir_all(&exports_root)
        .await
        .map_err(|e| BooksForgeError::internal(format!("create exports dir: {e}")))?;

    let title = manifest.meta.title.clone();
    let author = manifest
        .meta
        .authors
        .first()
        .cloned()
        .unwrap_or_else(|| "[PLACEHOLDER]".to_owned());
    let language = input
        .metadata_overrides
        .language
        .clone()
        .unwrap_or(manifest.meta.language.clone());
    let book_kind = manifest
        .project
        .book_kind
        .map(|k| k.as_str().to_owned())
        .unwrap_or_else(|| "unknown".to_owned());

    let t0 = std::time::Instant::now();
    let mut results: Vec<PlatformReadiness> = Vec::new();

    for platform in platforms {
        let dir = exports_root.join(platform);
        tokio::fs::create_dir_all(&dir)
            .await
            .map_err(|e| BooksForgeError::internal(format!("create {platform} dir: {e}")))?;
        let mut items: Vec<ReadinessItem> = Vec::new();

        // ── 1. EPUB (all platforms get one). ─────────────────────────
        let epub_path = dir.join("manuscript.epub");
        let epub_outcome = export_epub_inline(
            &project,
            epub_path.to_string_lossy().as_ref(),
            ExportProfile::GenericEpub,
            booksforge_domain::FormatProfile::default(),
            None,
        )
        .await;
        match epub_outcome {
            Ok(_) => items.push(ReadinessItem {
                id: "epub".into(),
                label: "EPUB-3 ebook".into(),
                status: "PASS".into(),
                detail: epub_path.to_string_lossy().into_owned(),
            }),
            Err(e) => items.push(ReadinessItem {
                id: "epub".into(),
                label: "EPUB-3 ebook".into(),
                status: "FAIL".into(),
                detail: format!("export failed: {e}"),
            }),
        }

        // ── 2. EPUBCheck (Apple gates on it; KDP / GP non-blocking). ─
        if epub_path.exists() {
            let (ok, msg, errors, warnings) =
                run_epubcheck_if_available(epub_path.to_string_lossy().as_ref()).await;
            let summary = msg.unwrap_or_else(|| {
                if ok {
                    "no issues".to_owned()
                } else {
                    "epubcheck reported issues".to_owned()
                }
            });
            let status = if ok && errors == 0 {
                "PASS"
            } else if errors > 0 {
                "FAIL"
            } else {
                "WARN"
            };
            items.push(ReadinessItem {
                id: "epubcheck".into(),
                label: "EPUBCheck validation".into(),
                status: status.into(),
                detail: format!("{summary} ({errors} errors / {warnings} warnings)"),
            });
        }

        // ── 3. PDF (KDP + Google Play; Apple Books skips). ──────────
        if matches!(platform, "kdp" | "google_play") {
            let pdf_path = dir.join("manuscript.pdf");
            // Prefer typst (Apache-2.0 single binary; no LaTeX required).
            // Falls back to pandoc-via-LaTeX when typst is missing.
            let outcome = if booksforge_export_typst::typst_on_path().is_some() {
                export_via_typst(
                    &project,
                    pdf_path.to_string_lossy().as_ref(),
                    ExportProfile::TradePdf6x9,
                )
                .await
            } else {
                export_via_pandoc(
                    &project,
                    pdf_path.to_string_lossy().as_ref(),
                    ExportProfile::TradePdf6x9,
                    booksforge_domain::FormatProfile::default(),
                    None,
                )
                .await
            };
            match outcome {
                Ok(_) => items.push(ReadinessItem {
                    id: "pdf".into(),
                    label: "Print PDF (6×9 trade)".into(),
                    status: "PASS".into(),
                    detail: pdf_path.to_string_lossy().into_owned(),
                }),
                Err(e) => items.push(ReadinessItem {
                    id: "pdf".into(),
                    label: "Print PDF (6×9 trade)".into(),
                    status: if platform == "kdp" { "FAIL" } else { "WARN" }.into(),
                    detail: format!(
                        "PDF export failed (install typst via brew/scoop/cargo, or a TeX engine for pandoc). Error: {e}"
                    ),
                }),
            }
        }

        // ── 4. Metadata file. ───────────────────────────────────────
        let metadata = build_metadata_block(
            &title,
            &author,
            &language,
            &book_kind,
            &input.metadata_overrides,
        );
        let metadata_path = match platform {
            "kdp" => {
                let p = dir.join("metadata.kdp.csv");
                let csv = render_kdp_csv(&metadata);
                tokio::fs::write(&p, csv)
                    .await
                    .map_err(|e| BooksForgeError::internal(e.to_string()))?;
                p
            }
            "google_play" => {
                let p = dir.join("metadata.gp.json");
                tokio::fs::write(&p, serde_json::to_vec_pretty(&metadata).unwrap_or_default())
                    .await
                    .map_err(|e| BooksForgeError::internal(e.to_string()))?;
                p
            }
            _ /* apple_books */ => {
                let p = dir.join("metadata.apple.json");
                tokio::fs::write(&p, serde_json::to_vec_pretty(&metadata).unwrap_or_default())
                    .await
                    .map_err(|e| BooksForgeError::internal(e.to_string()))?;
                p
            }
        };
        items.push(ReadinessItem {
            id: "metadata".into(),
            label: "Metadata package".into(),
            status: if metadata_has_placeholder(&metadata) {
                "WARN"
            } else {
                "PASS"
            }
            .into(),
            detail: format!(
                "{} (placeholders flagged inline)",
                metadata_path.to_string_lossy()
            ),
        });

        // ── 5. Cover brief stub. ────────────────────────────────────
        let brief_path = dir.join("cover_brief.md");
        tokio::fs::write(&brief_path, render_cover_brief(&title, &author, &book_kind))
            .await
            .map_err(|e| BooksForgeError::internal(e.to_string()))?;
        items.push(ReadinessItem {
            id: "cover_brief".into(),
            label: "Cover brief (HUMAN_REQUIRED to commission art)".into(),
            status: "HUMAN_REQUIRED".into(),
            detail: brief_path.to_string_lossy().into_owned(),
        });

        // ── 6. Per-platform HUMAN_REQUIRED items. ───────────────────
        match platform {
            "kdp" => {
                items.push(ReadinessItem {
                    id: "ai_disclosure".into(),
                    label: "AI-content disclosure".into(),
                    status: "HUMAN_REQUIRED".into(),
                    detail:
                        "KDP requires authors to disclose AI-generated content (text, images, translation) on submission. Confirm during the KDP upload flow."
                            .into(),
                });
                items.push(ReadinessItem {
                    id: "rights_review".into(),
                    label: "Rights / copyright review".into(),
                    status: "HUMAN_REQUIRED".into(),
                    detail:
                        "Confirm publishing rights + (optional) ISBN purchase before submission."
                            .into(),
                });
            }
            "google_play" => {
                items.push(ReadinessItem {
                    id: "preview_settings".into(),
                    label: "Preview settings".into(),
                    status: "HUMAN_REQUIRED".into(),
                    detail: "Set the preview percentage Google Play will show readers (default 20%; non-fiction often 10%).".into(),
                });
            }
            "apple_books" => {
                items.push(ReadinessItem {
                    id: "category_age_explicit".into(),
                    label: "Category / age-range / explicit-content fields".into(),
                    status: if metadata.age_range_present {
                        "PASS"
                    } else {
                        "HUMAN_REQUIRED"
                    }
                    .into(),
                    detail:
                        "Apple requires age-range + explicit-content boolean before publication."
                            .into(),
                });
            }
            _ => {}
        }

        // ── 7. README. ──────────────────────────────────────────────
        let readme_path = dir.join("READY_TO_UPLOAD.md");
        tokio::fs::write(&readme_path, render_readme(platform, &title, &items))
            .await
            .map_err(|e| BooksForgeError::internal(e.to_string()))?;

        // ── 8. readiness.json snapshot. ─────────────────────────────
        let readiness_path = dir.join("readiness.json");
        tokio::fs::write(
            &readiness_path,
            serde_json::to_vec_pretty(&items).unwrap_or_default(),
        )
        .await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;

        let uploadable = !items.iter().any(|i| i.status == "FAIL");
        results.push(PlatformReadiness {
            platform: platform.to_owned(),
            output_dir: dir.to_string_lossy().into_owned(),
            items,
            uploadable,
        });
    }

    Ok(PrepareForPublishingResult {
        project_id,
        platforms: results,
        elapsed_s: ((t0.elapsed().as_secs_f32()) * 10.0).round() / 10.0,
    })
}

// ── Helpers ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
struct MetadataBlock {
    title: String,
    subtitle: String,
    author: String,
    description: String,
    short_description: String,
    keywords: Vec<String>,
    bisac_codes: Vec<String>,
    age_range: String,
    age_range_present: bool,
    language: String,
    isbn: String,
    price_usd: String,
    publication_date: String,
    publisher: String,
    rights_statement: String,
    book_kind: String,
}

fn build_metadata_block(
    title: &str,
    author: &str,
    language: &str,
    book_kind: &str,
    overrides: &PublishingMetadata,
) -> MetadataBlock {
    let placeholder = "[PLACEHOLDER]".to_owned();
    let subtitle = overrides.subtitle.clone().unwrap_or_default();
    let age_range = overrides
        .age_range
        .clone()
        .unwrap_or_else(|| placeholder.clone());
    let age_range_present = overrides.age_range.is_some();
    MetadataBlock {
        title: title.to_owned(),
        subtitle,
        author: author.to_owned(),
        description: overrides.description.clone().unwrap_or(placeholder.clone()),
        short_description: overrides
            .short_description
            .clone()
            .unwrap_or(placeholder.clone()),
        keywords: overrides.keywords.clone().unwrap_or_default(),
        bisac_codes: overrides.bisac_codes.clone().unwrap_or_default(),
        age_range,
        age_range_present,
        language: language.to_owned(),
        isbn: overrides.isbn.clone().unwrap_or(placeholder.clone()),
        price_usd: overrides.price_usd.clone().unwrap_or(placeholder.clone()),
        publication_date: overrides
            .publication_date
            .clone()
            .unwrap_or(placeholder.clone()),
        publisher: overrides.publisher.clone().unwrap_or(placeholder.clone()),
        rights_statement: overrides.rights_statement.clone().unwrap_or(placeholder),
        book_kind: book_kind.to_owned(),
    }
}

fn metadata_has_placeholder(m: &MetadataBlock) -> bool {
    let mut fields = vec![
        m.description.as_str(),
        m.short_description.as_str(),
        m.isbn.as_str(),
        m.price_usd.as_str(),
        m.publication_date.as_str(),
        m.publisher.as_str(),
        m.rights_statement.as_str(),
    ];
    fields.push(m.age_range.as_str());
    fields.iter().any(|f| f.contains("PLACEHOLDER"))
}

fn render_kdp_csv(m: &MetadataBlock) -> String {
    let mut out = String::from("field,value\n");
    let row = |k: &str, v: &str| format!("{k},{}\n", csv_escape(v));
    out.push_str(&row("title", &m.title));
    out.push_str(&row("subtitle", &m.subtitle));
    out.push_str(&row("author", &m.author));
    out.push_str(&row("description", &m.description));
    out.push_str(&row("short_description", &m.short_description));
    out.push_str(&row("keywords", &m.keywords.join("; ")));
    out.push_str(&row("bisac_codes", &m.bisac_codes.join("; ")));
    out.push_str(&row("age_range", &m.age_range));
    out.push_str(&row("language", &m.language));
    out.push_str(&row("isbn", &m.isbn));
    out.push_str(&row("price_usd", &m.price_usd));
    out.push_str(&row("publication_date", &m.publication_date));
    out.push_str(&row("publisher", &m.publisher));
    out.push_str(&row("rights_statement", &m.rights_statement));
    out.push_str(&row("book_kind", &m.book_kind));
    out
}

fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_owned()
    }
}

fn render_cover_brief(title: &str, author: &str, book_kind: &str) -> String {
    format!(
        r#"# Cover Brief — {title}

**Author:** {author}
**Book kind:** {book_kind}

## Concept summary

[PLACEHOLDER — describe the book's emotional promise in 2–3 sentences.]

## Visual direction

- **Mood / palette:** [PLACEHOLDER]
- **Focal subject:** [PLACEHOLDER — silhouette / object / typography-only]
- **Typography:** [PLACEHOLDER]

## Trim + format

- **Print trim:** 6×9 (KDP trade paperback) — change in Settings if your run uses 5×8 or 5.5×8.5.
- **Interior paper:** cream uncoated for fiction, white uncoated for non-fiction.
- **Cover finish:** matte (genre fiction tends to glossy; literary tends to matte).
- **Bleed:** 0.125 in for cover; none for interior.

## Spine + back-cover copy

- **Spine eligibility:** depends on final pagecount (≥100 pages → eligible).
- **Spine text:** [author surname] · [TITLE] · [imprint name]
- **Back-cover copy:** lift the description from `metadata.kdp.csv` (or replace with a tighter blurb here).

## Thumbnail readability checklist

- Title legible at 200×300 px.
- Strong silhouette / high-contrast focal element.
- No more than 2–3 visual elements competing for attention.

## Full-wrap dimensions

Calculated once final pagecount + paper choice are locked in. KDP cover-spread calculator: <https://kdp.amazon.com/en_US/cover-templates>.

## HUMAN_REQUIRED next steps

1. Commission cover art from a designer (or generate via your tool of choice — disclose AI generation per platform rules).
2. Replace this brief with the final cover JPG/PDF before upload.
3. Update `metadata.kdp.csv` (and the per-platform metadata files) with the cover image filename.
"#
    )
}

fn render_readme(platform: &str, title: &str, items: &[ReadinessItem]) -> String {
    let plat_name = match platform {
        "kdp" => "Amazon KDP",
        "google_play" => "Google Play Books",
        "apple_books" => "Apple Books",
        other => other,
    };
    let mut out = format!(
        "# {plat_name} — Ready to Upload\n\n**Book:** {title}\n\n## Files in this directory\n\n",
    );
    for item in items {
        let icon = match item.status.as_str() {
            "PASS" => "✓",
            "WARN" => "⚠",
            "FAIL" => "✗",
            "HUMAN_REQUIRED" => "👤",
            _ => "?",
        };
        out.push_str(&format!("- {icon} **{}** — {}\n", item.label, item.detail));
    }
    out.push_str("\n## Submission steps\n\n");
    match platform {
        "kdp" => out.push_str(
            "1. Sign in at <https://kdp.amazon.com> and start a new title.\n\
             2. Paste fields from `metadata.kdp.csv` into the KDP form.\n\
             3. Upload `manuscript.epub` for the Kindle eBook.\n\
             4. Upload `manuscript.pdf` for the Paperback interior.\n\
             5. Upload your final cover (replace `cover_brief.md`).\n\
             6. **Confirm the AI-content disclosure** if any text or imagery was AI-generated.\n\
             7. Set price + KDP Select / Expanded Distribution preferences.\n\
             8. Submit for review (24–72 h).\n",
        ),
        "google_play" => out.push_str(
            "1. Sign in at <https://play.google.com/books/publish/>.\n\
             2. Create a new book; paste fields from `metadata.gp.json`.\n\
             3. Upload `manuscript.epub` (preferred) or `manuscript.pdf`.\n\
             4. Upload your final cover.\n\
             5. Set the **preview percentage** readers will see (10–20%).\n\
             6. Confirm pricing + countries; submit.\n",
        ),
        "apple_books" => out.push_str(
            "1. Sign in at <https://authors.apple.com>.\n\
             2. Create a new title; paste fields from `metadata.apple.json`.\n\
             3. Upload `manuscript.epub` — Apple validates against EPUBCheck on their side, so the report in this directory should be PASS.\n\
             4. Upload your final cover.\n\
             5. Confirm category + age-range + explicit-content fields.\n\
             6. Submit for review (1–10 days).\n",
        ),
        _ => {}
    }
    out
}

#[allow(dead_code)]
fn _exports_root_for(bundle: &Path) -> PathBuf {
    bundle.join("exports")
}
