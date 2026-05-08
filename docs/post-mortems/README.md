# Post-Mortems

> **Refs:** `docs/RUNBOOK.md §12`.

Public post-mortems for SEV-0 and SEV-1 incidents. Required within
**7 days** of incident resolution per the runbook.

---

## Filing convention

```
docs/post-mortems/YYYY-MM-DD-<slug>.md
```

- `YYYY-MM-DD` is the incident's resolution date (the day the patch
  released, not the day the report came in).
- `<slug>` is a short kebab-case description (e.g. `manuscript-leak`,
  `bundle-corruption`, `updater-key-rotation`).

---

## Template

Copy the block below into a new file. Fill in every section. If a
section truly does not apply, write *N/A — <one-line reason>* rather
than deleting the heading; consistent shape makes year-over-year
comparison possible.

```markdown
# YYYY-MM-DD — <Short Title>

**Severity:** SEV-0 / SEV-1
**Versions affected:** vX.Y.Z..vX.Y.W
**Mitigated:** YYYY-MM-DD HH:MM UTC
**Resolved:** YYYY-MM-DD HH:MM UTC
**Author:** <name>
**Reviewers:** <names>

---

## Summary

One paragraph. What happened, who was affected, and what was the
worst-case impact (in terms of users / data / privacy).

## Timeline

All times in UTC.

| Time | Event |
|------|-------|
| YYYY-MM-DD HH:MM | Reporter contacted us via <channel>. |
| HH:MM | Triage decision: SEV-0. |
| HH:MM | Affected releases yanked. |
| HH:MM | Updater feed disabled. |
| HH:MM | Patch tagged vX.Y.Z+1. |
| HH:MM | Patch published; updater feed restored. |
| HH:MM | Incident closed. |

## What happened

Factual reconstruction. Resist the urge to assign blame; describe
the sequence of events as observable facts.

## Impact

- Users affected: ...
- Data exposed: ...
- Manuscript content exposed: yes / no.
- Public-facing communication: link to the GitHub Security Advisory.

## Root cause

The actual underlying cause, not the proximate trigger. Use the
"five whys" method. Stop when each "why" answer would not move
the team to a different action.

## What went well

The detection paths, the response steps, the communication that
worked. Worth preserving.

## What went wrong

The detection paths, response steps, communication that did not.
Things to fix in the runbook.

## Lessons

A bulleted list of single-sentence lessons that future-self should
remember. Be specific and actionable.

## Action items

Cross-link to GitHub issues. Each action item must have an owner
and a target date.

| # | Action | Owner | Target | Status |
|---|--------|-------|--------|--------|
| 1 | Add CI test for ... | @owner | YYYY-MM-DD | ⬜ |
| 2 | Update RUNBOOK.md to ... | @owner | YYYY-MM-DD | ⬜ |
| 3 | Document ... in EXTERNAL_AUDIT_BACKLOG.md | @owner | YYYY-MM-DD | ⬜ |

## Linked artefacts

- GitHub Security Advisory: <link>
- Patch release: <link>
- Reporter (with permission): <name / handle>
```

---

## Index

*(populated as post-mortems are filed; empty until the first
incident)*

---

*Tone: direct, factual, no minimising. The post-mortem is for the
team to learn and for users to trust us — not a performance.*
