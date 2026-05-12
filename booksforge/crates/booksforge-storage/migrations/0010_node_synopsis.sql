-- 0010_node_synopsis.sql
--
-- Adds a `synopsis` column to the `nodes` table so the OutlineView
-- sidebar (UI/UX spec §5.1) can store a short writer-facing
-- description per scene without going through scene `pm_doc`.
--
-- Forward-only. The column is nullable with no default; existing
-- rows get NULL automatically and existing `SELECT id, parent_id,
-- kind, title, position, status, pov, beat, target_words,
-- created_at, updated_at, deleted_at FROM nodes` queries continue
-- to work (they just don't read the new column). New queries that
-- want synopsis must add it to their SELECT list.
--
-- Reverse migration lives in `migrations/reverse/0010_*.sql`.

ALTER TABLE nodes ADD COLUMN synopsis TEXT NULL;
