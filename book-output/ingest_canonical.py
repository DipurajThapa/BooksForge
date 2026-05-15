#!/usr/bin/env python3
"""
ingest_canonical.py — split the Rust example's `manuscript.md` into
per-chapter files under `<book-dir>/canonical/chapters/` so the
composer + final_review can consume them.

The Rust example writes one combined manuscript with H1 chapter
boundaries and H2 scene headings (per the writer code at lines
597-610 of multi_chapter_run.rs):

    # Chapter N
    ## <scene-title>
    <prose>
    ...

This script:
  - finds the most-recent run dir under book-output/multi-chapter-runs/
  - reads its manuscript.md
  - fixes UTF-8 double-encoding mojibake (qwen3.6:latest emits these
    intermittently — see `fix_mojibake` below)
  - splits on `# Chapter N` boundaries
  - writes each chapter to <book-dir>/canonical/chapters/chapter-NN.md

Usage:
    python ingest_canonical.py [book-dir] [--run-dir <path>]
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from booksforge_ollama_driver import REPO  # type: ignore


CHAPTER_HEADER_RE = re.compile(r"^#\s+Chapter\s+(\d+)\s*$", re.IGNORECASE)
SCENE_TITLE_RE = re.compile(r"^##\s+Ch\d+\s+S\d+\s*[—–\-]\s*(.+?)\s*$", re.IGNORECASE)


# ── Mojibake fix ─────────────────────────────────────────────────────────
#
# qwen3.6:latest occasionally emits double-encoded UTF-8 in long-form
# prose: a real UTF-8 byte sequence (e.g. `\xe2\x80\x99` for `’`) gets
# re-encoded as if each byte were a Latin-1 codepoint, producing
# `\xc3\xa2\xc2\x80\xc2\x99` on the wire. The Rust pipeline preserves
# these bytes faithfully — it's not the Rust layer's job to second-
# guess model output — so the manuscript on disk contains a mix of
# clean smart-quotes and mojibake.
#
# This byte-level fix targets the exact double-encoded sequences. Each
# pattern is ≥6 bytes long and is not a valid character sequence in
# any natural language, so the replacement can never corrupt legitimate
# prose. Source: byte-traced from the canonical run output 2026-05-15.
_MOJIBAKE_BYTES: list[tuple[bytes, str]] = [
    # Smart quotes (U+2018-U+201E)
    (b"\xc3\xa2\xc2\x80\xc2\x99", "’"),  # right single quote
    (b"\xc3\xa2\xc2\x80\xc2\x98", "‘"),  # left single quote
    (b"\xc3\xa2\xc2\x80\xc2\x9c", "“"),  # left double quote
    (b"\xc3\xa2\xc2\x80\xc2\x9d", "”"),  # right double quote
    (b"\xc3\xa2\xc2\x80\xc2\x9e", "„"),  # low double quote
    (b"\xc3\xa2\xc2\x80\xc2\x9a", "‚"),  # low single quote
    # Dashes
    (b"\xc3\xa2\xc2\x80\xc2\x94", "—"),  # em dash —
    (b"\xc3\xa2\xc2\x80\xc2\x93", "–"),  # en dash –
    # Ellipsis and bullet
    (b"\xc3\xa2\xc2\x80\xc2\xa6", "…"),  # …
    (b"\xc3\xa2\xc2\x80\xc2\xa2", "•"),  # •
    # Accented Latin (in case the model writes European names)
    (b"\xc3\x83\xc2\xa9", "é"),  # é
    (b"\xc3\x83\xc2\xa8", "è"),  # è
    (b"\xc3\x83\xc2\xb1", "ñ"),  # ñ
    (b"\xc3\x83\xc2\xa1", "á"),  # á
    (b"\xc3\x83\xc2\xb3", "ó"),  # ó
    (b"\xc3\x83\xc2\xad", "í"),  # í
    (b"\xc3\x83\xc2\xba", "ú"),  # ú
    # Non-breaking space mojibake
    (b"\xc3\x82\xc2\xa0", " "),  # NBSP
]


def fix_mojibake(text: str) -> tuple[str, int]:
    """Return (cleaned_text, replacement_count). Byte-level fix —
    targets the exact UTF-8 byte sequences qwen3.6 emits when it
    double-encodes Unicode. Each pattern is ≥4 bytes and unambiguous.
    """
    data = text.encode("utf-8")
    count = 0
    for bad, good in _MOJIBAKE_BYTES:
        if bad in data:
            n = data.count(bad)
            data = data.replace(bad, good.encode("utf-8"))
            count += n
    return data.decode("utf-8"), count


# ── Sentence-spacing fix ─────────────────────────────────────────────────
#
# qwen3.6 emits prose with missing spaces between sentences in ~10% of
# scenes — "looking.A rickshaw", "deafening.He stood", "weak.The line".
# The polish stack doesn't catch these because they look like prose
# defects, not AI-tells. They make the rendered book look broken.
#
# Fix: insert a space between sentence-ending punctuation and the
# capital letter that starts the next sentence. Constraints to avoid
# false positives:
#   - require 2+ lowercase letters before the punctuation
#     (skips abbreviations like "Dr.", "Mr.", initials "U.S.")
#   - require 2+ letters after the capital
#     (skips initials like "J.")
#   - leave URLs (with `://`) and decimal numbers alone
_SENTENCE_GLUE_RE = re.compile(
    r"([a-z]{2,})([.!?])([A-Z])"
)


def fix_sentence_spacing(text: str) -> tuple[str, int]:
    """Insert a single space between a sentence-end punctuation and
    the next sentence's leading capital letter. Returns (fixed,
    replacement_count). Idempotent — a second run is a no-op.
    """
    count = 0

    def _replace(m: re.Match) -> str:
        nonlocal count
        count += 1
        return f"{m.group(1)}{m.group(2)} {m.group(3)}"

    fixed = _SENTENCE_GLUE_RE.sub(_replace, text)
    return fixed, count


def find_latest_run_dir() -> Path:
    base = REPO / "book-output" / "multi-chapter-runs"
    if not base.exists():
        raise FileNotFoundError(f"no runs at {base}")
    candidates = sorted(d for d in base.iterdir() if d.is_dir())
    if not candidates:
        raise FileNotFoundError(f"no run directories under {base}")
    return candidates[-1]


def split_manuscript(text: str) -> dict[int, str]:
    """Return {chapter_number: chapter_markdown}. Each chapter's
    markdown starts with the `## Chapter N — <title>` heading
    expected by compose_book.py (we synthesise it).
    """
    lines = text.splitlines()
    by_chapter: dict[int, list[str]] = {}
    current: int | None = None
    for line in lines:
        m = CHAPTER_HEADER_RE.match(line.strip())
        if m:
            current = int(m.group(1))
            by_chapter[current] = []
            continue
        if current is not None:
            by_chapter[current].append(line)

    out: dict[int, str] = {}
    for n, body_lines in by_chapter.items():
        # Extract the chapter's actual title from the first scene's H2.
        # The Rust example formats scene H2s as "Ch{N} S{M} — <title>",
        # so the title after the em-dash is the chapter name. Without
        # this, compose_book.py's chapter_block falls back to just
        # "Chapter N" which duplicates the "CHAPTER N" eyebrow.
        chapter_title = ""
        for ln in body_lines:
            m = SCENE_TITLE_RE.match(ln.strip())
            if m:
                chapter_title = m.group(1).strip()
                break

        body = "\n".join(body_lines).strip()
        if chapter_title:
            ch_heading = f"## Chapter {n} — {chapter_title}"
        else:
            ch_heading = f"## Chapter {n}"
        out[n] = f"{ch_heading}\n\n{body}\n"
    return out


def main() -> int:
    p = argparse.ArgumentParser()
    p.add_argument("book_dir", nargs="?", default="my-confused-life")
    p.add_argument("--run-dir", help="path to the multi-chapter-runs/<timestamp>/ directory")
    args = p.parse_args()

    book_dir = Path(args.book_dir)
    if not book_dir.is_absolute():
        book_dir = Path(__file__).resolve().parent / book_dir
    if not book_dir.exists():
        print(f"FATAL: book dir {book_dir} does not exist", file=sys.stderr)
        return 2

    run_dir = Path(args.run_dir) if args.run_dir else find_latest_run_dir()
    manuscript = run_dir / "manuscript.md"
    if not manuscript.exists():
        print(f"FATAL: {manuscript} missing", file=sys.stderr)
        return 2

    text = manuscript.read_text(encoding="utf-8")
    text, mojibake_count = fix_mojibake(text)
    if mojibake_count > 0:
        print(f"  fixed {mojibake_count} mojibake sequence(s) in manuscript")
    text, spacing_count = fix_sentence_spacing(text)
    if spacing_count > 0:
        print(f"  fixed {spacing_count} missing-space defect(s) between sentences")

    chapters = split_manuscript(text)
    if not chapters:
        print(f"FATAL: no `# Chapter N` headings found in {manuscript}", file=sys.stderr)
        return 3

    target_dir = book_dir / "canonical" / "chapters"
    target_dir.mkdir(parents=True, exist_ok=True)
    for n, md in sorted(chapters.items()):
        out = target_dir / f"chapter-{n:02d}.md"
        out.write_text(md, encoding="utf-8")
        word_count = len(md.split())
        print(f"  wrote {out.relative_to(book_dir)} ({word_count} words)")

    sc = run_dir / "score_card.json"
    if sc.exists():
        (book_dir / "canonical" / "score_card.json").write_text(sc.read_text(encoding="utf-8"), encoding="utf-8")
        print(f"  copied score_card.json")

    print(f"\nIngested {len(chapters)} chapters from {run_dir.name}")
    print(f"Next: python final_review.py {args.book_dir}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
