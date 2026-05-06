# Test & Validation Strategy — BooksForge

**Version:** 1.0.0-draft  •  **Status:** For review  •  **Date:** 2026-05-06

---

## 1. Goals

The test strategy exists to (a) prevent regressions in durability, performance, and security; (b) make refactoring safe; (c) give Claude Code a tight feedback loop so generated code does not drift from the spec. Tests are part of the definition of done — a feature without tests is unfinished.

## 2. Test pyramid

```
       ┌─────────────────────┐
       │ Manual exploratory   │  rare — for new UX
       └─────────────────────┘
       ┌─────────────────────┐
       │ E2E (Playwright)    │  ~ dozens; happy paths + critical regressions
       └─────────────────────┘
     ┌────────────────────────┐
     │ Integration (real db, │  ~ hundreds; cross-layer
     │  real fs, mock LLM)   │
     └────────────────────────┘
   ┌──────────────────────────────┐
   │ Unit + property tests        │  ~ thousands; pure-domain logic
   └──────────────────────────────┘
```

Coverage targets: domain crates ≥ 90 % line coverage; adapters (storage, fs, ai-runtime) ≥ 80 %; UI components ≥ 60 %. Coverage isn't the goal but a floor; the goal is *meaningful* tests.

## 3. Frameworks

- **Rust unit/integration:** built-in `#[test]`, `proptest` for property tests, `insta` for snapshot tests, `criterion` for benchmarks, `mockall` for mocks.
- **TS unit:** Vitest.
- **E2E:** Playwright against the built Tauri app (using `tauri-driver` or WebDriver bridge).
- **Accessibility:** `axe-core` integrated in Playwright tests; manual audits per phase.
- **Visual regression:** Playwright screenshots on a curated set of UI states; tolerance tuned per OS to absorb font rendering differences.
- **Security:** `cargo-audit`, `cargo-deny`, `npm audit`, `osv-scanner` in CI; periodic SAST via Semgrep.
- **Fuzz:** `cargo-fuzz` against parsers (DOCX importer, manifest parser, plugin loader).

## 4. Reference hardware for benchmarks

Benchmarks are meaningless without a fixed bar. Reference machines:

- **Mac-Apple-Silicon-16GB:** M2 MacBook Air, 16 GB, macOS 14.
- **Mac-Apple-Silicon-32GB:** M2 Pro MacBook Pro, 32 GB, macOS 14.
- **Windows-Mid:** Dell XPS 13, i5-1240P, 16 GB, Windows 11.
- **Linux-Mid:** ThinkPad X1 Carbon, i7-1260P, 16 GB, Ubuntu 22.04.
- **Mac-Intel:** 2019 i7 MacBook Pro, 16 GB, macOS 12.

CI runs benchmarks on a subset (mac-arm and windows-mid). Full reference suite runs nightly.

## 5. Fixtures

A library of reference projects lives in `crates/booksforge-test-fixtures/`:

- `tiny.booksforge` — 100 words, smoke tests.
- `medium.booksforge` — 30k words, 5 chapters, 3 entities, 10 footnotes.
- `novel-100k.booksforge` — 100k words, 30 chapters, 50 entities, mass-market romance template.
- `novel-200k.booksforge` — 200k words, performance fixtures.
- `monograph-academic.booksforge` — 80k words, 600 footnotes, 200 citations, Chicago author-date.
- `tracked-changes-publisher.docx` — real publisher template with tracked changes for round-trip tests.
- `corruption-1.booksforge` — partially corrupted bundle for recovery tests.
- `corrupted-pm-json.booksforge` — invalid ProseMirror state for resilience tests.
- `unicode-rtl.booksforge` — Arabic/Hebrew content for bidi tests.
- `large-image-bombs.booksforge` — 50 large images for asset pipeline.

Fixtures are content-addressed and reproducible from a `regenerate.sh` script.

## 6. Specific test areas

### 6.1 Storage and durability

Property tests on the SQLite layer covering: insert/update/delete reach disk; WAL replay after kill -9 produces consistent state; foreign keys hold; schema migrations are reversible (where reversible) and snapshot-protected (where not). Crash-fuzz: a long-running test randomly kills the process and asserts that on next launch the project opens with at most one autosave-interval of loss.

### 6.2 Editor

Property tests on ProseMirror serialiser: random documents serialise → deserialise to identical state. Performance test: keystroke latency on a 50k-word chapter ≤ 30 ms p95. Snapshot tests: typical operations produce expected document trees.

### 6.3 Validators

Each built-in validator has positive and negative fixtures. Idempotence test: validator run twice produces identical results. Determinism test: same inputs produce same outputs in CI on every OS.

### 6.4 Templates and formatting engine

Each built-in template renders correctly to all its supported export profiles, asserted by snapshot of the canonical AST and visual snapshot of a sample page.

### 6.5 Export pipeline

The big one. Per-profile end-to-end tests:

For each (template, profile) pair we run a curated fixture project through the pipeline and assert:

- Reproducibility: hash matches a baseline; if the baseline must move, an explicit baseline-update commit is required.
- Validator pass: target store's validator (KDP/IngramSpark/Apple/epubcheck) passes.
- Spot-check correctness: ToC has expected entries; cover image is the right asset; copyright page renders; footnote numbering is right; cross-references resolve.
- Performance: under the §10 budgets.

DOCX tracked-changes round-trip tests use real publisher templates (legally acquired or community-contributed dummy versions) and a `pandoc-diff`-style structural comparator.

### 6.6 AI

Mock provider for unit tests of orchestration. Live local provider with TinyLlama for smoke tests of the inference path. VCR-recorded cassettes for cloud providers' adapter tests. Privacy invariant test: with the network mocked-fail, every local AI feature still works. Audit-log test: every AI call writes a row.

### 6.7 Plugins

Sandbox tests against adversarial WASM modules: allocator bombs, infinite loops, attempted host-call from outside capability set, large-result attacks. Capability prompt UX test (Playwright). Sideload-with-warning test. Marketplace signature verification test (with a corrupted signature fixture).

### 6.8 Security

Update signature verification: positive and negative tests (valid signature → applied; tampered → rejected). License activation: offline activation works; expired license enters grace; tampered token rejected. Encryption round-trip: encrypted project re-opens with correct passphrase, fails with wrong passphrase, never partially-decrypts.

### 6.9 Accessibility

Per phase, an axe-core audit of every UI surface in dark and light theme. Keyboard-only operation: a Playwright test traverses the new-project wizard with keyboard only. Screen-reader compatibility: manual audit with NVDA/VoiceOver per release; documented.

### 6.10 i18n

Locale-specific tests for: bidi, plural rules (ICU MessageFormat), date and number formatting, tokeniser for word count in non-space-separated languages (Chinese, Japanese — uses ICU break iterator).

### 6.11 Performance

Benchmark suite runs on every PR for a subset (cold-open, keystroke latency, validator, EPUB export). Full suite nightly. Regressions of >10 % fail CI; the PR must include a benchmark explanation.

## 7. CI matrix

```
PR:                 ubuntu-22, mac-13, windows-2022 — lint, typecheck, unit, smoke build
Merge to main:      add: integration tests, packaged artifact
Nightly:            add: full E2E, full benchmarks, accessibility audit, security scan
Pre-release:        add: all reference-hardware benchmarks, manual smoke of installers, signing verification
```

CI uses GitHub Actions OIDC for cloud secrets (no static secrets in repo). Artifact retention: nightly 14 days, releases indefinite.

## 8. Manual / exploratory testing

Per phase, a curated test plan in `docs/test-plans/phase-NN.md` covers exploratory areas a human eye catches better than a script: visual aesthetics of exports, AI suggestion quality on real prose, onboarding flow for a complete novice. Beta program (V1.0+) provides ongoing user testing with a feedback loop into the issue tracker.

## 9. Test data privacy

Fixtures contain no real personal data. All "author" names in fixtures are fictional; all manuscript content is public-domain or original-for-tests (we maintain a `LICENSE` for fixture text). Crash dump tests use distinctive marker tokens to verify scrubbing.

## 10. Definition of done (testing-relevant subset)

A change is not done until: unit tests for new pure logic; integration test for any cross-layer behaviour; E2E for any user-visible flow change; benchmark assertion for any performance-sensitive change; accessibility audit for any UI change; i18n key extracted for any string; security checklist line for any change touching trust boundaries.
