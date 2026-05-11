# 05 — Friction Report

The five friction points that most degrade the first-time-author experience, ranked by user-impact.

## F1. Switchboard tax (AgentsPanel)

The user opens AgentsPanel and is presented with **14 agent cards** organised into 4 categories ("prose-mutating", "generating", "memory", "meta"). The "meta" category includes Proposal Validator and Peer Review which are *auto-invoked by the orchestrator* — the user should never need to know they exist. The "memory" category is also infrastructure, not a creative action.

A first-time author's mental model is "I want to: outline a book / draft a chapter / fix mistakes / make it sound human / publish it." The current panel exposes the implementation, not the intent.

**Fix:** group by *intent* (Plan / Draft / Polish / Publish) with at most 5 cards visible by default. Move Validator + Peer Review under an "Advanced" disclosure.

## F2. The AI-output-not-in-editor bug (FIXED in this session)

The user's specific report: chapter-drafter ran successfully but its output was buried in a collapsed `<details>` JSON view. There was no path to put the prose into the editor.

**Fix landed:** `GenericAgentForm.tsx` now renders generated prose as a readable preview, has an **Apply to scene** button that takes a `pre_ai` snapshot and writes the `pm_doc` into the scene, and the `onApplied` callback is threaded through `AgentsPanel` and `EditorShell` so the editor reloads. Verified by typecheck.

**Follow-up (BACKLOG §A9):** route the apply through the Orchestrator + audit ledger row referencing the snapshot.

## F3. No "Prepare for Publishing" one-click action

See artifact 4. The user reaches the export panel and is then dropped on their own filesystem. The product does not produce a per-platform package ready for upload; the user has to assemble it manually for each marketplace.

## F4. No book-type branching

A children's-book author and an allocator-grade strategy-book author see the exact same wizard, the exact same agent set, and the exact same export panel. The system has no `BookKind` field. As a result every author has to know which agents to ignore and which fields to fill.

## F5. Publishing jargon in user-facing copy

`BISAC`, `trim`, `bleed`, `ULID`, `task_id`, `pm_doc`, `EPUBCheck` appear in user-facing surfaces. Each one is a glossary tooltip away from being acceptable, but today they are bare terms. A first-time author who has never published before will not know what `BISAC` is and the product does not tell them.

(See artifact 6 for the full simplification list.)
