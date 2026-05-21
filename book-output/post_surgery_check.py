#!/usr/bin/env python3
"""
post_surgery_check.py — validate the rewritten manuscript before export.

Run after `scene_surgery.py`. Verifies:

  1. POV consistency  — every narrative scene uses first-person past tense
     (heuristic: "I" / "me" / "my" frequency dominates "he" / "his" /
     "Arjun" in narration).
  2. No comma-spliced collapse — the failure mode we saw in the original
     bloated scenes ("walked to elevator, pressed button, doors opened,
     stepped in"). Scans for sequences of ≥4 short verb-led clauses
     separated only by commas inside a single paragraph.
  3. Length compliance — each scene within 70–130% of its outline target.
  4. No model commentary — output should be prose only, not "Here is the
     rewritten scene:" or "Note: I have…" framing.
  5. No leftover Latin-1 mojibake or missing inter-sentence spaces (we
     run the same fixers ingest_canonical.py uses, just on the rewritten
     text).

Exit code is non-zero if any check fails — gates the export step in
run_surgery.sh.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from ingest_canonical import fix_mojibake, fix_sentence_spacing  # type: ignore


SCENE_HEADER_RE = re.compile(r"^##\s+Ch(\d+)\s+S(\d+)(?:\s*[—–\-:].*)?\s*$", re.IGNORECASE)


def split_chapter(path: Path) -> list[tuple[int, int, str]]:
    lines = path.read_text(encoding="utf-8").splitlines()
    scenes: list[tuple[int, int, list[str]]] = []
    current: tuple[int, int, list[str]] | None = None
    for ln in lines[1:]:  # skip chapter heading
        m = SCENE_HEADER_RE.match(ln.strip())
        if m:
            if current is not None:
                scenes.append(current)
            current = (int(m.group(1)), int(m.group(2)), [])
            continue
        if current is not None:
            current[2].append(ln)
    if current is not None:
        scenes.append(current)
    return [(c, s, "\n".join(body).strip()) for c, s, body in scenes]


COMMA_LOOP_RE = re.compile(
    r"(\b\w+ed\b[^.!?]{0,40},[ ]\w+ed\b[^.!?]{0,40},[ ]\w+ed\b[^.!?]{0,40},[ ]\w+ed\b)",
    re.IGNORECASE,
)

COMMENTARY_LEADS = (
    "Here is the",
    "Here's the",
    "I have rewritten",
    "I've rewritten",
    "Below is the",
    "The rewritten scene",
    "Note:",
    "(Note",
    "Following the constraints",
    "Per your instructions",
)


def check_scene(ch: int, sc: int, body: str, target: int) -> list[str]:
    issues: list[str] = []
    wc = len(body.split())

    # 1. Length
    lo, hi = int(target * 0.7), int(target * 1.3)
    if wc < lo or wc > hi:
        issues.append(f"length {wc}w outside {lo}-{hi} (target {target})")

    # 2. POV (first-person should dominate)
    i_count = len(re.findall(r"\bI\b", body)) + len(re.findall(r"\bmy\b", body, re.I)) + len(re.findall(r"\bme\b", body, re.I))
    arjun_count = len(re.findall(r"\bArjun\b", body))
    # Count "he" / "his" only in narration: strip dialogue (rough — exclude
    # quoted spans).
    narration = re.sub(r'["“”].*?["“”]', "", body)
    he_count = len(re.findall(r"\bhe\b", narration, re.I)) + len(re.findall(r"\bhis\b", narration, re.I))
    if i_count == 0:
        issues.append(f"POV: no first-person pronouns (I={i_count}, he/his={he_count}, Arjun={arjun_count})")
    elif he_count > i_count * 0.5 and he_count >= 8:
        issues.append(f"POV: third-person presence (I={i_count}, he/his={he_count}, Arjun={arjun_count})")

    # 3. Comma loops
    m = COMMA_LOOP_RE.search(body)
    if m:
        issues.append(f"comma-spliced loop: …{m.group(1)[:120]}…")

    # 4. Model commentary
    first_line = body.split("\n", 1)[0].strip()
    for lead in COMMENTARY_LEADS:
        if first_line.lower().startswith(lead.lower()):
            issues.append(f"model commentary lead-in: {first_line[:80]!r}")
            break

    # 5. Mojibake (informational — we fix in normalise pass)
    _cleaned, mojibake_count = fix_mojibake(body)
    if mojibake_count > 0:
        issues.append(f"{mojibake_count} mojibake byte sequence(s) (fixable)")
    _cleaned, spacing_count = fix_sentence_spacing(body)
    if spacing_count > 0:
        issues.append(f"{spacing_count} missing-space defect(s) (fixable)")

    return issues


def main() -> int:
    p = argparse.ArgumentParser()
    p.add_argument("book_dir", nargs="?", default="my-confused-life")
    p.add_argument("--normalise", action="store_true",
                   help="apply mojibake + sentence-spacing fixes in place")
    args = p.parse_args()

    book_dir = Path(args.book_dir)
    if not book_dir.is_absolute():
        book_dir = Path(__file__).resolve().parent / book_dir
    chapters_dir = book_dir / "canonical" / "chapters"

    outline = json.loads((book_dir / "02-outline.json").read_text())
    chapters: list[dict] = []
    for part in outline.get("parts", []):
        for ch in part.get("chapters", []):
            chapters.append(ch)

    total_issues = 0
    total_scenes = 0
    total_words = 0
    total_mojibake_fixed = 0
    total_spacing_fixed = 0

    for ch_path in sorted(chapters_dir.glob("chapter-*.md")):
        scenes = split_chapter(ch_path)
        rewrite_needed = False
        rewritten_scenes: list[tuple[int, int, str]] = []
        for ch, sc, body in scenes:
            total_scenes += 1
            target = int(chapters[ch - 1]["scenes"][sc - 1].get("target_word_count") or 1200)
            wc = len(body.split())
            total_words += wc
            issues = check_scene(ch, sc, body, target)
            status = "OK" if not issues else "X"
            print(f"  [{status}] ch{ch}.s{sc}  {wc:5d}w (target {target})")
            for issue in issues:
                print(f"        • {issue}")
                total_issues += 1
            if args.normalise:
                cleaned, m_n = fix_mojibake(body)
                cleaned, s_n = fix_sentence_spacing(cleaned)
                total_mojibake_fixed += m_n
                total_spacing_fixed += s_n
                if m_n > 0 or s_n > 0:
                    rewrite_needed = True
                rewritten_scenes.append((ch, sc, cleaned))

        if args.normalise and rewrite_needed:
            # Rewrite the chapter file with cleaned bodies.
            lines = ch_path.read_text(encoding="utf-8").splitlines()
            heading = lines[0]
            new_lines = [heading, ""]
            for ch, sc, body in rewritten_scenes:
                new_lines.append(f"## Ch{ch} S{sc}")
                new_lines.append("")
                new_lines.append(body.strip())
                new_lines.append("")
            ch_path.write_text("\n".join(new_lines).rstrip() + "\n", encoding="utf-8")

    print()
    print(f"summary: {total_scenes} scenes, {total_words} words, {total_issues} issue(s)")
    if args.normalise:
        print(f"normalised: fixed {total_mojibake_fixed} mojibake + {total_spacing_fixed} spacing defect(s)")
    return 0 if total_issues == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
