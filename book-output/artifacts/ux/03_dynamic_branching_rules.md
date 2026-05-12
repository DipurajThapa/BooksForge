# 03 — Dynamic Branching Rules

**Score:** 6.0/10 · **Status:** WARN · **Weight:** 1.5

3/5 dynamic branches detected. The UI does NOT adapt to fiction vs non-fiction, beginner vs advanced, or KDP-print vs ebook-only. Every user sees the same field set.

## Detected branches

- fiction_vs_nonfiction: ✓
- kdp_vs_ebook_only: ✓
- beginner_vs_advanced: ✗
- childrens_book_layout: ✗
- chained_intake_outline: ✓
`booksforge/apps/desktop/src-ui/src/components/agents/IntakeAndOutlinePanel.tsx:25` — chained intake → outline panel exists

## Required branches (per the brief) and current state

| Branch | Required behaviour | Current state |
|---|---|---|
| `book_type = childrens` | Adjust word count, layout, illustration prompts, reading level, marketplace metadata | **NOT IMPLEMENTED** |
| `book_type = fiction` | Enable character bible, world bible, acts, scenes, dialogue polish, continuity | **PARTIALLY** — continuity agent exists; character/world bibles missing (BACKLOG §A13) |
| `book_type = non-fiction` | Enable argument structure, research dossier, chapter thesis, examples, citations | **PARTIALLY** — non-fiction template + `chapter-drafter-nf` exist; research-dossier agent missing |
| `target = KDP paperback` | Auto-select trim, margins, bleed, spine logic, PDF checks | **NOT IMPLEMENTED** — ExportPanel exposes formats but no KDP preset |
| `target = ebook only` | Hide print-only settings unless expanded | **NOT IMPLEMENTED** |
| `mode = beginner` | Hide advanced publishing settings behind "Advanced Options" | **NOT IMPLEMENTED** as an explicit mode toggle |

## Recommended implementation

Add a single `BookKind` field to `ProjectBrief`: `"fiction" | "non-fiction" | "childrens" | "memoir" | "poetry"`. Every downstream surface (wizard, AgentsPanel switchboard, ExportPanel, ValidatorPanel) reads this and shows / hides controls. This is a one-day change and unlocks the rest.
