# BooksForge — Production Specification Package

> **Working name:** BooksForge (placeholder — rename freely)
> **Document set version:** 1.1.0-draft
> **Last revised:** 2026-05-06
> **Owner:** Dipuraj Thapa

A local-first, AI-assisted, cross-platform book authoring and publishing platform with a **bounded local-LLM agent swarm** running on the user's machine via Ollama. This package contains every artifact required to take the product from concept to shipped V2.0, including a Claude Code prompt pack with hard guard-rails so agentic development stays inside the architectural envelope.

## Start here for implementation

Claude Code (and any engineer pairing with it) should read **`CLAUDE_CODE_START_HERE.md`** first. It points at the rest of the implementation pack in the right order.

The implementation pack — the contract for the first build — comprises:

| Document | Purpose |
|----------|---------|
| [`CLAUDE_CODE_START_HERE.md`](./CLAUDE_CODE_START_HERE.md) | Entry file for implementation; reading order, hard rules, Milestone Zero |
| [`PRODUCT_REQUIREMENTS.md`](./PRODUCT_REQUIREMENTS.md) | MVP scope, target users, journeys, acceptance criteria |
| [`ARCHITECTURE.md`](./ARCHITECTURE.md) | System design with Ollama-first AI runtime |
| [`AGENTS.md`](./AGENTS.md) | The agent catalog, prompts, schemas, orchestrator |
| [`DATA_MODEL.md`](./DATA_MODEL.md) | Entities and SQL schemas (including agent layer) |
| [`UI_UX_SPEC.md`](./UI_UX_SPEC.md) | MVP screens and behaviours |
| [`IMPLEMENTATION_PLAN.md`](./IMPLEMENTATION_PLAN.md) | Milestones and the first ten Claude Code tasks |
| [`TESTING_STRATEGY.md`](./TESTING_STRATEGY.md) | How we prove what we built works |
| [`SECURITY_PRIVACY.md`](./SECURITY_PRIVACY.md) | Privacy invariants and threat model for MVP |

## Deep specification index

The deep specs live under `_deep/` and are reference material. Read them on demand for trade-off rationale, FR-IDs, or the long-form ADR log. The implementation pack above is the source of truth for current behaviour.

| # | Document | Purpose | Audience |
|---|----------|---------|----------|
| 01 | [BRD — Business Requirements](./_deep/01-BRD-business-requirements.md) | Why we are building it, who for, what success looks like | Product, exec, investor |
| 02 | [FSD — Functional Specifications](./_deep/02-FSD-functional-specifications.md) | Every feature, user story, and acceptance criterion | Product, engineering, QA |
| 03 | [TAD — Technical Architecture](./_deep/03-TAD-technical-architecture.md) | Components, decisions, trade-offs, alternatives | Engineering, security |
| 04 | [Data Model & Project Format](./_deep/04-data-model-and-project-format.md) | SQLite schema reference | Engineering, plugin authors |
| 05 | [Workflow & Dataflow](./_deep/05-workflow-and-dataflow.md) | End-to-end process flows | Product, engineering |
| 06 | [Security, Privacy & Compliance](./_deep/06-security-privacy-compliance.md) | Threat model, controls, GDPR posture | Security, legal |
| 07 | [Plugin Architecture](./_deep/07-plugin-architecture.md) | Manifest, sandbox, capability model (V1.0+) | Engineering, ecosystem |
| 08 | [AI Integration](./_deep/08-ai-integration.md) | AI principles, prompt format, audit log | Engineering, product |
| 09 | [Export Pipeline](./_deep/09-export-pipeline.md) | DOCX/PDF export details (Pandoc) | Engineering |
| 10 | [Roadmap & Phasing](./_deep/10-roadmap-and-phasing.md) | V1.0 → V1.5 → V2.0 phases | All |
| 11 | [Test & Validation Strategy](./_deep/11-test-and-validation-strategy.md) | Long-term test posture | Engineering, QA |
| 12 | [Risk Register & Mitigations](./_deep/12-risk-register.md) | Identified risks, severity, mitigations | All |
| 13 | [Glossary & Decision Log](./_deep/13-glossary-and-decision-log.md) | Shared vocabulary, full ADR log | All |
| — | [Claude Code Prompt Pack](./prompts/README.md) | V1.0+ phase prompts with guard-rails | Engineering using Claude Code |
| — | [Diagrams folder](./diagrams/) | SVG architecture and flow diagrams | All |

## How to read this package

If you have one hour and are about to implement: read **`CLAUDE_CODE_START_HERE.md`**, **`PRODUCT_REQUIREMENTS.md`**, and **`ARCHITECTURE.md`**.
If you have one hour and want product context: read `_deep/01-BRD`, `_deep/03-TAD §§ 1–4`, and `_deep/10-roadmap-and-phasing`.
If you are an engineer about to build: follow **`CLAUDE_CODE_START_HERE.md`** end-to-end.
If you are a security reviewer: read `SECURITY_PRIVACY.md` plus `_deep/06-security-privacy-compliance.md`.
If you are a product/QA reviewer: read `PRODUCT_REQUIREMENTS.md`, `MVP_SCOPE.md`, and `TESTING_STRATEGY.md`.

## Decision posture

Every contentious choice has been locked, with the rationale captured in `_deep/13-glossary-and-decision-log.md` and `ARCHITECTURE_DECISIONS.md`. Locked defaults:

1. **Editor framework — TipTap** (ProseMirror-based). See TAD §6 and `[DECISION-002]`.
2. **Sidecar runtime — Rust** for application services. Pandoc, epubcheck, and Ollama are external processes. See `[DECISION-003]`.
3. **Pandoc distribution — sidecar binary**, not statically linked. See `[DECISION-005]`.
4. **Local LLM runtime — Ollama-first** over its HTTP API on `127.0.0.1:11434`. Embedded `llama.cpp` is **post-V1.0**. See `[DECISION-006-revB]` and `ARCHITECTURE.md §5`.
5. **Plugin sandbox — WebView-isolated UI plugins + WASM compute plugins** with a capability-token model. **Post-MVP** — not built in the first 16 weeks. See `[DECISION-008]` and `_deep/07-plugin-architecture.md`.
6. **Project file format — folder-based `.booksforge` directory bundle**. See `[DECISION-004]`.
7. **Agent architecture — bounded agent swarm** with hard caps and approval gates. See `[DECISION-016]` and `AGENTS.md`.
8. **ePUB pipeline — canonical-HTML approach** where the editor preview HTML is the export source. See `EXPORT_EPUB_SPEC.md`.

## Status legend

Inside the documents you will see:

- **[MUST]** — required for the phase to be considered complete
- **[SHOULD]** — strongly recommended; deviations need an ADR
- **[MAY]** — optional / deferrable
- **[DECISION]** — choice point; default given but reversible
- **[RISK]** — see risk register
- **[GUARD]** — Claude Code guard-rail (build must respect this)

## Change control

This package is the source of truth. When the implementation diverges, update the relevant doc in the same PR — never let docs drift. The decision log in `_deep/13-glossary-and-decision-log.md` records every architectural change with date, rationale, and impact.
