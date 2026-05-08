-- Migration v3 — `ai_calls` ledger for MZ-08 quick-action presets.
--
-- Quick actions (Sharpen / Continue / Rephrase) are inline editor helpers
-- that emit raw prose, not validated JSON.  They are explicitly distinct
-- from agents, so they get their own ledger rather than sharing
-- `agent_runs / agent_tasks / agent_outputs`.
--
-- Apply tracking (snapshot id + applied_at) is inlined here rather than
-- using `agent_applied_edits` because there is no `agent_tasks` parent
-- row to satisfy that table's NOT NULL FK.

CREATE TABLE IF NOT EXISTS ai_calls (
    id                    TEXT PRIMARY KEY NOT NULL,
    node_id               TEXT NOT NULL REFERENCES nodes(id) ON DELETE CASCADE,
    preset                TEXT NOT NULL CHECK (preset IN ('sharpen','continue','rephrase')),
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

    -- Apply tracking: NULL until the user clicks Accept in the diff panel.
    pre_edit_snapshot_id  TEXT REFERENCES snapshots(id) ON DELETE SET NULL,
    applied_at            TEXT
);

CREATE INDEX IF NOT EXISTS idx_ai_calls_node    ON ai_calls(node_id, created_at);
CREATE INDEX IF NOT EXISTS idx_ai_calls_preset  ON ai_calls(preset, created_at);
CREATE INDEX IF NOT EXISTS idx_ai_calls_applied ON ai_calls(applied_at) WHERE applied_at IS NOT NULL;
