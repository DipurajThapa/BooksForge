-- Reverse of migration v1.
-- Drops all tables in reverse dependency order.
-- NOT run automatically — used for testing and disaster recovery only.

DROP TABLE IF EXISTS tracked_changes;
DROP TABLE IF EXISTS comments;
DROP TABLE IF EXISTS refs;
DROP TABLE IF EXISTS exports;
DROP TABLE IF EXISTS memory_entries;
DROP TABLE IF EXISTS agent_applied_edits;
DROP TABLE IF EXISTS agent_outputs;
DROP TABLE IF EXISTS agent_tasks;
DROP TABLE IF EXISTS agent_runs;
DROP TABLE IF EXISTS model_settings;
DROP TABLE IF EXISTS style_book;
DROP TABLE IF EXISTS validator_issues;
DROP TABLE IF EXISTS validator_runs;
DROP TABLE IF EXISTS snapshots;
DROP TABLE IF EXISTS entity_scene_appearances;
DROP TABLE IF EXISTS entity_aliases;
DROP TABLE IF EXISTS entities;
DROP TABLE IF EXISTS notes;
DROP TABLE IF EXISTS scene_content;
DROP TABLE IF EXISTS nodes;
DROP TABLE IF EXISTS schema_migrations;
