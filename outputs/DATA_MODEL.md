# Data Model — BooksForge

**Version:** 1.0.0  •  **Date:** 2026-05-06  •  **Authoritative for the implementation schema.**

This document is the implementation-ready data model — the SQL schema and on-disk layout Claude Code builds against.

---

## 1. Entities at a glance

| Entity | Storage | Purpose |
|--------|---------|---------|
| Project | `manifest.toml` + `nodes` table | Top of the tree |
| Book mode & template | `manifest.toml` | Determines defaults |
| Node (Part / Chapter / Scene / Front / Back matter) | `nodes` | Document tree |
| Scene content | `scene_content` | ProseMirror JSON + word/char counts |
| Note | `notes` | Free-text notes attached to nodes |
| Entity (character / location / item / org / theme) | `entities`, `entity_aliases`, `entity_scene_appearances` | Series bible |
| Reference (footnote / endnote / citation / xref) | `references` | Inline references |
| Bibliography entry | `bibliography` | CSL-JSON records (V1.0) |
| Comment | `comments` | Inline comments threaded |
| Tracked change | `tracked_changes` | Editor round-trip (V1.0) |
| Snapshot | `snapshots` + `snapshots/objects/` | Versioning |
| Validator run / issue | `validator_runs`, `validator_issues` | Validation cache |
| Agent run / task / output / applied edit | **NEW** `agent_runs`, `agent_tasks`, `agent_outputs`, `agent_applied_edits` | Agent layer |
| Style book | **NEW** `style_book` | Project style choices |
| Plugin install | `plugin_installs` | Post-MVP |
| Export | `exports` | Export history |
| Settings | `settings` (per-project) and `~/.booksforge/settings.toml` (per-user) | Configuration |
| Model settings | **NEW** `model_settings` (per-project) and `~/.booksforge/models.toml` (per-user) | Ollama defaults |

The deep spec (`04-…`) covers everything except the bolded NEW entries. The new tables are specified below.

## 2. Project entity

A project is rooted at `manifest.toml`. The MVP additions:

```toml
[project]
id = "01HF8X5ZQK0QY9YV6S0R6P8N3K"
schema_version = 1                    # MVP starts at v1 (greenfield)
app_version_min = "1.0.0"
app_version_built = "1.2.3"
created_at = "2026-04-12T09:14:22Z"
updated_at = "2026-05-06T11:02:01Z"
language = "en-US"
mode = "fiction"                      # fiction | non_fiction | memoir | academic
template = { id = "fiction-generic-novel", version = "1.0.0" }

# … unchanged: meta, encryption, integrity, plugins, signing …

[ai]
enabled = false                       # explicit consent required
default_runtime = "ollama"            # MVP: only ollama
ollama_host = "http://127.0.0.1:11434"
default_model = "qwen2.5:7b-instruct-q4_K_M"
agent_runtime_caps = { max_calls = 8, max_minutes = 10, max_tokens = 200000, max_retries = 3 }

[style_book]                          # mirrors style_book table for portability
em_dash = "em"                        # em | en | hyphen
oxford_comma = true
quote_style = "smart"                 # smart | straight
spaces_after_period = 1
ellipsis_form = "single_glyph"        # single_glyph | three_dots
serial_dialogue_attribution = "comma"
locale_overrides = { "spelling" = "en-US" }
```

The `[ai]` and `[style_book]` blocks are persisted to disk on every save; they round-trip to the in-database `model_settings` and `style_book` tables.

## 3. Document tree

Per `04-… §4`, unchanged:

```sql
CREATE TABLE nodes (
  id            TEXT PRIMARY KEY,
  parent_id     TEXT REFERENCES nodes(id) ON DELETE CASCADE,
  kind          TEXT NOT NULL CHECK (kind IN ('project','part','chapter','scene','front_matter','back_matter')),
  title         TEXT NOT NULL DEFAULT '',
  position      INTEGER NOT NULL,        -- LexoRank
  status        TEXT DEFAULT 'planned',  -- planned|drafting|revised|final
  pov           TEXT,
  beat          TEXT,
  target_words  INTEGER,
  created_at    TEXT NOT NULL,
  updated_at    TEXT NOT NULL,
  deleted_at    TEXT
);
```

Plus `scene_content`, `notes`, `entities`, `entity_aliases`, `entity_scene_appearances`, `references`, `bibliography`, `comments`, `tracked_changes`, `snapshots`, `validator_runs`, `validator_issues`, `plugin_installs`, `exports` — all per `04-…`. The MVP **uses but does not write to** `tracked_changes`, `bibliography`, `plugin_installs` — those are V1.0 surfaces.

## 4. Style book

The style book is the project's mechanical-style choices. The Copyeditor Agent reads from it; the user edits it from project settings.

```sql
CREATE TABLE style_book (
  -- one row per project (singleton)
  id                     INTEGER PRIMARY KEY CHECK (id = 1),
  em_dash                TEXT NOT NULL DEFAULT 'em',           -- em|en|hyphen
  oxford_comma           INTEGER NOT NULL DEFAULT 1,
  quote_style            TEXT NOT NULL DEFAULT 'smart',        -- smart|straight
  spaces_after_period    INTEGER NOT NULL DEFAULT 1,
  ellipsis_form          TEXT NOT NULL DEFAULT 'single_glyph',
  spelling_locale        TEXT NOT NULL DEFAULT 'en-US',
  capitalize_after_colon INTEGER NOT NULL DEFAULT 0,
  bold_emphasis_allowed  INTEGER NOT NULL DEFAULT 0,
  custom_rules_json      BLOB,                                  -- forward-compat
  updated_at             TEXT NOT NULL
);
```

## 5. Agent layer schema

These tables are the audit and run-state for the agent swarm. They are the most important new schema in this doc.

```sql
-- ---------------------------------------------------------------
-- An agent_run is one user-initiated workflow (e.g., "Copyedit chapter 3")
-- ---------------------------------------------------------------
CREATE TABLE agent_runs (
  id              TEXT PRIMARY KEY,                  -- ULID
  workflow_id     TEXT NOT NULL,                     -- e.g. 'IntakeAndOutline', 'Copyedit'
  scope           TEXT NOT NULL,                     -- 'project'|'part'|'chapter'|'scene'
  scope_id        TEXT,                              -- node ULID when scope != project
  status          TEXT NOT NULL,                     -- 'running'|'awaiting_user'|'completed'|'cancelled'|'error'|'invalid'
  started_at      TEXT NOT NULL,
  completed_at    TEXT,
  cancel_reason   TEXT,
  caps_json       BLOB NOT NULL,                     -- captured caps at start
  totals_json     BLOB,                              -- aggregated tokens/duration after completion
  ollama_version  TEXT,
  user_initiated  INTEGER NOT NULL DEFAULT 1
);

CREATE INDEX idx_agent_runs_workflow ON agent_runs(workflow_id, started_at);

-- ---------------------------------------------------------------
-- An agent_task is one call to one agent inside an agent_run
-- ---------------------------------------------------------------
CREATE TABLE agent_tasks (
  id                    TEXT PRIMARY KEY,            -- ULID
  run_id                TEXT NOT NULL REFERENCES agent_runs(id) ON DELETE CASCADE,
  step_index            INTEGER NOT NULL,            -- 0-based order within the run
  agent_id              TEXT NOT NULL,               -- 'intake'|'outline-architect'|...
  prompt_template_id    TEXT NOT NULL,               -- e.g. 'outline-architect.v1'
  prompt_template_hash  TEXT NOT NULL,               -- blake3 of the template file
  model                 TEXT NOT NULL,               -- e.g. 'qwen2.5:7b-instruct-q4_K_M'
  model_digest          TEXT,                        -- from Ollama /api/show
  input_hash            TEXT NOT NULL,               -- blake3 of the assembled context bundle
  output_hash           TEXT,                        -- blake3 of the parsed output
  context_tokens        INTEGER,
  output_tokens         INTEGER,
  duration_ms           INTEGER,
  retries               INTEGER NOT NULL DEFAULT 0,
  status                TEXT NOT NULL,               -- 'running'|'completed'|'invalid'|'cancelled'|'error'
  error_category        TEXT,                        -- 'schema'|'semantic'|'external'|'timeout'|'cancelled'
  error_message         TEXT,                        -- human-readable; never raw stack traces
  created_at            TEXT NOT NULL,
  updated_at            TEXT NOT NULL
);

CREATE INDEX idx_agent_tasks_run ON agent_tasks(run_id, step_index);
CREATE INDEX idx_agent_tasks_agent ON agent_tasks(agent_id, created_at);

-- ---------------------------------------------------------------
-- agent_outputs holds the parsed proposal (schema-valid only)
-- Large outputs (>4 KB) are stored in agent_runs/<run_id>/<task_id>.json
-- and content_inline is NULL.
-- ---------------------------------------------------------------
CREATE TABLE agent_outputs (
  task_id         TEXT PRIMARY KEY REFERENCES agent_tasks(id) ON DELETE CASCADE,
  schema_id       TEXT NOT NULL,                     -- e.g. 'OutlineProposal'
  schema_version  INTEGER NOT NULL,
  content_inline  BLOB,                              -- JSON, zstd-compressed if >1 KB
  content_path    TEXT,                              -- path inside the bundle when not inline
  hash            TEXT NOT NULL,
  validated_at    TEXT NOT NULL
);

-- ---------------------------------------------------------------
-- agent_applied_edits records every edit the user accepted from a proposal
-- One row per applied edit; each row triggers a pre_agent_edit snapshot.
-- ---------------------------------------------------------------
CREATE TABLE agent_applied_edits (
  id                  TEXT PRIMARY KEY,
  task_id             TEXT NOT NULL REFERENCES agent_tasks(id) ON DELETE RESTRICT,
  node_id             TEXT NOT NULL REFERENCES nodes(id) ON DELETE RESTRICT,
  pre_edit_snapshot_id TEXT NOT NULL REFERENCES snapshots(id) ON DELETE RESTRICT,
  applied_at          TEXT NOT NULL,
  edit_kind           TEXT NOT NULL,                 -- 'text_replace'|'rename_entity'|'reorder'|'note_add'
  edit_payload_json   BLOB NOT NULL,                 -- canonical edit description
  reverted_at         TEXT
);

CREATE INDEX idx_agent_edits_node ON agent_applied_edits(node_id, applied_at);
```

### 5.1 Invariants (enforced in code, not just SQL)

- For every `agent_applied_edits` row, `pre_edit_snapshot_id` must reference a snapshot whose `created_at < applied_at`. If not, the orchestrator refuses to commit.
- For every `agent_tasks` row with status `completed`, an `agent_outputs` row must exist.
- The `prompt_template_hash` must match the on-disk template's blake3 hash at the time of the run; if the template changes between versions, both versions remain in the repo so the run is reproducible.
- The `model_digest` is captured from Ollama's `/api/show` immediately before the call. If absent, the row is marked `model_digest = unknown` rather than empty — never crash on missing digest.

## 6. Model settings

Per-user defaults in `~/.booksforge/models.toml`; per-project overrides in `model_settings`.

```sql
CREATE TABLE model_settings (
  id                 INTEGER PRIMARY KEY CHECK (id = 1),
  default_runtime    TEXT NOT NULL DEFAULT 'ollama',
  ollama_host        TEXT NOT NULL DEFAULT 'http://127.0.0.1:11434',
  default_model      TEXT NOT NULL,
  per_agent_overrides_json BLOB,                  -- { "copyeditor": "llama3.1:8b...", ... }
  caps_json          BLOB NOT NULL,
  updated_at         TEXT NOT NULL
);
```

## 7. AI calls (single-shot quick-action presets)

Single-shot inline presets (Sharpen, Continue, Rephrase, etc.) — **not** agents — are recorded in `ai_calls` per `04-… §4`. The MVP keeps that table for inline calls and uses the new `agent_runs / agent_tasks` tables for agent workflows. The two ledgers are independent and queryable separately.

Schema is unchanged from `04-…`; the only addition is a column for the runtime:

```sql
ALTER TABLE ai_calls ADD COLUMN runtime TEXT NOT NULL DEFAULT 'ollama';  -- 'ollama'|'cloud'(post-MVP)|'mock'
```

## 8. Snapshot kinds

Snapshot `trigger` values per `04-…` are extended:

- `manual` (user-clicked snapshot)
- `auto` (scheduled)
- `pre_ai` (pre-quick-action edit)
- `pre_agent_edit` — **new** — pre-agent-applied edit, mandatory
- `pre_export`
- `pre_migration`
- `crash_recovery` — **new** — created when the recovery flow merges a recovery log

The `snapshots.trigger CHECK` constraint adds these new values.

## 9. Migrations

The MVP starts at `schema_version = 1`. The deep spec (`_deep/04-data-model-and-project-format.md §4`) describes a notional v5 baseline; that was a planning convenience from before the implementation pack and does not reflect any shipped database. The first SQLite migration (M0 task **MZ-02**) creates everything in this document at v1 in one step.

The migration policy thereafter is per `04-… §5`:

- Forward migrations are written by hand and run automatically on open after a `pre_migration` snapshot.
- Reverse migrations exist as scripts for support; they are not auto-run.
- A version-jump (e.g., v3 directly to v5 because an intermediate jump introduces bugs) requires a tested intermediate-build path.
- Forward-compatibility rule (§10) is invariant.

Future schema bumps for the MVP build are anticipated when the V1.0 features land: tracked-changes round-trip tables, citation/CSL tables, encryption parameters, and plugin install records. Each will be its own migration with the same pattern.

## 10. Forward-compatibility rule

A v7 project must not crash a v6 build; it shows a clear "this project was last opened with a newer BooksForge — please update." Newer BooksForge always opens older projects with a one-time migration that takes a snapshot first.

## 11. Storage strategy summary

- **SQLite** for structured state (the source of truth).
- **Markdown mirror** under `manuscript/` regenerated on every save (disaster recovery).
- **Content-addressed assets** under `assets/<aa>/<rest>` (dedupe).
- **Content-addressed snapshots** under `snapshots/objects/`.
- **Per-run agent artifacts** under `agent_runs/<run_id>/<task_id>.json` for outputs over 4 KB.
- **Local-first.** No cloud writes in MVP.
- **Backups.** The bundle is one folder — copy/zip/move/sync are native. Self-contained export bundles to `*.booksforge.zip` for transport.

## 12. Example SQL queries

These are the queries the UI will run. They are stable for the MVP.

```sql
-- Recent agent runs for a project, with progress
SELECT id, workflow_id, status, started_at, completed_at,
       (SELECT COUNT(*) FROM agent_tasks t WHERE t.run_id = r.id) AS step_count,
       (SELECT COUNT(*) FROM agent_tasks t WHERE t.run_id = r.id AND t.status = 'completed') AS completed_count
FROM agent_runs r
ORDER BY started_at DESC
LIMIT 50;

-- All applied edits from a given run
SELECT e.id, e.node_id, e.edit_kind, e.applied_at, t.agent_id
FROM agent_applied_edits e
JOIN agent_tasks t ON t.id = e.task_id
WHERE t.run_id = ?
ORDER BY e.applied_at;

-- Edits per agent, last 30 days (Operations / curiosity)
SELECT t.agent_id, COUNT(*) AS applied
FROM agent_applied_edits e
JOIN agent_tasks t ON t.id = e.task_id
WHERE e.applied_at >= datetime('now','-30 days')
GROUP BY t.agent_id
ORDER BY applied DESC;
```
