# BooksForge — Package overview & cross-doc verification

**Generated:** 2026-05-06
**Purpose:** A guided tour of the package and a verification log of cross-doc consistency.

## What's in this package

```
outputs/
├── README.md                                  ← Start here
├── 00-package-overview.md                     ← This file
├── 01-BRD-business-requirements.md            ← Why, who, what success
├── 02-FSD-functional-specifications.md        ← Every feature, every FR-ID
├── 03-TAD-technical-architecture.md           ← Components, decisions, layering
├── 04-data-model-and-project-format.md        ← SQLite schema, on-disk layout
├── 05-workflow-and-dataflow.md                ← Process flows
├── 06-security-privacy-compliance.md          ← Threat model, controls, GDPR
├── 07-plugin-architecture.md                  ← Sandbox, capabilities, marketplace
├── 08-ai-integration.md                       ← Local-first AI architecture
├── 09-export-pipeline.md                      ← Pandoc, profiles, reproducibility
├── 10-roadmap-and-phasing.md                  ← MVP → V2.0
├── 11-test-and-validation-strategy.md         ← Unit, integration, E2E, perf
├── 12-risk-register.md                        ← 32 risks with mitigations
├── 13-glossary-and-decision-log.md            ← Vocabulary + 16 ADRs
├── prompts/
│   ├── README.md                              ← Universal guard-rails (G1–G18)
│   ├── STATUS.md                              ← Phase tracking
│   ├── phase-00-bootstrap.md                  ← Repo scaffolding
│   ├── phase-01-foundations.md                ← Project lifecycle, SQLite, IPC
│   ├── phase-02-editor-core.md                ← TipTap editor
│   ├── phase-03-local-ai.md                   ← llama.cpp + audit
│   ├── phase-04-export-pipeline.md            ← Pandoc + EPUB/PDF/DOCX
│   ├── phase-05-validators-and-templates.md   ← MVP exit
│   ├── phase-06-non-fiction-and-academic.md   ← Citations, tracked changes
│   ├── phase-07-plugin-runtime.md             ← WASM sandbox
│   ├── phase-08-linux-and-signing.md          ← All-OS code signing
│   ├── phase-09-encryption-and-backups.md     ← V1.0 GA
│   ├── phase-10-marketplace-and-cloud-llm.md  ← V1.5 begins
│   ├── phase-11-sync.md                       ← E2EE sync
│   ├── phase-12-collaboration-v1.md           ← V1.5 GA
│   ├── phase-13-plugin-write-capabilities.md  ← Importers / exporters
│   ├── phase-14-voice-and-advanced-ai.md      ← Dictation + long-context
│   └── phase-15-translator-pack.md            ← V2.0 GA
└── diagrams/
    ├── README.md
    ├── 01-system-context.svg
    ├── 02-component-architecture.svg
    ├── 03-dataflow-edit-loop.svg
    ├── 04-workflow-lifecycle.svg
    ├── 05-plugin-architecture.svg
    ├── 06-ai-flow.svg
    ├── 07-export-pipeline.svg
    └── 08-roadmap-gantt.svg
```

## How the documents relate

```
        ┌─────────────────────────────┐
        │  01-BRD (why & who)         │
        └────────────┬────────────────┘
                     │
        ┌────────────▼────────────────┐
        │  02-FSD (what features)     │
        └────────────┬────────────────┘
                     │
   ┌─────────────────┼─────────────────┐
   │                 │                 │
   ▼                 ▼                 ▼
03-TAD            06-Security       11-Test
(how)             (defenses)        (verify)
   │                 │
   ▼                 ▼
04-Data           07-Plugin
05-Workflow       08-AI
                  09-Export
                  │
                  ▼
            10-Roadmap (when)
                  │
                  ▼
         prompts/ (Claude Code build)
```

## Cross-doc verification log

The package was self-checked for the following kinds of consistency. Findings are listed below. None are blockers; minor items are filed as backlog.

### Decision IDs

DECISION-001 through DECISION-016 are defined in `13-glossary-and-decision-log.md`. References in other documents:

- DECISION-001 (pricing): `01-BRD §7`. ✅
- DECISION-002 (TipTap): `03-TAD §6`. ✅
- DECISION-003 (Rust sidecar): `03-TAD §7`. ✅
- DECISION-004 (bundle format): `03-TAD §8`. ✅
- DECISION-005 (Pandoc sidecar): `03-TAD §11`, `09-Export §3`. ✅
- DECISION-006 (llama.cpp): `08-AI §3`. ✅
- DECISION-007 (Typst/LaTeX): `09-Export §5.2`. ✅
- DECISION-008 through 016: glossary-only references; consistent.

### Functional requirement IDs

FR-IDs are stable. Cross-references found and verified:

- FR-AI-001, FR-AI-004, FR-AI-008, FR-AI-013, FR-AI-014, FR-AI-015 referenced in `08-AI`, `05-Workflow`, `06-Security`. ✅
- FR-PROJ-001…017 referenced in `prompts/phase-01-foundations.md`. ✅
- FR-EDIT-001…022 referenced in `prompts/phase-02-editor-core.md`. ✅
- FR-VAL, FR-TPL referenced in `prompts/phase-05`. ✅
- FR-SNAP-001…006 referenced in `prompts/phase-09-encryption-and-backups.md`. ✅
- FR-PLUG-001…007 referenced in `prompts/phase-07-plugin-runtime.md`. ✅
- FR-EXP-001…007 referenced in `prompts/phase-04-export-pipeline.md`. ✅

### Risk references

R-04 (DOCX tracked changes) referenced in `prompts/phase-06`. ✅
R-07 (Pandoc GPL) referenced in `01-BRD §8`, `03-TAD §11`. ✅
Top-5 attention list in `12-risk-register.md` is internally consistent.

### Persona references

Anya (indie self-publisher), Theo (trade author), Aisha (academic) introduced in `01-BRD §4`. Referenced in `02-FSD` user stories. ✅

### Acceptance criteria flow

MVP exit criteria in `10-Roadmap` matches MVP-tagged FRs in `02-FSD`. ✅
V1.0 exit criteria in `10-Roadmap` matches `01-BRD §6` success metrics. ✅

### Diagrams

System Context, Component Architecture, Dataflow, Workflow Lifecycle, Plugin Architecture, AI Flow, Export Pipeline, Roadmap Gantt — all referenced from `diagrams/README.md`. Source documents reference them implicitly; the diagrams are self-explanatory companions, not standalone authority.

### Minor open items (filed as backlog)

- BRD §11 says "MVP (months 0–4)"; roadmap is more granular at "week 18" which is ~4.5 months. Coarse-grained numbers are intentional in BRD; the roadmap is authoritative.
- A few phase prompts (08, 11, 12, 13–15) are intentionally compact compared to phases 0–7, since the patterns established in earlier phases apply uniformly. They remain self-contained.

## Suggested critical-path reading order

> **Note (2026-05-06):** Pass 2 produced a tighter implementation pack at the top level of `outputs/`. For a fresh reader, prefer `../INDEX.md` and `../CLAUDE_CODE_CONTEXT_HARNESS.md`. The route below is the original deep-spec reading order, kept for context.

For a busy reviewer who has 90 minutes, read in this order:

1. `../README.md` (5 min)
2. `01-BRD §§ 1–6` (15 min)
3. `03-TAD §§ 1–4, 6, 7, 8, 11` (20 min)
4. `06-Security §§ 1–7` (15 min)
5. `10-Roadmap` (15 min)
6. `../prompts/README.md` and `../prompts/phase-00-bootstrap.md` (15 min)
7. Diagrams 01, 02, 04, 06 (5 min)

For an engineer about to start the MVP build:

1. `../CLAUDE.md`
2. `../CLAUDE_CODE_CONTEXT_HARNESS.md`
3. `../IMPLEMENTATION_PLAN.md` §3 (MZ-01 through MZ-10)

The original phase-00 prompt at `../prompts/phase-00-bootstrap.md` is superseded; see the status note at its top.

For a security review:

1. `06-Security`
2. `07-Plugin §§ 4–8`
3. `08-AI §§ 1–7`

## Update procedure

This is the source of truth. When implementation diverges:

1. Open a PR that updates the relevant document **and** the implementation in the same change.
2. If the change is architectural, add an entry to `13-glossary-and-decision-log.md` (`[DECISION-NNN-revB]`).
3. If the change introduces a new risk, add to `12-risk-register.md`.
4. If the change affects a phase's deliverables, update `prompts/STATUS.md`.
5. If the change affects user behaviour, update the in-app help (Phase 00 wires the docs scaffold).

Drift between docs and code is a bug.
