# Prompt-Template Archive

> **Purpose.** Every prompt template is hash-versioned. When a
> template is updated, the *previous* version is moved here so
> historical `ai_calls` rows that reference the old hash remain
> resolvable for audit and replay.
>
> **Refs:** `EXTERNAL_AUDIT_BACKLOG.md #56`,
> `outputs/AGENTS.md`, `crates/booksforge-storage/migrations/`
> (the `ai_calls.template_hash` column).

---

## Why archive instead of just bumping the version

Every entry in `ai_calls` carries a `template_hash` field naming the
exact template body that produced the model's response. When the
team updates a template (e.g. `outline-architect/v1.toml` → `v2.toml`),
two requirements collide:

1. **Forward:** new runs should use the new template.
2. **Backward:** old `ai_calls` rows must remain *resolvable* —
   `template_hash → file path` must always succeed for any row in
   the database.

If we just deleted `v1.toml`, requirement (2) fails. If we only
add `v2.toml` next to `v1.toml`, the directory grows without
bound and nothing tells us which is "current".

The convention this directory enforces:

```
templates/
├── outline-architect/
│   ├── v2.toml                     ← current, what new runs use
│   └── archive/
│       └── outline-architect-v1-<hash>.toml  ← previous, frozen
├── final-review-editor/
│   └── v1.toml
└── archive/                        ← cross-cutting archive
    └── README.md (this file)
```

`booksforge-prompt`'s loader checks **both** the per-agent
directory and `archive/` when resolving a template hash. The build
verifies that every `template_hash` recorded in fixture
`ai_calls` rows resolves to an existing file.

---

## When to archive

Archive a template's old version if **any** of the following is
true:

- A `template_hash` value referencing it has ever been written to
  a `project.db` (which happens the first time a real run uses
  the template; the team's fixtures count too).
- The template has been published in a tagged release.
- A peer-review or audit cycle has cited the template by hash.

For pre-release in-development templates that have never produced
an `ai_calls` row, archiving is optional. When in doubt, archive.

---

## How to archive a template

1. Compute the hash that the loader uses for the OLD version. The
   loader's hashing function lives at
   `crates/booksforge-prompt/src/lib.rs` (function
   `template_hash`).
2. Move the old file:
   ```
   mv  templates/outline-architect/v1.toml \
       templates/outline-architect/archive/outline-architect-v1-<hash>.toml
   ```
   The `<hash>` suffix is the first 12 chars of the loader's hash
   over the OLD file — this protects against accidental re-archive
   of a different file.
3. Add a one-line entry to `templates/<agent>/archive/CHANGES.md`
   (or create that file if missing).
4. Commit the move + the new template in the same commit. Bump
   the spec doc in `outputs/AGENTS.md` if the prompt's contract
   changed.

The pre-commit `lefthook.yml` should grow a hook that fails the
commit if a known-archived `template_hash` is no longer resolvable.

---

## Cross-cutting archive (this directory)

Use `templates/archive/` for templates that have been **fully
retired** (no longer current and not associated with any active
agent). Examples: an experimental agent that was scrapped before
launch.

Within this directory, the structure is:

```
archive/
├── README.md  (this file)
├── CHANGES.md (chronological log of archives)
└── <agent>/
    └── <agent>-<vN>-<hash>.toml
```

---

## Audit query

The release-pipeline preflight should run:

```bash
cd booksforge
# Check every template_hash in fixture ai_calls rows resolves.
cargo run -p booksforge-prompt --example audit-template-hashes
```

If any hash fails to resolve, the release fails fast. Implementation
of `audit-template-hashes` belongs to the team — this README codifies
the contract that test enforces.

---

*This file is the structural-only half of `EXTERNAL_AUDIT_BACKLOG.md
#56`. The audit query implementation is the team's work, blocked
on Stabilisation Sprint S1 landing the in-flight prompt-crate
changes.*
