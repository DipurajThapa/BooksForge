# Phase 04 — Export pipeline

> **Status note (2026-05-06):** This phase prompt is **superseded** by Milestone 5 (M5) in `IMPLEMENTATION_PLAN.md` and `EXPORT_EPUB_SPEC.md` (canonical-HTML pipeline). The procedures below are preserved for historical context; use the implementation pack for the MVP build.

## Goal

Ship the canonical export pipeline: BooksForge-AST builder, Pandoc sidecar integration, asset processor, font subsetter, epubcheck post-processor. Profiles delivered: Manuscript-DOCX, Generic-PDF (Typst engine), Generic-EPUB-3, KDP-eBook EPUB-3. Reproducibility test fixture in CI. The exit gate is a 100k-word fixture exporting to all three formats with KDP profile passing epubcheck.

## Pre-conditions

Phase 01–02 merged. CI green. May run in parallel with Phase 03.

## Inputs

1. `../_deep/09-export-pipeline.md` — entire document.
2. `../_deep/02-FSD-functional-specifications.md` — section 9.
3. `../_deep/05-workflow-and-dataflow.md` — section 7 (export flow).
4. `../_deep/03-TAD-technical-architecture.md` — section 11.
5. `../_deep/06-security-privacy-compliance.md` — section 4.5 (sidecar isolation).

## Deliverables

### 1. `booksforge-export` (Layer 3)

`Ast` — canonical document representation: front-matter, mainmatter (parts → chapters → blocks), back-matter, metadata. `AstBuilder` resolves cross-refs, footnote numbering (per profile: footnote-per-page or endnotes), language tags, citations (deferred resolution if Phase 06 not done; placeholder rendering ok). Tracked-changes accept policy applied here (per profile: accept all / preserve / reject all).

Pure functions; no I/O.

### 2. `booksforge-export-pandoc` (Layer 4)

Pandoc sidecar driver. Spawns `pandoc --from=json --to=<target> ...` with a temp resource directory. Stdin = Pandoc-AST JSON; stdout = output stream (or file); stderr captured for error reporting.

Pandoc binary bundled in `apps/desktop/sidecars/pandoc/<arch>/<os>/`. CI verifies binaries present per OS at build time.

### 3. Profile resolution

`Profile` declarative format in `apps/desktop/profiles/*.toml` for the four launch profiles. Profile composes: template, target format, post-processors list, validators-required list, asset rules (DPI, max dimensions, format conversions).

`ProfileResolver` loads, validates, and produces a `ResolvedProfile` for the orchestrator.

### 4. Asset pipeline

Asset handler in the export crate: copies referenced assets to the temp work dir with stable filenames, applies per-profile rules (DPI, format, dimensions), rewrites references in the AST. Image library: `image` crate; HEIC support via `libheif-rs` (optional feature).

### 5. Font subsetter

`booksforge-export/src/fonts.rs` using `subset-font` (Rust) or `pyftsubset` (Python sidecar) — pick the Rust path. Subsets fonts to embedded glyphs only for EPUB and embedded fonts in PDF. Track fonts referenced by template; load from `apps/desktop/resources/fonts/`.

### 6. epubcheck integration

Bundle a jlink-stripped JRE under `apps/desktop/sidecars/jre/<os>/`. Bundle epubcheck JAR. Run `java -jar epubcheck.jar <output.epub>` post-export. Parse JSON output; surface errors and warnings; fail export on errors per profile setting; warnings logged.

### 7. Export orchestrator (Layer 2)

Tauri commands:

- `export.list_profiles({ project_id }) -> Profile[]`
- `export.preflight({ project_id, profile_id }) -> PreflightReport` (runs validators)
- `export.run({ project_id, profile_id, options }) -> JobId` (streams `export.progress` events)
- `export.cancel({ job_id }) -> ()`
- `export.list_history({ project_id, limit }) -> ExportRecord[]`

The orchestrator: takes `pre_export` snapshot; resolves profile; runs preflight; if blocking errors and user setting blocks → abort with report; builds AST; transforms to Pandoc-AST; spawns Pandoc; runs post-processors in order; runs final validation; writes file; records `exports` row.

### 8. Export wizard UI

Step 1 pick profile. Step 2 preflight summary (errors, warnings, info — counts and click-to-source). Step 3 confirm. Step 4 progress with per-step status. Step 5 done with file link.

History panel listing past exports with file path, profile, app version, validators run, hash, timestamp.

### 9. Reproducibility test

In `crates/booksforge-export/tests/reproducibility.rs`: load a fixture project (`medium.booksforge`), run export to EPUB-3 KDP twice, compute file hashes, assert equal. Pin Pandoc version. Use `--metadata-file` with a fixed timestamp. Track baseline hashes per profile in a checked-in file; deviation requires an explicit baseline-update commit.

### 10. Tests

- Unit: AstBuilder cross-ref resolution, footnote numbering, tracked-change accept policies.
- Integration: export of `medium.booksforge` to all four profiles.
- Reproducibility test (above) per profile.
- Performance: 100k-word EPUB-3 export ≤ 30 s on reference Mac.
- E2E: export wizard end-to-end on a real project.
- Sidecar integrity: Pandoc binary hash verified at startup; mismatch → typed error on first export (not on launch).

### 11. Documentation

- `docs/exports/architecture.md` — pipeline shape, post-processors.
- `docs/exports/profiles.md` — how a profile composes; how to author one (Phase 1.5 polishes this).
- In-app help: "Exporting to EPUB", "Exporting to KDP", "Reproducible exports".

## Guard-rails specific to this phase

**[GUARD-P4-1]** Pandoc must be invoked as a sidecar process. Do not link any GPL crate. `cargo deny` enforces.

**[GUARD-P4-2]** Asset rewriting must be reference-stable: same input always produces same output filenames in the temp work dir.

**[GUARD-P4-3]** Reproducibility test is part of CI. A baseline-changing PR must include a commit message explaining why the bytes changed.

**[GUARD-P4-4]** Pre-export snapshot is taken before the orchestrator does anything destructive (none in Phase 04 — but the snapshot is the audit trail).

**[GUARD-P4-5]** No Pandoc CLI flags are user-controllable except through profile authorship — no shell-injection vector.

**[GUARD-P4-6]** Export progress events must be emitted at least every 2 s; UI shouldn't show frozen progress.

## Acceptance criteria

1. `medium.booksforge` exports to all four profiles in CI on all three OSes.
2. KDP-eBook EPUB-3 export passes epubcheck with zero errors and ≤ 5 warnings.
3. Reproducibility test green for all four profiles.
4. 100k-word `novel-100k.booksforge` exports to EPUB-3 in ≤ 30 s on reference Mac.
5. Export history records full traceability per export.
6. Export sidebar UI shows progress and produces a clickable file link on completion.

## Review gate

- Pandoc binaries shipped per OS; license file at `licenses/pandoc/`.
- JRE jlink-stripped (target ≤ 50 MB); license file shipped.
- Profile authors can read the profile spec without reading Rust code.
- Reproducibility test exists and is non-skippable in CI.

## Out of scope

- Tracked-changes round-trip (Phase 06).
- Citation engine to CSL output (Phase 06; placeholder rendering ok in Phase 04).
- KDP-Print PDF, IngramSpark, Apple Books profiles (Phase 05 adds them).
- Custom user profile editor (Phase 1.5).
- LaTeX export (Phase 06).
- PDF/X compliance (Phase 05+).

## When you finish

PR title `Phase 04: Export pipeline`. Update `STATUS.md`. Phase 05 may begin once Phase 04 merges.
