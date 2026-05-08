-- Migration v8 — extend `snapshots.trigger` CHECK to allow `pre_restore`
-- (Turn B — A1 from the audit).
--
-- Distinct trigger value so the SnapshotsPanel timeline can filter
-- user-initiated `manual` snapshots from automatic safety captures.
-- SQLite cannot ALTER a CHECK in place — rebuild the table.

PRAGMA foreign_keys = OFF;

CREATE TABLE IF NOT EXISTS snapshots_old AS SELECT * FROM snapshots;
DROP TABLE snapshots;

CREATE TABLE snapshots (
    id          TEXT PRIMARY KEY NOT NULL,
    scope       TEXT NOT NULL CHECK (scope IN ('project','part','chapter','scene')),
    scope_id    TEXT,
    label       TEXT,
    trigger     TEXT NOT NULL CHECK (trigger IN (
                    'manual','auto','pre_ai','pre_export','pre_migration',
                    'pre_agent_edit','pre_restore','crash_recovery')),
    tree_hash   TEXT NOT NULL,
    created_at  TEXT NOT NULL,
    size_bytes  INTEGER NOT NULL DEFAULT 0
);

INSERT INTO snapshots SELECT * FROM snapshots_old;
DROP TABLE snapshots_old;

CREATE INDEX IF NOT EXISTS idx_snapshots_scope ON snapshots(scope, scope_id, created_at);
CREATE INDEX IF NOT EXISTS idx_snapshots_trigger ON snapshots(trigger, created_at);

PRAGMA foreign_keys = ON;
