# BooksForge — Audit Scripts

> Static checks that translate `EXTERNAL_AUDIT_BACKLOG.md` items into
> runnable, mechanical pass/fail tests. Each script prints exactly
> what would need to change (file:line) and exits 1 if the audit item
> is still open.
>
> These scripts are **runnable today** against the working tree —
> they don't require the team's in-flight refactor to land first.
> When an audit item is genuinely closed, its script becomes a CI
> regression guard.
>
> **Refs:** `EXTERNAL_AUDIT_BACKLOG.md`,
> `.github/workflows/audit-checks.yml` (CI wiring).

---

## How to run

```bash
# Run every audit script.
./scripts/audit/run-all.sh

# Run a single check.
./scripts/audit/check-csp-no-unsafe-inline.sh
```

All scripts:
- exit `0` if the audit item is currently satisfied,
- exit `1` (and print details) if the item is still open,
- assume the working directory is the repo root.

CI runs all of them on every PR via
`.github/workflows/audit-checks.yml`. **Failing scripts do not
block merge** in the initial rollout — they post a check-summary
comment so the team can see at-a-glance which audit items still
need attention without each one being a per-PR blocker. Once the
team has worked the open items down, individual scripts will be
promoted to required status checks per `docs/REPO_SETTINGS.md §1`.

---

## Catalogue

| Script | Audit item | Type |
|--------|------------|------|
| `check-csp-no-unsafe-inline.sh` | #15 | Static grep on `tauri.conf.json` |
| `check-tauri-conf-metadata.sh` | #44 | JSON-key presence on `tauri.conf.json` |
| `check-no-handwritten-ipc-types.sh` | #19 | Grep for hand-written types in `shared-types/src/` |
| `check-unsafe-blocks-documented.sh` | #12 | Grep for undocumented `unsafe {` blocks |
| `check-let-underscore-budget.sh` | #18 | Count `let _ =` discards in `apps/desktop/src/commands/` |
| `check-no-manuscript-over-wire.sh` | #8 | Static grep for crates that import both `reqwest` and manuscript types |
| `check-no-todo-tracker.sh` | (hygiene) | Counts `TODO` / `FIXME` / `XXX` / `HACK` in source; reports trend |
| `run-all.sh` | (runner) | Runs every script and prints a summary table |

---

## Adding a new audit script

1. Create `scripts/audit/check-<short-name>.sh` from the template
   below.
2. Add a row to the catalogue above.
3. Add a step to `.github/workflows/audit-checks.yml`.

### Template

```bash
#!/usr/bin/env bash
# Closes audit #NN — <short title>.
#
# Pass criterion: <one sentence>.
# Fail behaviour: prints file:line of every offending site and exits 1.

set -euo pipefail

# Anchor to repo root regardless of where the script is invoked from.
cd "$(dirname "$0")/../.."

OFFENDERS=$(your_check_here)

if [[ -z "${OFFENDERS}" ]]; then
  echo "✅ #NN — <short title>: clean."
  exit 0
fi

echo "❌ #NN — <short title>: still open. Offenders:"
echo "${OFFENDERS}"
echo
echo "Resolution: see EXTERNAL_AUDIT_BACKLOG.md #NN."
exit 1
```

---

*Last updated 2026-05-08.*
