-- Migration v4 — extend `exports.profile` CHECK to allow `markdown`.
--
-- M0 ships the simplest export profile (Markdown) so writers can take their
-- manuscript out of BooksForge before the full Pandoc / EPUB-3 pipeline
-- lands in M5.  SQLite cannot ALTER a CHECK in place, so we rebuild the
-- table preserving any existing rows.

PRAGMA foreign_keys = OFF;

CREATE TABLE IF NOT EXISTS exports_old AS SELECT * FROM exports;

DROP TABLE exports;

CREATE TABLE exports (
    id          TEXT PRIMARY KEY NOT NULL,
    profile     TEXT NOT NULL CHECK (profile IN (
                    'markdown','docx','generic_epub','kdp_ebook',
                    'trade_pdf_5x8','trade_pdf_6x9')),
    output_path TEXT NOT NULL,
    hash        TEXT NOT NULL,
    created_at  TEXT NOT NULL
);

INSERT INTO exports SELECT * FROM exports_old;
DROP TABLE exports_old;

CREATE INDEX IF NOT EXISTS idx_exports_created ON exports(created_at);

PRAGMA foreign_keys = ON;
