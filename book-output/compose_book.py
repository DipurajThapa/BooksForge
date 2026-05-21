#!/usr/bin/env python3
"""
compose_book.py — assemble a publication-ready fiction interior.

Inputs:  a `book-output/<dir>/` directory produced by booksforge_fiction_pipeline.py
         (must contain `01-brief.json`, `02-outline.json`, `chapters/chapter-NN.md`)

Output:  the same directory's `FULL_MANUSCRIPT.md` is rewritten with:

  Front matter
    1. Half-title (title only)
    2. Title page (title + author + publisher)
    3. Copyright page (year, ISBN placeholder, edition, fiction disclaimer)
    4. Dedication
    5. Epigraph
  Body
    6. Chapter N — Title  (one per chapter; rendered with drop cap + page break)
  Back matter
    7. The End ornament
    8. Acknowledgements
    9. About the Author
   10. Colophon
   11. Back-cover blurb (acts as final spread)

Each section is wrapped in a `<div class="...">` block so the print CSS
in `fiction_print.css` can target it. Pandoc passes raw HTML through
unchanged in GFM mode.

Placeholders (author name, ISBN, publisher) are clearly bracketed —
this is BooksForge's validation manuscript, not a real publication.
"""

from __future__ import annotations

import datetime as dt
import json
import re
import sys
from pathlib import Path


# ── Placeholder publication metadata ──────────────────────────────────────
# Bracketed values mark fields a real author/publisher would fill in.
# Don't substitute real names or numbers here — the validation manuscript
# should never accidentally claim to be a published work.
PUBLICATION = {
    "author": "[Author Name]",
    "publisher": "BooksForge Press",
    "publisher_city": "[City]",
    "year": dt.datetime.now().year,
    "edition": "First Edition",
    "isbn_print": "978-0-00-000000-0",
    "isbn_ebook": "978-0-00-000000-7",
    "rights_holder": "[Author Name]",
}


def half_title(title: str) -> str:
    # Use raw HTML (not markdown `# title`) so pandoc does NOT generate
    # an H1-derived `<section>` here. Pandoc's EPUB splitter cuts at H1
    # boundaries and was leaving the wrapping `<div>` unclosed across
    # split files, producing invalid XHTML like:
    #   <div class="frontmatter half-title">
    #   </section>
    # …which EPUB readers report as a tag-mismatch error.
    return f"""\
<div class="frontmatter half-title">
<p class="half-title-text">{title}</p>
</div>
"""


def title_page(title: str, author: str, publisher: str) -> str:
    # Same reasoning as half_title — no markdown `# heading`. The
    # `.book-title` is rendered via CSS, not via H1 semantics.
    return f"""\
<div class="frontmatter title-page">
<p class="book-title">{title}</p>
<p class="byline">{author}</p>
<p class="publisher-mark">{publisher}</p>
</div>
"""


def copyright_page(title: str, pub: dict) -> str:
    return f"""\
<div class="frontmatter copyright-page">

<p class="copyright-title">{title}</p>

<p>Copyright © {pub['year']} by {pub['rights_holder']}</p>

<p>All rights reserved.</p>

<p>No part of this book may be reproduced, scanned, or distributed in any printed
or electronic form without permission. Please do not participate in or encourage
piracy of copyrighted materials in violation of the author's rights. Purchase
only authorised editions.</p>

<p class="disclaimer"><em>This is a work of fiction. Names, characters, places,
and incidents are products of the author's imagination or used fictitiously. Any
resemblance to actual persons, living or dead, businesses, events, or locales
is entirely coincidental.</em></p>

<p>Published by {pub['publisher']}<br/>
{pub['publisher_city']}</p>

<p>{pub['edition']}</p>

<p>ISBN (print): {pub['isbn_print']}<br/>
ISBN (ebook): {pub['isbn_ebook']}</p>

<p class="bf-mark">Produced with BooksForge — a local-first writing toolchain.</p>

</div>
"""


def dedication_page() -> str:
    return """\
<div class="frontmatter dedication">

For everyone who is still finding their way home.

</div>
"""


def epigraph_page() -> str:
    return """\
<div class="frontmatter epigraph">

<blockquote class="epigraph-quote">
<p>"We are not human beings having a spiritual experience.<br/>
We are spiritual beings having a human experience."</p>
<p class="attribution">— attributed</p>
</blockquote>

</div>
"""


def chapter_block(num: int, title: str, body_md: str) -> str:
    """Wrap a chapter so the print CSS can find its number, title, and body.

    Body markdown is left as-is; the print CSS targets the first paragraph
    via `.chapter-body > p:first-of-type` for drop cap + no-indent.

    The leading `<div style="page-break-before: always"></div>` is the
    portable DOCX page-break trick — pandoc's OOXML writer translates
    `page-break-before: always` to a `<w:br w:type="page"/>` element so
    each chapter starts on a fresh page in Word. HTML/PDF already get
    page breaks via the CSS `.chapter` rule; this raw div is silent for
    them. EPUB readers ignore the inline style.
    """
    # Strip "Chapter N — " prefix from the title if present — the
    # composer adds its own numbered chapter-eyebrow above the title.
    clean_title = re.sub(
        r"^\s*chapter\s+\d+\s*[—–\-:]\s*",
        "",
        title.strip(),
        flags=re.IGNORECASE,
    ).strip()
    return f"""\
<div style="page-break-before: always"></div>

<div class="chapter" id="ch{num:02d}">

<p class="chapter-eyebrow">Chapter {num}</p>

<h2 class="chapter-title">{clean_title}</h2>

<div class="chapter-body">

{body_md.strip()}

</div>

</div>
"""


def the_end_ornament() -> str:
    return """\
<div class="backmatter the-end">

<p class="ornament">❦</p>

</div>
"""


def acknowledgements_page() -> str:
    return """\
<div class="backmatter acknowledgements">

<h2>Acknowledgements</h2>

<p>This book exists because of the people who taught me — by example more than by
instruction — that growth is not a project you finish, but a posture you
practise. To my family, who waited while I learned what I should already have
known. To the friends who answered the phone late at night without asking why.
To the strangers in temples and teashops whose names I never learned and whose
patience I never deserved.</p>

<p>And to the reader who has stayed this far: thank you. The story was never
mine alone.</p>

</div>
"""


def about_the_author(author: str) -> str:
    return f"""\
<div class="backmatter about-the-author">

<h2>About the Author</h2>

<p>{author} is a writer working in literary and devotional fiction. This is
[his/her/their] first novel. [Author bio placeholder — fill in: where the
author lives, what they do when they aren't writing, where readers can find
their work online.]</p>

</div>
"""


def colophon(pub: dict) -> str:
    return f"""\
<div class="backmatter colophon">

<h2>Colophon</h2>

<p>The text of this book is set in EB Garamond, a digital revival of the
sixteenth-century roman of Claude Garamont. Display elements are set in the
same family.</p>

<p>Manuscript drafted on qwen3.5:9b and polished on qwen3.5:27b, both run
locally via Ollama. Interior typesetting rendered by BooksForge's Pandoc and
print pipeline. No part of this manuscript was sent to a remote endpoint.</p>

<p>{pub['edition']}, {pub['year']}.</p>

</div>
"""


def back_cover_blurb(title: str, brief: dict, pub: dict) -> str:
    premise = brief.get("premise", "")
    # Tighten the premise to two paragraphs for a back-cover read.
    return f"""\
<div class="backmatter back-cover">

<h2 class="back-cover-title">{title}</h2>

<p class="back-cover-blurb">{premise}</p>

<p class="back-cover-credit">{pub['author']}<br/>
{pub['publisher']}</p>

<p class="back-cover-isbn">ISBN {pub['isbn_print']}</p>

</div>
"""


# ── Chapter-file parser ────────────────────────────────────────────────────


HEADING_RE = re.compile(r"^##\s+(.*?)\s*$")


def parse_chapter_file(path: Path) -> tuple[str, str]:
    """Read a chapter-NN.md file and return (title, body).

    Body excludes the leading `## …` chapter heading. Within-chapter
    scene-title H2 lines (`## Ch1 S1 — …`) are stripped and replaced
    with centred scene-break ornaments (`* * *`) — both because that's
    the publication convention for fiction interiors, and because
    pandoc auto-wraps each remaining H2 in `<section>` blocks, which
    cross my chapter `<div>` boundary and produce invalid XHTML.

    The first scene gets no ornament (it starts the chapter); each
    subsequent scene is preceded by a centred * * * break.
    """
    lines = path.read_text(encoding="utf-8").splitlines()
    title = ""
    body_start = 0
    for i, line in enumerate(lines):
        m = HEADING_RE.match(line)
        if m:
            title = m.group(1)
            body_start = i + 1
            if body_start < len(lines) and lines[body_start].strip() == "":
                body_start += 1
            break

    body_lines = lines[body_start:]
    cleaned: list[str] = []
    scene_seen = False
    for ln in body_lines:
        if HEADING_RE.match(ln) and not ln.startswith("###"):
            # Scene-title H2 — replace with scene-break ornament. The
            # very first scene of the chapter gets no preceding break.
            if scene_seen:
                cleaned.append("")
                cleaned.append('<p class="scene-break">* * *</p>')
                cleaned.append("")
            scene_seen = True
            # Skip the H2 line itself and any blank line right after.
            continue
        cleaned.append(ln)

    body = "\n".join(cleaned).strip()
    return title, body


# ── Main ──────────────────────────────────────────────────────────────────


def main() -> int:
    args = sys.argv[1:]
    if not args:
        print("usage: compose_book.py <book-output-dir>", file=sys.stderr)
        return 1
    root = Path(args[0])
    if not root.is_absolute():
        # Resolve relative to this script's directory so the script can be
        # invoked from anywhere.
        root = Path(__file__).resolve().parent / root

    brief_path = root / "01-brief.json"
    chapters_dir = root / "chapters"
    if not brief_path.exists() or not chapters_dir.is_dir():
        print(f"missing {brief_path} or {chapters_dir}", file=sys.stderr)
        return 2

    brief = json.loads(brief_path.read_text())
    title = (brief.get("title_suggestions") or ["[Title]"])[0]

    parts: list[str] = []

    # ── Front matter ─────────────────────────────────────────────────────
    parts.append(half_title(title))
    parts.append(title_page(title, PUBLICATION["author"], PUBLICATION["publisher"]))
    parts.append(copyright_page(title, PUBLICATION))
    parts.append(dedication_page())
    parts.append(epigraph_page())

    # ── Body ─────────────────────────────────────────────────────────────
    # Source-of-truth priority (highest first):
    #   1. canonical/chapters/         — book_canonical.py output
    #                                    (canonical ≥95% pipeline)
    #   2. chapters-v2/                — targeted_redraft.py output
    #   3. chapters/                   — original fiction pipeline output
    canonical_dir = root / "canonical" / "chapters"
    chapters_v2_dir = root / "chapters-v2"
    chapter_paths = sorted(chapters_dir.glob("chapter-*.md"))
    for i, ch_path in enumerate(chapter_paths, start=1):
        canonical_path = canonical_dir / ch_path.name
        v2_path = chapters_v2_dir / ch_path.name
        if canonical_path.exists():
            active = canonical_path
        elif v2_path.exists():
            active = v2_path
        else:
            active = ch_path
        ch_title, ch_body = parse_chapter_file(active)
        parts.append(chapter_block(i, ch_title, ch_body))

    # ── Back matter ──────────────────────────────────────────────────────
    parts.append(the_end_ornament())
    parts.append(acknowledgements_page())
    parts.append(about_the_author(PUBLICATION["author"]))
    parts.append(colophon(PUBLICATION))
    parts.append(back_cover_blurb(title, brief, PUBLICATION))

    full = "\n\n".join(parts).rstrip() + "\n"
    out_path = root / "FULL_MANUSCRIPT.md"
    out_path.write_text(full)

    word_count = len(re.sub(r"<[^>]+>", " ", full).split())
    print(f"composed: {out_path}")
    print(f"words:    ~{word_count} (incl. front/back matter)")
    print(f"chapters: {len(chapter_paths)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
