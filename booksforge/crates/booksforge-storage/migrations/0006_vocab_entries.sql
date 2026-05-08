-- Migration v6 — `vocab_entries` ledger for the layered vocabulary
-- subsystem (Phase 3 — VOCABULARY_DICTIONARIES.md).
--
-- One row per dictionary entry.  Layers are encoded as namespaced strings
-- (`"project"`, `"genre:fantasy"`, `"audience:adult"`, `"ai_tells"`, etc.)
-- so the lookup logic can sort by length-then-string for a deterministic
-- "most-specific layer wins" precedence without needing a separate
-- `layers` table.

CREATE TABLE IF NOT EXISTS vocab_entries (
    id            TEXT PRIMARY KEY NOT NULL,
    layer         TEXT NOT NULL,
    --`term` is stored lowercase for case-insensitive matching; the UI
    --shows the original casing via `display_term`.
    term          TEXT NOT NULL,
    display_term  TEXT NOT NULL,
    kind          TEXT NOT NULL CHECK (kind IN ('prefer','avoid','replace')),
    replacement   TEXT,
    rationale     TEXT,
    --Curated source: 'starter' (shipped baseline), 'user' (manual add),
    --'agent' (Vocabulary Dictionary Agent learned from accepted edits).
    source        TEXT NOT NULL DEFAULT 'user'
                      CHECK (source IN ('starter','user','agent')),
    created_at    TEXT NOT NULL,
    updated_at    TEXT NOT NULL,
    UNIQUE (layer, term, kind)
);

CREATE INDEX IF NOT EXISTS idx_vocab_layer ON vocab_entries(layer);
CREATE INDEX IF NOT EXISTS idx_vocab_term  ON vocab_entries(term);
CREATE INDEX IF NOT EXISTS idx_vocab_kind  ON vocab_entries(kind);
