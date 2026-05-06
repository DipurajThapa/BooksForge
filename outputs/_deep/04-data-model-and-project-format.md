# Data Model & Project Format — BooksForge

**Version:** 1.0.0-draft  •  **Status:** For review  •  **Date:** 2026-05-06

---

## 1. Goals of the data model

Manuscripts are long-lived. A project started in 2026 must open in 2036. The data model therefore prioritises **stability**, **forward compatibility**, **partial recoverability**, and **inspectability** over compactness or write speed.

## 2. On-disk project bundle

A project is a directory with a `.booksforge` extension. On macOS the directory is marked as a package via UTI so it appears as one icon. On Windows and Linux it is a normal folder.

```
MyBook.booksforge/
├── manifest.toml                # Top-level manifest (versioned, signed if from marketplace)
├── project.db                   # SQLite — all structured data (see schema below)
├── project.db-wal               # SQLite WAL (transient)
├── project.db-shm               # SQLite shared memory (transient)
├── manuscript/                  # Cached Markdown export of every scene (round-trippable)
│   ├── 01-part-one/
│   │   └── 01-chapter-one/
│   │       ├── 001-scene-opening.md
│   │       └── 002-scene-confrontation.md
│   └── ...
├── assets/                      # Images, fonts, equations, attachments — content-addressed
│   ├── ab/cd1234…ef.png
│   └── ...
├── snapshots/                   # Content-addressed snapshot store (dedupe)
│   ├── manifest.json
│   └── objects/
│       ├── ab/cd1234…ef
│       └── ...
├── exports/                     # Last N exports kept for diff/inspect
│   └── 2026-05-06T14-22-MyBook.epub
├── plugins/                     # Project-pinned plugin instances (manifest only)
│   └── enabled.toml
├── .lock                        # Advisory lock file
└── .booksforge-version           # Plain text: minimum app version that can open
```

**Why both SQLite and Markdown?** SQLite is the source of truth for structured data (relationships, metadata, validator caches). The `manuscript/` Markdown mirror is regenerated on save and is the **disaster-recovery surface**: if `project.db` is corrupted, a re-import from `manuscript/` reconstructs ≥95% of the project. Markdown is also git-friendly and human-readable.

## 3. `manifest.toml`

```toml
[project]
id = "01HF8X5ZQK0QY9YV6S0R6P8N3K"   # ULID, immutable
schema_version = 5                   # See §5
app_version_min = "1.0.0"
app_version_built = "1.2.3"
created_at = "2026-04-12T09:14:22Z"
updated_at = "2026-05-06T11:02:01Z"
language = "en-US"
mode = "fiction"                     # fiction | non_fiction | academic
template = { id = "romance-mass-market", version = "2.1.0" }

[meta]
title = "The Cartographer's Daughter"
subtitle = ""
authors = [{ name = "Anya Becker", role = "author" }]
isbn_print = "978-1-23456-789-0"
isbn_ebook = ""
genre = ["romance", "historical"]
tags = ["regency", "series-novel"]
target_word_count = 95000

[encryption]
enabled = false
algorithm = ""
salt = ""
kdf = ""

[integrity]
manifest_hash_alg = "blake3"
project_db_hash = ""                 # written on close, validated on open
manuscript_tree_hash = ""

[plugins]
required = []                        # plugins required to open project

[signing]                            # only present if from marketplace
publisher = ""
public_key = ""
signature = ""
```

## 4. SQLite schema (V1, 5th iteration)

We model the project as a **typed tree** of nodes plus tables for cross-cutting metadata. Every node has a stable ULID and a soft-delete flag.

```sql
-- ---------------------------------------------------------------
-- Migrations table
-- ---------------------------------------------------------------
CREATE TABLE schema_migrations (
  version       INTEGER PRIMARY KEY,
  applied_at    TEXT NOT NULL,
  description   TEXT NOT NULL,
  checksum      TEXT NOT NULL
);

-- ---------------------------------------------------------------
-- Document tree
-- ---------------------------------------------------------------
CREATE TABLE nodes (
  id            TEXT PRIMARY KEY,           -- ULID
  parent_id     TEXT REFERENCES nodes(id) ON DELETE CASCADE,
  kind          TEXT NOT NULL CHECK (kind IN ('project','part','chapter','scene','front_matter','back_matter')),
  title         TEXT NOT NULL DEFAULT '',
  position      INTEGER NOT NULL,           -- LexoRank for stable reorder
  status        TEXT DEFAULT 'planned',     -- planned|drafting|revised|final
  pov           TEXT,
  beat          TEXT,
  target_words  INTEGER,
  created_at    TEXT NOT NULL,
  updated_at    TEXT NOT NULL,
  deleted_at    TEXT
);

CREATE INDEX idx_nodes_parent ON nodes(parent_id);
CREATE INDEX idx_nodes_kind   ON nodes(kind);

-- ---------------------------------------------------------------
-- Scene content (ProseMirror JSON, plus a derived Markdown mirror)
-- ---------------------------------------------------------------
CREATE TABLE scene_content (
  node_id        TEXT PRIMARY KEY REFERENCES nodes(id) ON DELETE CASCADE,
  pm_doc_json    BLOB NOT NULL,             -- ProseMirror document, zstd-compressed
  word_count     INTEGER NOT NULL DEFAULT 0,
  char_count     INTEGER NOT NULL DEFAULT 0,
  hash           TEXT NOT NULL,             -- blake3 of canonical pm_doc_json
  updated_at     TEXT NOT NULL
);

-- ---------------------------------------------------------------
-- Notes (per-scene, per-project)
-- ---------------------------------------------------------------
CREATE TABLE notes (
  id            TEXT PRIMARY KEY,
  node_id       TEXT REFERENCES nodes(id) ON DELETE CASCADE,
  body          TEXT NOT NULL,
  kind          TEXT DEFAULT 'general',     -- general|todo|question
  created_at    TEXT NOT NULL,
  updated_at    TEXT NOT NULL
);

-- ---------------------------------------------------------------
-- Series bible
-- ---------------------------------------------------------------
CREATE TABLE entities (
  id            TEXT PRIMARY KEY,
  type          TEXT NOT NULL,              -- character|location|item|organisation|theme|custom
  name          TEXT NOT NULL,
  fields_json   BLOB NOT NULL,              -- typed per `type`
  notes         TEXT,
  created_at    TEXT NOT NULL,
  updated_at    TEXT NOT NULL,
  deleted_at    TEXT
);

CREATE TABLE entity_aliases (
  entity_id     TEXT REFERENCES entities(id) ON DELETE CASCADE,
  alias         TEXT NOT NULL,
  PRIMARY KEY (entity_id, alias)
);

CREATE TABLE entity_scene_appearances (
  entity_id     TEXT REFERENCES entities(id) ON DELETE CASCADE,
  node_id       TEXT REFERENCES nodes(id) ON DELETE CASCADE,
  confirmed     INTEGER NOT NULL DEFAULT 0, -- 0=auto-suggested, 1=user-confirmed
  PRIMARY KEY (entity_id, node_id)
);

-- ---------------------------------------------------------------
-- Footnotes / endnotes / citations / cross-refs
-- ---------------------------------------------------------------
CREATE TABLE references (
  id            TEXT PRIMARY KEY,
  kind          TEXT NOT NULL,              -- footnote|endnote|citation|crossref
  node_id       TEXT REFERENCES nodes(id) ON DELETE CASCADE,
  body          TEXT NOT NULL,              -- footnote text OR citation key OR target ref id
  csl_key       TEXT,                       -- CSL key for citations
  created_at    TEXT NOT NULL
);

CREATE TABLE bibliography (
  csl_key       TEXT PRIMARY KEY,
  csl_json      BLOB NOT NULL,              -- CSL-JSON record
  imported_from TEXT,                       -- bibtex|zotero|manual
  updated_at    TEXT NOT NULL
);

-- ---------------------------------------------------------------
-- Comments and tracked changes
-- ---------------------------------------------------------------
CREATE TABLE comments (
  id            TEXT PRIMARY KEY,
  node_id       TEXT REFERENCES nodes(id) ON DELETE CASCADE,
  range_from    INTEGER NOT NULL,
  range_to      INTEGER NOT NULL,
  parent_id     TEXT REFERENCES comments(id),
  author        TEXT NOT NULL,
  body          TEXT NOT NULL,
  resolved_at   TEXT,
  created_at    TEXT NOT NULL
);

CREATE TABLE tracked_changes (
  id            TEXT PRIMARY KEY,
  node_id       TEXT REFERENCES nodes(id) ON DELETE CASCADE,
  op            TEXT NOT NULL,              -- insert|delete|format
  range_from    INTEGER NOT NULL,
  range_to      INTEGER NOT NULL,
  payload_json  BLOB NOT NULL,
  author        TEXT NOT NULL,
  state         TEXT NOT NULL DEFAULT 'pending', -- pending|accepted|rejected
  created_at    TEXT NOT NULL
);

-- ---------------------------------------------------------------
-- Snapshots (manifest table; objects in snapshots/objects/)
-- ---------------------------------------------------------------
CREATE TABLE snapshots (
  id            TEXT PRIMARY KEY,
  scope         TEXT NOT NULL,              -- project|part|chapter|scene
  scope_id      TEXT,                       -- node id when scope != project
  label         TEXT,
  trigger       TEXT NOT NULL,              -- manual|auto|pre_ai|pre_export|pre_migration
  tree_hash     TEXT NOT NULL,              -- root content-address into snapshots/objects
  created_at    TEXT NOT NULL,
  size_bytes    INTEGER NOT NULL
);

-- ---------------------------------------------------------------
-- Validator cache
-- ---------------------------------------------------------------
CREATE TABLE validator_runs (
  id            TEXT PRIMARY KEY,
  validator_id  TEXT NOT NULL,              -- e.g. 'kdp.print.v1'
  ran_at        TEXT NOT NULL,
  status        TEXT NOT NULL,              -- ok|warnings|errors|crashed
  duration_ms   INTEGER NOT NULL,
  scope_hash    TEXT NOT NULL               -- input hash; results valid until any input changes
);

CREATE TABLE validator_issues (
  id            TEXT PRIMARY KEY,
  run_id        TEXT REFERENCES validator_runs(id) ON DELETE CASCADE,
  severity      TEXT NOT NULL,              -- error|warning|info
  category      TEXT NOT NULL,
  rule_id       TEXT NOT NULL,
  node_id       TEXT,
  range_from    INTEGER,
  range_to      INTEGER,
  message       TEXT NOT NULL,
  fix_kind      TEXT,                       -- none|deterministic|suggested
  fix_payload   BLOB
);

-- ---------------------------------------------------------------
-- AI audit log
-- ---------------------------------------------------------------
CREATE TABLE ai_calls (
  id            TEXT PRIMARY KEY,
  node_id       TEXT,
  provider      TEXT NOT NULL,              -- llamacpp|ollama|anthropic|openai|...
  model         TEXT NOT NULL,
  preset        TEXT,                       -- 'sharpen'|'expand'|...|custom
  prompt_template_hash TEXT NOT NULL,
  context_tokens INTEGER NOT NULL,
  output_tokens INTEGER NOT NULL,
  duration_ms   INTEGER NOT NULL,
  cost_estimate_usd REAL,
  status        TEXT NOT NULL,              -- ok|cancelled|error
  error         TEXT,
  created_at    TEXT NOT NULL
);

-- ---------------------------------------------------------------
-- Plugin state
-- ---------------------------------------------------------------
CREATE TABLE plugin_installs (
  plugin_id     TEXT PRIMARY KEY,
  version       TEXT NOT NULL,
  capabilities_granted TEXT NOT NULL,       -- JSON array
  installed_at  TEXT NOT NULL,
  enabled       INTEGER NOT NULL DEFAULT 1
);

-- ---------------------------------------------------------------
-- Export history
-- ---------------------------------------------------------------
CREATE TABLE exports (
  id            TEXT PRIMARY KEY,
  format        TEXT NOT NULL,              -- docx|pdf|epub|tex|md|html
  profile       TEXT,                       -- 'kdp.ebook'|'ingramspark.print'|...
  path          TEXT NOT NULL,
  template      TEXT NOT NULL,
  template_version TEXT NOT NULL,
  app_version   TEXT NOT NULL,
  validators_run TEXT NOT NULL,             -- JSON
  duration_ms   INTEGER NOT NULL,
  bytes         INTEGER NOT NULL,
  hash          TEXT NOT NULL,
  created_at    TEXT NOT NULL
);
```

## 5. Schema versioning

`schema_version` lives in `manifest.toml`. Migrations are forward-only at runtime; reverse migrations exist as scripts for support. Before any migration runs, an automatic snapshot tagged `pre_migration` is taken.

**Forward-compatibility rule:** newer apps read older schemas. Older apps refuse to open newer schemas with a clear message ("This project was last opened with BooksForge 1.4 — please update to open it.")

## 6. Content-addressed asset store

`assets/` is a [content-addressed](https://en.wikipedia.org/wiki/Content-addressable_storage) store: file path is `assets/<first-2-chars-of-hash>/<rest>.<ext>`. The `assets` table in SQLite indexes hashes with metadata (mime type, original filename, alt text, dimensions). Re-using the same image across scenes does not duplicate bytes. Delete-from-bundle uses GC: any asset hash not referenced anywhere is removed during a periodic compaction.

## 7. Snapshot storage

Snapshots are content-addressed objects under `snapshots/objects/`. A "tree" object lists `(node_id → content-hash)` pairs. A "scene-content" object is a zstd-compressed ProseMirror JSON. Diffs are computed at the node level; identical nodes between snapshots share the same object hash and aren't duplicated. Deleting a snapshot is a tombstone; periodic compaction removes orphaned objects.

## 8. Locking

Advisory file lock on `.lock` (using `fs2` or platform locking primitives). On open, write `{pid, host, locked_at}`. On close, remove. On open with an existing lock: confirm pid is alive on this host; if not, recover; if yes, refuse and surface "already open in another instance" UI.

## 9. Crash recovery

WAL mode preserves committed transactions across crashes. The autosave queue is journalled to a recovery log inside the bundle (`.recovery.log`) with append-only writes. On launch, if the recovery log has entries newer than `project_db_hash`, surface a recovery dialog: "We found unsaved work from your last session. Recover?"

## 10. Encryption (when enabled)

Per project: a master key is derived from the user's passphrase via Argon2id (id 19, m=64MiB, t=3, p=1). Each scene-content blob, asset, and snapshot object is encrypted with AES-256-GCM with a random nonce. Manifest stores the salt and KDF parameters; the master key never leaves memory. The database file itself uses [SQLCipher](https://www.zetetic.net/sqlcipher/) for transparent page-level encryption (alternative considered: `rusqlite` + manual at-rest encryption — rejected for complexity).

## 11. Backups and portability

The bundle is one folder — copy/zip/move/sync are all native. We expose **Save Self-Contained Copy** which produces `MyBook.booksforge.zip` with all referenced assets. Importantly, this includes the Markdown mirror, so even if a future BooksForge can't open the project DB, the user's words remain readable.

## 12. Validator inputs

Validators take a typed `ProjectView` (read-only handle) plus a `ValidatorContext` (clock, RNG seed if needed, plugin host). They output `Issue[]`. They are pure functions of inputs and produce deterministic results — important for caching and reproducibility.

## 13. Reasoning about state

We avoid global mutable state. Application state is held in a single `AppState` (read-mostly via `RwLock`) and project state is loaded on open and held in a `ProjectState`. Every long-running job receives an immutable snapshot of the relevant state at the moment of dispatch — this keeps async tasks deterministic and avoids the "value changed under me" class of bug.
