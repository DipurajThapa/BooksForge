-- Initial schema: nodes, scene_content, entities, style_book, memory_entries.
--
-- All IDs are stored as TEXT (ULID string representation).
-- Timestamps are ISO-8601 TEXT in UTC.
-- JSON blobs are stored as TEXT with CHECK constraints.

-- ── nodes ────────────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS nodes (
    id          TEXT PRIMARY KEY NOT NULL,
    parent_id   TEXT REFERENCES nodes(id) ON DELETE CASCADE,
    kind        TEXT NOT NULL CHECK(kind IN (
                    'project','part','chapter','scene',
                    'front_matter','back_matter')),
    title       TEXT NOT NULL DEFAULT '',
    position    TEXT NOT NULL DEFAULT '0|hzzzzz:',   -- LexoRank string
    status      TEXT NOT NULL DEFAULT 'planned'
                    CHECK(status IN ('planned','drafting','revised','final')),
    pov         TEXT,
    beat        TEXT,
    target_words INTEGER,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL,
    deleted_at  TEXT
);

CREATE INDEX IF NOT EXISTS idx_nodes_parent ON nodes(parent_id);
CREATE INDEX IF NOT EXISTS idx_nodes_status ON nodes(status);

-- ── scene_content ─────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS scene_content (
    node_id     TEXT PRIMARY KEY NOT NULL REFERENCES nodes(id) ON DELETE CASCADE,
    pm_doc      TEXT NOT NULL DEFAULT '{}',  -- ProseMirror JSON
    word_count  INTEGER NOT NULL DEFAULT 0,
    char_count  INTEGER NOT NULL DEFAULT 0,
    updated_at  TEXT NOT NULL
);

-- ── entities ─────────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS entities (
    id           TEXT PRIMARY KEY NOT NULL,
    kind         TEXT NOT NULL CHECK(kind IN (
                     'character','location','organization',
                     'object','concept','event')),
    canonical    TEXT NOT NULL,
    aliases      TEXT NOT NULL DEFAULT '[]',  -- JSON array
    attributes   TEXT NOT NULL DEFAULT '{}',  -- JSON object
    notes        TEXT NOT NULL DEFAULT '',
    created_at   TEXT NOT NULL,
    updated_at   TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_entities_kind ON entities(kind);

-- ── style_book ────────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS style_book (
    id              INTEGER PRIMARY KEY CHECK(id = 1),  -- singleton row
    em_dash         TEXT NOT NULL DEFAULT 'spaced_en',
    quote_style     TEXT NOT NULL DEFAULT 'curly',
    ellipsis_form   TEXT NOT NULL DEFAULT 'spaced',
    serial_comma    INTEGER NOT NULL DEFAULT 1,  -- boolean
    pov_default     TEXT NOT NULL DEFAULT 'third_limited',
    tense_default   TEXT NOT NULL DEFAULT 'past',
    profanity_level TEXT NOT NULL DEFAULT 'none',
    reading_age     INTEGER NOT NULL DEFAULT 16,
    custom_rules    TEXT NOT NULL DEFAULT '[]',  -- JSON array of strings
    updated_at      TEXT NOT NULL
);

-- ── memory_entries ────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS memory_entries (
    id          TEXT PRIMARY KEY NOT NULL,
    scope       TEXT NOT NULL CHECK(scope IN ('book','chapter','entity','style')),
    key         TEXT NOT NULL,
    value_json  TEXT NOT NULL,
    agent_id    TEXT NOT NULL,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL,
    UNIQUE(scope, key)
);

CREATE INDEX IF NOT EXISTS idx_memory_scope_key ON memory_entries(scope, key);
