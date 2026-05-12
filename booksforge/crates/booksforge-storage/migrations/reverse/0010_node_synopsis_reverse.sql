-- Reverse of migration 0010 (node synopsis column).
--
-- SQLite supports `ALTER TABLE ... DROP COLUMN` since 3.35 (2021).
-- The Rust toolchain pins SQLite via bundled `libsqlite3-sys`, which
-- has been >= 3.35 since 2022. Safe to use here.
--
-- NOT run automatically — used for testing and disaster recovery only.

ALTER TABLE nodes DROP COLUMN synopsis;
