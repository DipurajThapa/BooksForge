# Architecture diagrams

> **Status (2026-05-06):** Diagrams 02, 06, and 07 were **refreshed in Pass 2** to reflect the Ollama-first pivot, the bounded agent swarm with the nine MVP agents, the memory + vocabulary subsystems, and the canonical-HTML ePUB pipeline. Diagram 08 (roadmap gantt) is still based on the original 18-week plan and remains stale until a milestone closes; the prose in `IMPLEMENTATION_PLAN.md` wins on that one. Where any diagram contradicts the prose, **the prose wins.**

Embed these in any document that benefits from a visual. They render as SVG everywhere modern.

| File | Purpose | Status | Referenced from |
|------|---------|--------|-----------------|
| `01-system-context.svg` | High-level system context, actors, opt-in cloud | Active | README, `_deep/03-TAD-technical-architecture.md` §2 |
| `02-component-architecture.svg` | Layered component map (4 layers, 16 crates) — **refreshed Pass 2** | Active | `ARCHITECTURE.md §3` |
| `03-dataflow-edit-loop.svg` | Hot-path dataflow for write/save/mirror/snapshot | Active | `_deep/05-workflow-and-dataflow.md` §3, §10 |
| `04-workflow-lifecycle.svg` | Project state lifecycle | Active | `_deep/05-workflow-and-dataflow.md` §1 |
| `05-plugin-architecture.svg` | Plugin sandbox + capabilities + types (post-MVP) | Active | `_deep/07-plugin-architecture.md` |
| `06-ai-flow.svg` | Agent swarm + Ollama HTTP flow — **refreshed Pass 2** | Active | `AGENTS.md`, `OLLAMA_LOCAL_LLM_SPEC.md`, `ARCHITECTURE.md §5–6` |
| `07-export-pipeline.svg` | Canonical-HTML export pipeline — **refreshed Pass 2** | Active | `EXPORT_EPUB_SPEC.md`, `EXPORT_EPUB_QA.md` |
| `08-roadmap-gantt.svg` | Phase timeline (legacy 18-week plan) | Stale — refresh at every milestone close | `_deep/10-roadmap-and-phasing.md`, `IMPLEMENTATION_PLAN.md` |

To regenerate or modify: edit the SVGs directly. They use no fonts or images outside the SVG markup.

When you refresh a diagram, also update the row above and add a `CHANGELOG_DOC_REFACTOR.md` entry.
