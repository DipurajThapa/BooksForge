# 06 — UX Recommendations (ranked by leverage)

## Required UX fixes (ship before V1.0)

| # | Fix | Effort | Impact |
|---|---|---|---|
| R1 | Make AI-output populate the editor (FIXED this session) | 1d | Highest — without this the product fundamentally doesn't work |
| R2 | Add `BookKind` field + dynamic branching for fiction / non-fiction / children's | 1d backend, 2d UI | Unlocks all the per-type defaults below |
| R3 | Group AgentsPanel by intent (Plan / Draft / Polish / Publish), hide meta agents under Advanced | 1d | Removes the switchboard tax for first-timers |
| R4 | Single "Prepare for Publishing" action — produce per-platform package + HUMAN_REQUIRED checklist | 3-5d | Closes the export → marketplace gap |
| R5 | Replace publishing jargon with plain English + glossary tooltips (BISAC, trim, bleed, ULID, etc.) | 1d | Beginner-mode-readable |
| R6 | Add the four mandatory approval gates (topic, plan, character/world bibles, manuscript-pre-final-polish) | 2d | Restores user creative control |
| R7 | Surface the honest rubric score in the editor (don't hide 6.1/10) | 1d | Trust-builder per RCA §L3.3 |

## Nice-to-have UX improvements (post-V1.0)

| # | Improvement | Effort | Impact |
|---|---|---|---|
| N1 | Sample-chapter preview before whole-book draft | 2d | Saves wasted compute when voice is wrong |
| N2 | Per-chapter user-note hook between drafts (one-line tweak input) | 1d | Closes the human-in-loop gap |
| N3 | Auto-pre-fill Chapter Drafter form from outline node | 0.5d | Removes 60% of manual input |
| N4 | "Draft all scenes in this chapter" sweep action | 1d | Reduces repetition |
| N5 | Source-comp anchoring field in `ProjectBrief` (paste 1-3 paragraphs) | 1d | Per RCA §L2.3, lifts prose-rubric 0.5-1.0 pts |
| N6 | Streaming preview of generation (token-by-token, like QuickActionBar already does) for ALL generators, not just QuickActionBar | 2d | Perceived speed |
| N7 | Per-platform readiness checklist UI with HUMAN_REQUIRED markers | 1d | Honest about non-AI steps |

## Removals (consider cutting)

- The "Proposal Validator (Tier 2)" and "Peer Review" cards in AgentsPanel — these are auto-invoked, the user should never click them. Move them out of the user-facing switchboard entirely.
- Default to a single export format (EPUB) instead of asking the user to pick — most first-timers won't know what they need.
