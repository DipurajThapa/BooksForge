# Phase 15 — Translator pack (V2.0 release)

## Goal

Translator workflow with terminology preservation: glossary-locked translation that preserves entity names and key terms. The flagship feature for international authors.

## Pre-conditions

Phase 14 merged.

## Inputs

1. `../_deep/01-BRD-business-requirements.md` — V2.0 scope.
2. `../_deep/08-ai-integration.md` — translation workflow.

## Deliverables

### 1. Glossary

Per-project glossary: entity names, technical terms, idiomatic phrases. Entries can be locked (must not be translated), translated explicitly, or "translator's choice" with examples.

### 2. Translation workflow

Per-scene translation pass with the glossary embedded in the prompt template. Output staged in a "Translated" branch alongside the original, so author retains source. Quality checks (back-translation diff) flag drift.

### 3. Bilingual export

Export profile that produces a bilingual EPUB (left page source, right page translation) — useful for language-learning markets and for translator review.

### 4. Tests

- Translation preserves locked terms from glossary.
- Bilingual export passes epubcheck.

## Guard-rails

**[GUARD-P15-1]** Original text is never overwritten by translation; translation lives in its own branch within the project.

**[GUARD-P15-2]** Glossary-locked terms verified preserved in tests.

## When you finish

PR title `Phase 15: Translator pack`. After merge cut **V2.0 release**.

---

## End of phase pack

Once Phase 15 ships, the prompt pack is complete for the V2.0 horizon described in the roadmap. Future phases require updates to the BRD and a new prompt-pack edition.
