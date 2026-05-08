-- Migration v1: initial schema.
-- Creates ALL tables for MVP so future migrations are purely additive.
-- Tables not written by MVP are created empty (refs, comments, tracked_changes).
--
-- IDs: TEXT (ULID string).  Timestamps: ISO-8601 TEXT UTC.  Booleans: INTEGER 0/1.

-- ── Migration bookkeeping ────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS schema_migrations (
    version     INTEGER PRIMARY KEY,
    applied_at  TEXT NOT NULL,
    description TEXT NOT NULL,
    checksum    TEXT NOT NULL
);

-- ── Document tree ────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS nodes (
    id            TEXT PRIMARY KEY NOT NULL,
    parent_id     TEXT REFERENCES nodes(id) ON DELETE CASCADE,
    kind          TEXT NOT NULL CHECK (kind IN (
                      'project','part','chapter','scene',
                      'front_matter','back_matter')),
    title         TEXT NOT NULL DEFAULT '',
    position      TEXT NOT NULL DEFAULT '0|hzzzzz:',
    status        TEXT NOT NULL DEFAULT 'planned'
                      CHECK (status IN ('planned','drafting','revised','final')),
    pov           TEXT,
    beat          TEXT,
    target_words  INTEGER,
    created_at    TEXT NOT NULL,
    updated_at    TEXT NOT NULL,
    deleted_at    TEXT
);

CREATE INDEX IF NOT EXISTS idx_nodes_parent ON nodes(parent_id);
CREATE INDEX IF NOT EXISTS idx_nodes_kind   ON nodes(kind);
CREATE INDEX IF NOT EXISTS idx_nodes_status ON nodes(status);

-- ── Scene content ────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS scene_content (
    node_id     TEXT PRIMARY KEY NOT NULL REFERENCES nodes(id) ON DELETE CASCADE,
    pm_doc      TEXT NOT NULL DEFAULT '{}',
    word_count  INTEGER NOT NULL DEFAULT 0,
    char_count  INTEGER NOT NULL DEFAULT 0,
    hash        TEXT NOT NULL DEFAULT '',
    updated_at  TEXT NOT NULL
);

-- ── Notes ────────────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS notes (
    id          TEXT PRIMARY KEY NOT NULL,
    node_id     TEXT REFERENCES nodes(id) ON DELETE CASCADE,
    body        TEXT NOT NULL,
    kind        TEXT NOT NULL DEFAULT 'general'
                    CHECK (kind IN ('general','todo','question')),
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_notes_node ON notes(node_id);

-- ── Series bible — entities ──────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS entities (
    id          TEXT PRIMARY KEY NOT NULL,
    kind        TEXT NOT NULL CHECK (kind IN (
                    'character','location','item',
                    'organisation','theme','custom')),
    name        TEXT NOT NULL,
    fields_json TEXT NOT NULL DEFAULT '{}',
    notes       TEXT NOT NULL DEFAULT '',
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL,
    deleted_at  TEXT
);

CREATE INDEX IF NOT EXISTS idx_entities_kind ON entities(kind);

CREATE TABLE IF NOT EXISTS entity_aliases (
    entity_id   TEXT NOT NULL REFERENCES entities(id) ON DELETE CASCADE,
    alias       TEXT NOT NULL,
    PRIMARY KEY (entity_id, alias)
);

CREATE TABLE IF NOT EXISTS entity_scene_appearances (
    entity_id   TEXT NOT NULL REFERENCES entities(id) ON DELETE CASCADE,
    node_id     TEXT NOT NULL REFERENCES nodes(id) ON DELETE CASCADE,
    confirmed   INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (entity_id, node_id)
);

-- ── Snapshots ────────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS snapshots (
    id          TEXT PRIMARY KEY NOT NULL,
    scope       TEXT NOT NULL CHECK (scope IN ('project','part','chapter','scene')),
    scope_id    TEXT,
    label       TEXT,
    trigger     TEXT NOT NULL CHECK (trigger IN (
                    'manual','auto','pre_ai','pre_export','pre_migration',
                    'pre_agent_edit','crash_recovery')),
    tree_hash   TEXT NOT NULL,
    created_at  TEXT NOT NULL,
    size_bytes  INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_snapshots_scope ON snapshots(scope, scope_id, created_at);

-- ── Validator cache ──────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS validator_runs (
    id           TEXT PRIMARY KEY NOT NULL,
    validator_id TEXT NOT NULL,
    ran_at       TEXT NOT NULL,
    status       TEXT NOT NULL CHECK (status IN ('ok','warnings','errors','crashed')),
    duration_ms  INTEGER NOT NULL,
    scope_hash   TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS validator_issues (
    id           TEXT PRIMARY KEY NOT NULL,
    run_id       TEXT NOT NULL REFERENCES validator_runs(id) ON DELETE CASCADE,
    node_id      TEXT REFERENCES nodes(id) ON DELETE SET NULL,
    severity     TEXT NOT NULL CHECK (severity IN ('info','warning','error')),
    code         TEXT NOT NULL,
    message      TEXT NOT NULL,
    offset_from  INTEGER,
    offset_to    INTEGER,
    auto_fixable INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_validator_issues_run ON validator_issues(run_id, severity);

-- ── Style book (singleton) ───────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS style_book (
    id                     INTEGER PRIMARY KEY CHECK (id = 1),
    em_dash                TEXT NOT NULL DEFAULT 'em'
                               CHECK (em_dash IN ('em','en','hyphen')),
    oxford_comma           INTEGER NOT NULL DEFAULT 1,
    quote_style            TEXT NOT NULL DEFAULT 'smart'
                               CHECK (quote_style IN ('smart','straight')),
    spaces_after_period    INTEGER NOT NULL DEFAULT 1,
    ellipsis_form          TEXT NOT NULL DEFAULT 'single_glyph'
                               CHECK (ellipsis_form IN ('single_glyph','three_dots')),
    spelling_locale        TEXT NOT NULL DEFAULT 'en-US',
    capitalize_after_colon INTEGER NOT NULL DEFAULT 0,
    bold_emphasis_allowed  INTEGER NOT NULL DEFAULT 0,
    custom_rules_json      TEXT NOT NULL DEFAULT '[]',
    updated_at             TEXT NOT NULL
);

-- ── Model settings (singleton) ───────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS model_settings (
    id                        INTEGER PRIMARY KEY CHECK (id = 1),
    default_runtime           TEXT NOT NULL DEFAULT 'ollama',
    ollama_host               TEXT NOT NULL DEFAULT 'http://127.0.0.1:11434',
    default_model             TEXT NOT NULL DEFAULT '',
    per_agent_overrides_json  TEXT NOT NULL DEFAULT '{}',
    caps_json                 TEXT NOT NULL DEFAULT '{"max_calls":8,"max_minutes":10,"max_tokens":200000,"max_retries":3}',
    updated_at                TEXT NOT NULL
);

-- ── Agent layer ──────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS agent_runs (
    id             TEXT PRIMARY KEY NOT NULL,
    workflow_id    TEXT NOT NULL,
    project_id     TEXT NOT NULL,
    status         TEXT NOT NULL CHECK (status IN (
                       'running','awaiting_user','completed',
                       'cancelled','error','invalid')),
    started_at     TEXT NOT NULL,
    completed_at   TEXT,
    total_tokens   INTEGER,
    error_message  TEXT,
    ollama_version TEXT,
    user_initiated INTEGER NOT NULL DEFAULT 1
);

CREATE INDEX IF NOT EXISTS idx_agent_runs_workflow ON agent_runs(workflow_id, started_at);
CREATE INDEX IF NOT EXISTS idx_agent_runs_project  ON agent_runs(project_id, started_at);

CREATE TABLE IF NOT EXISTS agent_tasks (
    id                   TEXT PRIMARY KEY NOT NULL,
    run_id               TEXT NOT NULL REFERENCES agent_runs(id) ON DELETE CASCADE,
    step_index           INTEGER NOT NULL,
    agent_id             TEXT NOT NULL,
    prompt_template_id   TEXT NOT NULL,
    prompt_template_hash TEXT NOT NULL,
    model                TEXT NOT NULL,
    model_digest         TEXT,
    input_hash           TEXT NOT NULL,
    output_hash          TEXT,
    context_tokens       INTEGER,
    output_tokens        INTEGER,
    duration_ms          INTEGER,
    retries              INTEGER NOT NULL DEFAULT 0,
    status               TEXT NOT NULL CHECK (status IN (
                             'running','completed','invalid','cancelled','error')),
    error_category       TEXT,
    error_message        TEXT,
    created_at           TEXT NOT NULL,
    updated_at           TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_agent_tasks_run   ON agent_tasks(run_id, step_index);
CREATE INDEX IF NOT EXISTS idx_agent_tasks_agent ON agent_tasks(agent_id, created_at);

CREATE TABLE IF NOT EXISTS agent_outputs (
    task_id        TEXT PRIMARY KEY NOT NULL REFERENCES agent_tasks(id) ON DELETE CASCADE,
    schema_id      TEXT NOT NULL,
    schema_version INTEGER NOT NULL,
    content_inline TEXT,
    content_path   TEXT,
    hash           TEXT NOT NULL,
    validated_at   TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS agent_applied_edits (
    id                    TEXT PRIMARY KEY NOT NULL,
    task_id               TEXT NOT NULL REFERENCES agent_tasks(id) ON DELETE RESTRICT,
    node_id               TEXT NOT NULL REFERENCES nodes(id) ON DELETE RESTRICT,
    pre_edit_snapshot_id  TEXT NOT NULL REFERENCES snapshots(id) ON DELETE RESTRICT,
    applied_at            TEXT NOT NULL,
    edit_kind             TEXT NOT NULL CHECK (edit_kind IN (
                              'text_replace','rename_entity','reorder','note_add')),
    edit_payload_json     TEXT NOT NULL,
    reverted_at           TEXT
);

CREATE INDEX IF NOT EXISTS idx_agent_edits_node ON agent_applied_edits(node_id, applied_at);

-- ── Memory entries ───────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS memory_entries (
    id          TEXT PRIMARY KEY NOT NULL,
    scope       TEXT NOT NULL CHECK (scope IN ('book','chapter','entity','style')),
    key         TEXT NOT NULL,
    value_json  TEXT NOT NULL,
    agent_id    TEXT NOT NULL,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL,
    UNIQUE (scope, key)
);

CREATE INDEX IF NOT EXISTS idx_memory_scope_key ON memory_entries(scope, key);

-- ── Export history ───────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS exports (
    id          TEXT PRIMARY KEY NOT NULL,
    profile     TEXT NOT NULL CHECK (profile IN (
                    'kdp_ebook','generic_epub',
                    'trade_pdf_5x8','trade_pdf_6x9','docx')),
    output_path TEXT NOT NULL,
    hash        TEXT NOT NULL,
    created_at  TEXT NOT NULL
);

-- ── V1.0 tables (created empty, not written by MVP) ─────────────────────────
CREATE TABLE IF NOT EXISTS refs (
    id          TEXT PRIMARY KEY NOT NULL,
    kind        TEXT NOT NULL CHECK (kind IN ('footnote','endnote','citation','crossref')),
    node_id     TEXT REFERENCES nodes(id) ON DELETE CASCADE,
    body        TEXT NOT NULL,
    csl_key     TEXT,
    created_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS comments (
    id          TEXT PRIMARY KEY NOT NULL,
    node_id     TEXT NOT NULL REFERENCES nodes(id) ON DELETE CASCADE,
    range_from  INTEGER NOT NULL,
    range_to    INTEGER NOT NULL,
    parent_id   TEXT REFERENCES comments(id),
    author      TEXT NOT NULL,
    body        TEXT NOT NULL,
    resolved_at TEXT,
    created_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS tracked_changes (
    id          TEXT PRIMARY KEY NOT NULL,
    node_id     TEXT NOT NULL REFERENCES nodes(id) ON DELETE CASCADE,
    op          TEXT NOT NULL CHECK (op IN ('insert','delete','format')),
    range_from  INTEGER NOT NULL,
    range_to    INTEGER NOT NULL,
    payload_json TEXT NOT NULL,
    author      TEXT NOT NULL,
    state       TEXT NOT NULL DEFAULT 'pending'
                    CHECK (state IN ('pending','accepted','rejected')),
    created_at  TEXT NOT NULL
);
