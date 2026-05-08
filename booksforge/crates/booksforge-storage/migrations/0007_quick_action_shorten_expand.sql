-- Migration v7 — extend `ai_calls.preset` CHECK to allow `shorten` and
-- `expand` (Turn A — full quick-action preset set per MVP_SCOPE §2.5).
--
-- SQLite cannot ALTER a CHECK in place, so we rebuild the table preserving
-- existing rows (same approach as migration 0005).

PRAGMA foreign_keys = OFF;

CREATE TABLE IF NOT EXISTS ai_calls_old AS SELECT * FROM ai_calls;
DROP TABLE ai_calls;

CREATE TABLE ai_calls (
    id                    TEXT PRIMARY KEY NOT NULL,
    node_id               TEXT NOT NULL REFERENCES nodes(id) ON DELETE CASCADE,
    preset                TEXT NOT NULL CHECK (preset IN (
                              'sharpen','continue','rephrase','final_polish',
                              'shorten','expand')),
    model                 TEXT NOT NULL,
    prompt_template_id    TEXT NOT NULL,
    prompt_template_hash  TEXT NOT NULL,
    scope_text_len        INTEGER NOT NULL,
    output_text           TEXT,
    context_tokens        INTEGER,
    output_tokens         INTEGER,
    duration_ms           INTEGER,
    status                TEXT NOT NULL CHECK (status IN ('ok','cancelled','error')),
    error_message         TEXT,
    created_at            TEXT NOT NULL,
    pre_edit_snapshot_id  TEXT REFERENCES snapshots(id) ON DELETE SET NULL,
    applied_at            TEXT
);

INSERT INTO ai_calls SELECT * FROM ai_calls_old;
DROP TABLE ai_calls_old;

CREATE INDEX IF NOT EXISTS idx_ai_calls_node    ON ai_calls(node_id, created_at);
CREATE INDEX IF NOT EXISTS idx_ai_calls_preset  ON ai_calls(preset, created_at);
CREATE INDEX IF NOT EXISTS idx_ai_calls_applied ON ai_calls(applied_at) WHERE applied_at IS NOT NULL;

PRAGMA foreign_keys = ON;
