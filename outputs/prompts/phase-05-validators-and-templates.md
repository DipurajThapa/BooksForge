# Phase 05 — Validators and templates (MVP-completing)

> **Status note (2026-05-06):** This phase prompt is **superseded** by Milestone 4 (M4) in `IMPLEMENTATION_PLAN.md`. The procedures below are preserved for historical context; use the implementation pack for the MVP build.

## Goal

Land the validator engine and the first three production templates. After this phase a user can create a Romance project, write 50k+ words, and produce a KDP EPUB-3 that passes the validators on first export — the MVP exit gate.

## Pre-conditions

Phases 01–04 merged. Pandoc/epubcheck integration solid.

## Inputs

1. `../_deep/02-FSD-functional-specifications.md` — section 5 (FR-VAL) and section 3 (FR-TPL).
2. `../_deep/05-workflow-and-dataflow.md` — section 6 (validation flow).
3. `../_deep/04-data-model-and-project-format.md` — `validator_runs`, `validator_issues`.
4. `../_deep/09-export-pipeline.md` — section 9 (export-time validators).

## Deliverables

### 1. `booksforge-validator`

Validator API:

```rust
pub trait Validator {
    fn id(&self) -> &str;                  // "manuscript.heading-hierarchy.v1"
    fn category(&self) -> Category;
    fn applies_to(&self) -> AppliesTo;     // mode + template + target
    fn run(&self, project: &ProjectView, ctx: &Ctx) -> Vec<Issue>;
}
```

Pure-function contract enforced by determinism tests. `ProjectView` is a read-only handle. `Issue` has severity, category, rule_id, node ref, range, message, and an optional `fix_kind` (none / deterministic / suggested) plus `fix_payload` for deterministic fixes.

### 2. Built-in validators (≥ 20)

Manuscript: heading hierarchy gaps, orphaned paragraphs, broken cross-refs, missing alt text, oversized images (per profile), unmatched quotes, double spaces, em-dash inconsistency, mixed apostrophe types, stray markdown remnants, empty scenes, missing front-matter (title page, copyright), missing ISBN when assigned, language tag missing, footnote without anchor, anchor without footnote, very long sentence (configurable), passive-voice density (info, not error), heading containing markdown.

KDP-eBook EPUB-3 profile validators: file-size limit, ToC depth, cover dimensions, embedded font license check (against bundled font licence list), forbidden CSS properties.

EPUB-3 schema: epubcheck wrapper as a validator (in addition to its post-export role).

### 3. Templates v1 (three production templates)

`Generic Novel`, `Romance Mass Market`, `Sci-Fi/Fantasy Trade`. Each ships:

- `template.toml` (manifest)
- `scaffold.json` (project skeleton)
- `styles.toml` (typography, page setup, element styles)
- `prompt-overrides.toml` (preset overlays, e.g., a Romance-specific Sharpen rewrite that respects voice conventions)
- `validators.toml` (which validators apply, severity overrides)

Style rules compile per export target — same template, different rules for Manuscript-DOCX vs KDP-eBook EPUB-3. Compilation is in `booksforge-template`.

### 4. Pre-export validator gate

The export orchestrator (Phase 04) consumes a profile's `validators-required` list. Phase 05 wires the actual rules. User setting per project: "Block on errors" / "Warn on errors" / "Off".

### 5. Validator UI panel

Right-side panel: tabs for Errors / Warnings / Info. Each issue: message, click-to-source (jumps the editor to the node range), and a "Fix" button when `fix_kind` is deterministic. Bulk-fix-all for deterministic fixes.

### 6. Tests

- Determinism: each validator runs twice on the same input — output equal.
- Idempotence: applying a deterministic fix and re-running the validator yields zero issues for that rule.
- Per-validator positive and negative fixtures.
- Performance: full validation of a 100k-word project in ≤ 10 s on reference Mac.
- E2E: a fresh Romance project; preflight surfaces issues; deterministic-fix-all clears them; export succeeds.

### 7. Documentation

- `docs/validators/authoring.md` (relevant for Phase 07 plugin authors but useful here).
- In-app help: "Pre-export checks", "Fixing common issues".

## Guard-rails specific to this phase

**[GUARD-P5-1]** Validators are pure of inputs. No I/O, no clock, no random.

**[GUARD-P5-2]** Validator results are cached by `scope_hash`; cached results invalidate when any input changes.

**[GUARD-P5-3]** A "fix" never silently changes content; it produces a typed `Mutation` the orchestrator applies after taking a `pre_validator_fix` snapshot.

**[GUARD-P5-4]** Templates compile to per-target style rules; CI tests render a sample page per template per profile and snapshot-compares.

## Acceptance criteria

1. Each of the 20+ built-in validators has positive and negative fixtures and passes determinism.
2. The three templates produce valid exports (EPUB-3 + DOCX + Generic-PDF).
3. A 100k-word fixture validates in ≤ 10 s.
4. A user can create a Romance project, write content, hit "Export → KDP-eBook", let preflight run, fix issues, and produce a valid `.epub`.
5. **MVP gate:** end-to-end UAT walkthrough by a tester not on the team produces a valid book.

## Review gate

- Validator IDs are stable and namespaced (`manuscript.*`, `kdp.*`, `epub3.*`, `template.*`).
- Templates ship style rules for every supported target.
- Validator UI shows clear messages with how-to-fix guidance.
- MVP exit checklist (FSD §16) satisfied.

## Out of scope

- Genre/series consistency validators (Phase 06 — needs entity bible).
- Plugin-provided validators (Phase 07).
- Custom user validators (V1.5).
- IngramSpark, Apple Books, Kobo profiles (Phase 06+).

## When you finish

PR title `Phase 05: Validators and templates (MVP-complete)`. After merge, cut a public **MVP beta** release tag.
