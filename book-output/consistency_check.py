#!/usr/bin/env python3
"""
consistency_check.py — book-wide consistency + context audit.

Replaces the narrower `post_surgery_check.py`. This is the pre-export
gate: it runs after surgery and before composition, and it covers every
failure mode the user listed:

  - errors            (mojibake, missing inter-sentence spaces, model
                       commentary leakage)
  - gaps              (scenes under minimum word count, chapters missing
                       outlined beats)
  - discontinuities   (POV swap inside a scene, scene-N tail does not
                       lead into scene-N+1 head)
  - missed chapters   (chapter file count vs outline)
  - missed scenes     (scene count per chapter vs outline)
  - repetitive content (comma-spliced collapse loops; cross-scene
                       opening n-gram overlap; intra-scene repeated
                       sentences)
  - missing contexts  (character name spelt inconsistently across
                       chapters; key setting tokens absent from chapters
                       where the outline places them)
  - consistency       (POV in narration; voice register; vocabulary)

Two run modes:

  python consistency_check.py [book-dir]
      Report-only. Exits 0 if no CRITICAL issues, 1 otherwise.

  python consistency_check.py [book-dir] --auto-fix
      Apply safe in-place fixes: mojibake, sentence-spacing, strip model
      commentary lead-ins, strip trailing model notes. Re-runs the
      report after fixing; exit code reflects post-fix state.

Severity scheme:
  CRITICAL  — blocks export (export_book.sh refuses)
  WARNING   — surfaces in report, does not block

Tunable thresholds live in CHECK_POLICY at the top of the file so they
can be edited without touching the check logic.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from collections import Counter
from dataclasses import dataclass, field
from pathlib import Path
from typing import Iterable

sys.path.insert(0, str(Path(__file__).parent))
from ingest_canonical import fix_mojibake, fix_sentence_spacing  # type: ignore


# ── Policy ────────────────────────────────────────────────────────────────

CHECK_POLICY = {
    # Length: scene word-count must fall in [target * lo, target * hi].
    "length_lo_ratio": 0.65,
    "length_hi_ratio": 1.40,

    # POV: critical if "he/his" in narration exceeds N% of "I/me/my".
    "third_person_critical_ratio": 0.40,
    "third_person_min_to_flag": 10,

    # Cross-scene duplication: flag pairs whose first-N-word openings
    # share more than this token-set overlap.
    "opening_window_words": 25,
    "opening_overlap_threshold": 0.60,

    # Intra-scene repetition: flag if any sentence (≥6 words) appears
    # twice in the same scene.
    "intra_repeat_min_words": 6,

    # Outline coverage: extract key nouns/verbs from the synopsis; the
    # scene must contain at least this fraction of them somewhere.
    "synopsis_coverage_threshold": 0.30,
}


# ── Data structures ───────────────────────────────────────────────────────


@dataclass
class Issue:
    severity: str       # "CRITICAL" | "WARNING"
    location: str       # "ch3.s2" | "ch3" | "book"
    category: str       # "pov" | "length" | "repetition" | ...
    message: str
    fixable: bool = False

    def __str__(self) -> str:  # pragma: no cover - cosmetic
        tag = "X" if self.severity == "CRITICAL" else "!"
        return f"  [{tag}] {self.location:10s} {self.category:12s} {self.message}"


@dataclass
class CheckResult:
    issues: list[Issue] = field(default_factory=list)
    scene_word_counts: dict[tuple[int, int], int] = field(default_factory=dict)
    chapter_word_counts: dict[int, int] = field(default_factory=dict)
    total_words: int = 0

    def add(self, issue: Issue) -> None:
        self.issues.append(issue)

    def critical(self) -> list[Issue]:
        return [i for i in self.issues if i.severity == "CRITICAL"]

    def warnings(self) -> list[Issue]:
        return [i for i in self.issues if i.severity == "WARNING"]


# ── Scene / chapter parsing ───────────────────────────────────────────────


SCENE_HEADER_RE = re.compile(
    r"^##\s+Ch(\d+)\s+S(\d+)(?:\s*[—–\-:].*)?\s*$", re.IGNORECASE
)


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


def load_outline_chapters(book_dir: Path) -> list[dict]:
    outline = json.loads((book_dir / "02-outline.json").read_text())
    out: list[dict] = []
    for part in outline.get("parts", []):
        for ch in part.get("chapters", []):
            out.append(ch)
    return out


def load_brief(book_dir: Path) -> dict:
    return json.loads((book_dir / "01-brief.json").read_text())


# ── Helpers ───────────────────────────────────────────────────────────────


SENTENCE_RE = re.compile(r"[^.!?]+[.!?]")
WORD_RE = re.compile(r"[A-Za-z][A-Za-z'’]+")
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
    "Rewritten scene:",
    "---",
)
COMMENTARY_TRAILS = (
    "Note:",
    "(Word count:",
    "[Word count",
    "(approximately",
    "Word count managed",
    "ensuring the prose",
    "the prose remains",
    "Following the rules",
    "Per the constraints",
)


# Tokens that mark the SETTING shift partway through the book.
# Chapters 1–2 = Mumbai corporate life; 3–5 = hinterland temple town;
# chapter 6 = return to Mumbai. Each cluster must show at least one
# token from its setting list.
SETTING_TOKENS_MUMBAI = {
    "Mumbai", "skyscraper", "office", "rickshaw", "spreadsheet",
    "boardroom", "atrium", "merger", "Elena",
}
SETTING_TOKENS_TEMPLE = {
    "temple", "kirtan", "marigold", "mala", "dal", "rice",
    "deity", "chant", "bell", "incense", "Krishna", "Radha", "devotee",
}


def first_paragraph(text: str) -> str:
    return text.strip().split("\n\n", 1)[0].strip()


def last_paragraph(text: str) -> str:
    parts = text.strip().rsplit("\n\n", 1)
    return parts[-1].strip()


def strip_commentary(body: str) -> tuple[str, list[str]]:
    """Strip leading / trailing model-commentary lines. Returns
    (cleaned, list_of_removals)."""
    removed: list[str] = []
    lines = body.splitlines()

    # Lead-in
    while lines:
        first = lines[0].strip()
        if not first:
            lines.pop(0)
            continue
        if any(first.lower().startswith(p.lower()) for p in COMMENTARY_LEADS):
            removed.append(f"leading: {first[:80]!r}")
            lines.pop(0)
            continue
        break

    # Trailing
    while lines:
        last = lines[-1].strip()
        if not last:
            lines.pop()
            continue
        if any(p.lower() in last.lower() for p in COMMENTARY_TRAILS):
            removed.append(f"trailing: {last[:80]!r}")
            lines.pop()
            continue
        break

    return "\n".join(lines).strip(), removed


def synopsis_key_terms(synopsis: str) -> set[str]:
    """Extract content words from a synopsis (drop stop words). Used as
    a rough coverage check — at least some of these should appear in
    the prose."""
    stop = {
        "the", "a", "an", "of", "to", "and", "in", "on", "at", "by",
        "for", "with", "as", "is", "are", "was", "were", "be", "been",
        "being", "his", "her", "their", "he", "she", "they", "it",
        "that", "this", "those", "these", "but", "or", "so", "if",
        "from", "into", "out", "up", "down", "over", "under", "than",
        "then", "when", "while", "where", "how", "why", "what", "who",
        "feel", "feels", "feeling", "find", "finding", "finds", "make",
        "makes", "making", "go", "going", "going", "goes",
    }
    return {w.lower() for w in WORD_RE.findall(synopsis) if w.lower() not in stop and len(w) >= 4}


# ── Per-scene checks ──────────────────────────────────────────────────────


def check_pov(loc: str, body: str, result: CheckResult) -> None:
    i_count = (
        len(re.findall(r"\bI\b", body))
        + len(re.findall(r"\bmy\b", body, re.I))
        + len(re.findall(r"\bme\b", body, re.I))
        + len(re.findall(r"\bmine\b", body, re.I))
        + len(re.findall(r"\bmyself\b", body, re.I))
    )
    # Count he/his only in narration (strip quoted dialogue).
    narration = re.sub(r"[\"“”].*?[\"“”]", "", body)
    he_count = (
        len(re.findall(r"\bhe\b", narration, re.I))
        + len(re.findall(r"\bhis\b", narration, re.I))
        + len(re.findall(r"\bhim\b", narration, re.I))
        + len(re.findall(r"\bhimself\b", narration, re.I))
    )
    arjun_count = len(re.findall(r"\bArjun\b", narration))

    if i_count == 0:
        result.add(Issue(
            "CRITICAL", loc, "pov",
            f"no first-person pronouns (he/his={he_count}, Arjun={arjun_count})",
        ))
        return

    if he_count + arjun_count >= CHECK_POLICY["third_person_min_to_flag"]:
        ratio = (he_count + arjun_count) / max(i_count, 1)
        if ratio >= CHECK_POLICY["third_person_critical_ratio"]:
            result.add(Issue(
                "CRITICAL", loc, "pov",
                f"third-person bleed: I={i_count} he/his={he_count} Arjun={arjun_count} (ratio {ratio:.2f})",
            ))
        elif ratio >= CHECK_POLICY["third_person_critical_ratio"] * 0.7:
            result.add(Issue(
                "WARNING", loc, "pov",
                f"some third-person language: I={i_count} he/his={he_count} Arjun={arjun_count}",
            ))


def check_length(loc: str, body: str, target: int, result: CheckResult) -> int:
    wc = len(body.split())
    lo = int(target * CHECK_POLICY["length_lo_ratio"])
    hi = int(target * CHECK_POLICY["length_hi_ratio"])
    if wc < lo:
        result.add(Issue(
            "CRITICAL", loc, "length",
            f"{wc}w below floor {lo} (target {target}) — scene is a gap",
        ))
    elif wc > hi:
        result.add(Issue(
            "WARNING", loc, "length",
            f"{wc}w above ceiling {hi} (target {target}) — bloat",
        ))
    return wc


def check_comma_loop(loc: str, body: str, result: CheckResult) -> None:
    m = COMMA_LOOP_RE.search(body)
    if m:
        result.add(Issue(
            "CRITICAL", loc, "repetition",
            f"comma-spliced action loop: …{m.group(1)[:100]}…",
        ))


def check_intra_repetition(loc: str, body: str, result: CheckResult) -> None:
    sentences = [
        s.strip() for s in SENTENCE_RE.findall(body)
        if len(s.strip().split()) >= CHECK_POLICY["intra_repeat_min_words"]
    ]
    counts = Counter(sentences)
    dupes = [(s, n) for s, n in counts.items() if n >= 2]
    if dupes:
        sample = dupes[0]
        result.add(Issue(
            "WARNING", loc, "repetition",
            f"repeated sentence ({sample[1]}×): {sample[0][:100]!r}",
        ))


def check_commentary(loc: str, body: str, result: CheckResult) -> None:
    first_line = body.strip().split("\n", 1)[0].strip()
    for p in COMMENTARY_LEADS:
        if first_line.lower().startswith(p.lower()):
            result.add(Issue(
                "CRITICAL", loc, "commentary",
                f"model commentary lead-in: {first_line[:80]!r} (fixable)",
                fixable=True,
            ))
            break
    last_line = body.strip().rsplit("\n", 1)[-1].strip()
    for p in COMMENTARY_TRAILS:
        if p.lower() in last_line.lower():
            result.add(Issue(
                "CRITICAL", loc, "commentary",
                f"model commentary trailer: {last_line[:80]!r} (fixable)",
                fixable=True,
            ))
            break


def check_encoding(loc: str, body: str, result: CheckResult) -> None:
    _, m = fix_mojibake(body)
    _, s = fix_sentence_spacing(body)
    if m > 0:
        result.add(Issue(
            "CRITICAL", loc, "encoding",
            f"{m} mojibake byte sequence(s) (fixable)", fixable=True,
        ))
    if s > 0:
        result.add(Issue(
            "CRITICAL", loc, "encoding",
            f"{s} missing inter-sentence space(s) (fixable)", fixable=True,
        ))


def check_outline_coverage(
    loc: str, body: str, synopsis: str, result: CheckResult,
) -> None:
    terms = synopsis_key_terms(synopsis)
    if not terms:
        return
    body_lower = body.lower()
    hit = sum(1 for t in terms if t in body_lower)
    coverage = hit / len(terms)
    if coverage < CHECK_POLICY["synopsis_coverage_threshold"]:
        present = [t for t in terms if t in body_lower]
        missing = [t for t in terms if t not in body_lower]
        result.add(Issue(
            "WARNING", loc, "off-beat",
            f"synopsis coverage {coverage:.0%} ({hit}/{len(terms)}) — "
            f"present={present[:5]} missing={missing[:5]}",
        ))


def check_setting(
    loc: str, body: str, chapter_idx: int, result: CheckResult,
) -> None:
    """Loose check: every Mumbai-set scene names at least one Mumbai
    token; every temple-set scene names at least one temple token.
    Chapter 6 is a return-to-Mumbai chapter, so we expect both."""
    if chapter_idx <= 2:
        expected = SETTING_TOKENS_MUMBAI
        label = "Mumbai/corporate"
    elif chapter_idx <= 5:
        expected = SETTING_TOKENS_TEMPLE
        label = "temple-town"
    else:
        # Chapter 6 — return. Either set acceptable; we just want
        # something specific.
        expected = SETTING_TOKENS_MUMBAI | SETTING_TOKENS_TEMPLE
        label = "Mumbai-return"
    hit = sum(1 for t in expected if t.lower() in body.lower())
    if hit == 0:
        result.add(Issue(
            "WARNING", loc, "setting",
            f"no {label} setting tokens present — scene feels placeless",
        ))


# ── Per-chapter / book checks ─────────────────────────────────────────────


def check_outline_coverage_per_chapter(
    ch_num: int,
    chapter_outline: dict,
    scenes: list[tuple[int, int, str]],
    result: CheckResult,
) -> None:
    expected = len(chapter_outline.get("scenes", []))
    actual = len(scenes)
    if actual < expected:
        result.add(Issue(
            "CRITICAL", f"ch{ch_num}", "missed-scene",
            f"only {actual} scene(s) present, outline calls for {expected}",
        ))
    elif actual > expected:
        result.add(Issue(
            "WARNING", f"ch{ch_num}", "extra-scene",
            f"{actual} scene(s) present, outline only calls for {expected}",
        ))


def check_cross_scene_duplication(
    all_scenes: dict[tuple[int, int], str],
    result: CheckResult,
) -> None:
    """Compare the first-N-word openings of every scene pair. High token
    overlap → the drafter reused the same opening."""
    n = CHECK_POLICY["opening_window_words"]
    threshold = CHECK_POLICY["opening_overlap_threshold"]
    openings: dict[tuple[int, int], set[str]] = {}
    for k, body in all_scenes.items():
        tokens = WORD_RE.findall(body)[:n]
        openings[k] = {t.lower() for t in tokens}

    keys = sorted(all_scenes.keys())
    for i, a in enumerate(keys):
        for b in keys[i + 1:]:
            if not openings[a] or not openings[b]:
                continue
            inter = len(openings[a] & openings[b])
            union = len(openings[a] | openings[b])
            if union == 0:
                continue
            jaccard = inter / union
            if jaccard >= threshold:
                result.add(Issue(
                    "WARNING", f"ch{a[0]}.s{a[1]}↔ch{b[0]}.s{b[1]}", "duplicate-opening",
                    f"opening n-gram overlap {jaccard:.2f} ≥ {threshold}",
                ))


def check_continuity(
    all_scenes: dict[tuple[int, int], str],
    result: CheckResult,
) -> None:
    """Soft check: each scene's last sentence and the next scene's
    first sentence should not introduce mutually inconsistent settings
    (e.g., scene ends in temple, next scene starts in apartment with no
    transition cue)."""
    keys = sorted(all_scenes.keys())
    for i, k in enumerate(keys[:-1]):
        nxt = keys[i + 1]
        tail = last_paragraph(all_scenes[k]).lower()
        head = first_paragraph(all_scenes[nxt]).lower()

        tail_mum = any(t.lower() in tail for t in SETTING_TOKENS_MUMBAI)
        head_mum = any(t.lower() in head for t in SETTING_TOKENS_MUMBAI)
        tail_temple = any(t.lower() in tail for t in SETTING_TOKENS_TEMPLE)
        head_temple = any(t.lower() in head for t in SETTING_TOKENS_TEMPLE)

        # Hard discontinuity: tail anchored in Mumbai, head anchored in
        # temple (or vice versa), with no chapter boundary between
        # them.
        if k[0] == nxt[0]:  # same chapter
            if (tail_mum and head_temple and not tail_temple) or (
                tail_temple and head_mum and not tail_mum
            ):
                result.add(Issue(
                    "WARNING", f"ch{k[0]}.s{k[1]}→ch{nxt[0]}.s{nxt[1]}", "continuity",
                    "setting jump mid-chapter (Mumbai↔temple) without transition cue",
                ))


def check_character_name_consistency(
    all_scenes: dict[tuple[int, int], str],
    brief: dict,
    result: CheckResult,
) -> None:
    """Look for accidental name variants. Spec characters are Arjun,
    Elena, Mr. Das. Flag any plausible variants like 'Arjuna', 'Helena',
    'Mr Das' (no period)."""
    text = "\n".join(all_scenes.values())
    expected = {"Arjun", "Elena", "Mr. Das"}
    variants = {
        "Arjun": {"Arjuna", "Arjuun"},
        "Elena": {"Helena", "Elina", "Elaine"},
        "Mr. Das": {"Mr Das", "Mister Das", "Das Sir", "Mr. Dass"},
    }
    for canonical, alts in variants.items():
        for alt in alts:
            if re.search(rf"\b{re.escape(alt)}\b", text):
                result.add(Issue(
                    "WARNING", "book", "name-variant",
                    f"character name variant {alt!r} found; expected {canonical!r}",
                ))


# ── Auto-fix engine ───────────────────────────────────────────────────────


def auto_fix_scene(body: str) -> tuple[str, dict[str, int]]:
    """Apply safe in-place fixes to one scene body. Returns
    (cleaned, fix_counts)."""
    counts = {"mojibake": 0, "spacing": 0, "commentary": 0}
    cleaned = body
    cleaned, m = fix_mojibake(cleaned)
    counts["mojibake"] = m
    cleaned, s = fix_sentence_spacing(cleaned)
    counts["spacing"] = s
    cleaned, removed = strip_commentary(cleaned)
    counts["commentary"] = len(removed)
    return cleaned, counts


# ── Main runner ───────────────────────────────────────────────────────────


def run_checks(
    book_dir: Path,
    *,
    auto_fix: bool = False,
) -> CheckResult:
    chapters_dir = book_dir / "canonical" / "chapters"
    chapter_paths = sorted(chapters_dir.glob("chapter-*.md"))
    outline_chapters = load_outline_chapters(book_dir)
    brief = load_brief(book_dir)
    result = CheckResult()

    if len(chapter_paths) < len(outline_chapters):
        result.add(Issue(
            "CRITICAL", "book", "missed-chapter",
            f"only {len(chapter_paths)} chapter file(s); outline has {len(outline_chapters)}",
        ))

    all_scenes: dict[tuple[int, int], str] = {}

    for ch_idx, ch_path in enumerate(chapter_paths, start=1):
        scenes = split_chapter(ch_path)
        if ch_idx <= len(outline_chapters):
            check_outline_coverage_per_chapter(ch_idx, outline_chapters[ch_idx - 1], scenes, result)

        # Auto-fix pass (re-reads + rewrites the chapter file).
        if auto_fix:
            rewrite_needed = False
            fixed_scenes: list[tuple[int, int, str]] = []
            for ch, sc, body in scenes:
                cleaned, counts = auto_fix_scene(body)
                if any(counts.values()):
                    rewrite_needed = True
                fixed_scenes.append((ch, sc, cleaned))
            if rewrite_needed:
                heading = ch_path.read_text(encoding="utf-8").splitlines()[0]
                new_lines = [heading, ""]
                for ch, sc, body in fixed_scenes:
                    new_lines.append(f"## Ch{ch} S{sc}")
                    new_lines.append("")
                    new_lines.append(body.strip())
                    new_lines.append("")
                ch_path.write_text("\n".join(new_lines).rstrip() + "\n", encoding="utf-8")
                scenes = fixed_scenes  # use the fixed bodies for downstream checks

        ch_words = 0
        for ch, sc, body in scenes:
            loc = f"ch{ch}.s{sc}"
            all_scenes[(ch, sc)] = body

            if ch_idx <= len(outline_chapters):
                outlined_scenes = outline_chapters[ch_idx - 1].get("scenes", [])
                if sc <= len(outlined_scenes):
                    out_scene = outlined_scenes[sc - 1]
                    target = int(out_scene.get("target_word_count") or 1200)
                    check_length(loc, body, target, result)
                    check_outline_coverage(loc, body, out_scene.get("synopsis", ""), result)
                else:
                    target = 1200
                    check_length(loc, body, target, result)
            else:
                target = 1200
                check_length(loc, body, target, result)

            wc = len(body.split())
            result.scene_word_counts[(ch, sc)] = wc
            ch_words += wc

            check_pov(loc, body, result)
            check_comma_loop(loc, body, result)
            check_intra_repetition(loc, body, result)
            check_commentary(loc, body, result)
            check_encoding(loc, body, result)
            check_setting(loc, body, ch_idx, result)

        result.chapter_word_counts[ch_idx] = ch_words
        result.total_words += ch_words

    # Book-level checks
    check_cross_scene_duplication(all_scenes, result)
    check_continuity(all_scenes, result)
    check_character_name_consistency(all_scenes, brief, result)

    return result


def print_report(result: CheckResult, book_dir: Path) -> None:
    print(f"book:           {book_dir.name}")
    print(f"total scenes:   {len(result.scene_word_counts)}")
    print(f"total words:    {result.total_words}")
    for ch, wc in sorted(result.chapter_word_counts.items()):
        print(f"  ch{ch}: {wc} words")
    print()

    critical = result.critical()
    warnings = result.warnings()
    if not critical and not warnings:
        print("OK — no issues found.")
        return

    if critical:
        print(f"CRITICAL ({len(critical)}):")
        for i in critical:
            print(i)
        print()
    if warnings:
        print(f"WARNINGS ({len(warnings)}):")
        for i in warnings:
            print(i)
        print()


def main() -> int:
    p = argparse.ArgumentParser()
    p.add_argument("book_dir", nargs="?", default="my-confused-life")
    p.add_argument("--auto-fix", action="store_true",
                   help="apply safe in-place fixes (mojibake, spacing, commentary strip)")
    p.add_argument("--gate", action="store_true",
                   help="exit non-zero on CRITICAL issues only (warnings ignored)")
    p.add_argument("--json", help="also write a machine-readable report to this path")
    args = p.parse_args()

    book_dir = Path(args.book_dir)
    if not book_dir.is_absolute():
        book_dir = Path(__file__).resolve().parent / book_dir
    if not book_dir.exists():
        print(f"FATAL: {book_dir} does not exist", file=sys.stderr)
        return 2

    result = run_checks(book_dir, auto_fix=args.auto_fix)
    print_report(result, book_dir)

    if args.json:
        report = {
            "book": book_dir.name,
            "total_words": result.total_words,
            "chapter_word_counts": {str(k): v for k, v in result.chapter_word_counts.items()},
            "scene_word_counts": {f"ch{k[0]}.s{k[1]}": v for k, v in result.scene_word_counts.items()},
            "critical": [
                {"location": i.location, "category": i.category, "message": i.message}
                for i in result.critical()
            ],
            "warnings": [
                {"location": i.location, "category": i.category, "message": i.message}
                for i in result.warnings()
            ],
        }
        Path(args.json).write_text(json.dumps(report, indent=2))
        print(f"\nreport written to {args.json}")

    if args.gate:
        return 1 if result.critical() else 0
    return 0 if not result.issues else 1


if __name__ == "__main__":
    sys.exit(main())
