-- Migration v2: extend `agent_applied_edits.edit_kind` CHECK to allow
-- `tree_create` — the audit ledger value emitted by MZ-07's
-- `apply_outline` flow when the Outline Architect's proposal is accepted
-- and the document tree is materialised.
--
-- SQLite cannot ALTER CHECK constraints in place, so we rebuild the table.

-- Disable FK checks during the rebuild (re-enabled when the connection is
-- recreated by the pool; safe within a single migration run).
PRAGMA foreign_keys = OFF;

-- Stash existing rows (if any).
CREATE TABLE IF NOT EXISTS agent_applied_edits_old AS SELECT * FROM agent_applied_edits;

DROP TABLE agent_applied_edits;

CREATE TABLE agent_applied_edits (
    id                    TEXT PRIMARY KEY NOT NULL,
    task_id               TEXT NOT NULL REFERENCES agent_tasks(id) ON DELETE RESTRICT,
    node_id               TEXT NOT NULL REFERENCES nodes(id) ON DELETE RESTRICT,
    pre_edit_snapshot_id  TEXT NOT NULL REFERENCES snapshots(id) ON DELETE RESTRICT,
    applied_at            TEXT NOT NULL,
    edit_kind             TEXT NOT NULL CHECK (edit_kind IN (
                              'text_replace','rename_entity','reorder','note_add','tree_create')),
    edit_payload_json     TEXT NOT NULL,
    reverted_at           TEXT
);

INSERT INTO agent_applied_edits SELECT * FROM agent_applied_edits_old;
DROP TABLE agent_applied_edits_old;

CREATE INDEX IF NOT EXISTS idx_agent_edits_node ON agent_applied_edits(node_id, applied_at);
CREATE INDEX IF NOT EXISTS idx_agent_edits_task ON agent_applied_edits(task_id);

PRAGMA foreign_keys = ON;
