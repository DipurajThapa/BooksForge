#!/usr/bin/env python3
"""
scene_surgery.py — rewrites each scene of a canonical-pipeline manuscript
to fix the content-level defects the validator can't fix downstream:

  - POV inconsistency (some scenes first-person, some third-person)
  - Length bloat (drafter ran past target and collapsed into
    comma-spliced loops: "walked to elevator, pressed button, doors
    opened, stepped in, pressed lobby...")
  - Repetition (same beat replayed two or three times in one scene)
  - Off-beat content (scene drifted away from its outlined synopsis)

Strategy: hand each scene to qwen3.6:latest (the polish/heavy model)
with:
  - the original outline scene synopsis (the intent)
  - the chapter purpose (the local arc)
  - the previous-scene tail and next-scene head (continuity hooks)
  - the existing draft text (as raw material — preserve imagery, fix
    prose; pre-trim if bloated to drop the collapse tail)
  - hard constraints: POV, target length, no comma-spliced lists, no
    repetition

The brief specifies "introspective first-person past tense throughout".
The canonical drafter drifted from this in 12 of 18 scenes. Surgery
normalises everything to first-person to honour the spec.

Usage:
    python scene_surgery.py [book-dir] [--scenes ch1.s1,ch3.s2,...] [--all]

Output: rewritten chapter files at
    <book-dir>/canonical/chapters/chapter-NN.md

A backup of the originals is written to
    <book-dir>/canonical/chapters.pre-surgery/
on first run.
"""

from __future__ import annotations

import argparse
import json
import re
import shutil
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from book_helpers import chat_no_thinking  # type: ignore


MODEL_HEAVY = "qwen3.6:latest"  # reserved for critical-thinking / polish

# Maximum input scene length in words. Bloated scenes (5K+ words) get
# trimmed to the first MAX_INPUT_WORDS — the collapse tail is always at
# the end (drafter ran past target and looped), so the strong prose
# lives in the head.
MAX_INPUT_WORDS = 3000


def load_outline(book_dir: Path) -> tuple[dict, list[dict]]:
    brief = json.loads((book_dir / "01-brief.json").read_text())
    outline = json.loads((book_dir / "02-outline.json").read_text())
    chapters: list[dict] = []
    for part in outline.get("parts", []):
        for ch in part.get("chapters", []):
            chapters.append(ch)
    return brief, chapters


# Accept both shapes:
#   "## Ch1 S1 — Some Title"     (original Rust-emitted form)
#   "## Ch1 S1"                  (rewritten form this tool emits)
SCENE_HEADER_RE = re.compile(r"^##\s+Ch(\d+)\s+S(\d+)(?:\s*[—–\-:].*)?\s*$", re.IGNORECASE)


def split_chapter_into_scenes(path: Path) -> tuple[str, list[tuple[int, int, str]]]:
    """Return (chapter_heading, list_of_(ch, sc, body))."""
    lines = path.read_text(encoding="utf-8").splitlines()
    if not lines:
        return "", []
    chapter_heading = lines[0]
    scenes: list[tuple[int, int, list[str]]] = []
    current: tuple[int, int, list[str]] | None = None
    for ln in lines[1:]:
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
    return chapter_heading, [(c, s, "\n".join(body).strip()) for c, s, body in scenes]


def trim_input_for_bloat(text: str) -> str:
    """If the scene body is bloated, trim to the first MAX_INPUT_WORDS
    words. The collapse tail (comma-spliced loop) is always at the end,
    so keeping the head preserves the strongest prose."""
    words = text.split()
    if len(words) <= MAX_INPUT_WORDS:
        return text
    trimmed = " ".join(words[:MAX_INPUT_WORDS])
    # Cut at the last paragraph break before the limit so we don't slice
    # a sentence in half.
    last_para = trimmed.rfind("\n\n")
    if last_para > MAX_INPUT_WORDS * 4:  # crude — keep most of it
        trimmed = trimmed[:last_para]
    return trimmed + "\n\n[draft truncated — model went on past target, "
    "tail discarded as filler]"


def first_paragraph(text: str) -> str:
    text = text.strip()
    if not text:
        return ""
    parts = text.split("\n\n", 1)
    return parts[0].strip()


def last_paragraph(text: str) -> str:
    text = text.strip()
    if not text:
        return ""
    parts = text.rsplit("\n\n", 1)
    return parts[-1].strip()


# ── Running digest (book-wide context for each rewrite) ──────────────────
#
# The rewrite model needs more than the immediate prev/next scene hooks to
# stay consistent across the book. It needs to know:
#
#   - which named characters exist and how they relate to Arjun
#   - which settings exist and which chapter is in which one
#   - what's happened so far in the narrative (running story-so-far summary)
#   - which key objects / motifs recur (marigolds, brass bell, mala, etc.)
#
# Without this, scene 5 might suddenly call Elena's brother by a name not
# established in scene 2, or reverse a fact set up in chapter 1. We build
# the digest once from the brief + outline + already-written scene heads
# and pass it into every rewrite as a "BOOK CONTEXT" block.


CANON_CHARACTERS = """\
Arjun — narrator and protagonist. Corporate man in his early thirties,
working at a Mumbai firm. The whole novel is his first-person past-tense
account.

Elena — Arjun's wife. Cold, efficient, controlling. Works in the same
corporate orbit. Pre-existing distance between them is the marital
backdrop the book opens on.

Mr. Das — Arjun's mentor at work. Dies near the start of the book
(chapter 1, scene 3). Smells of old paper and dried tea leaves. His
death is the catalyst that breaks Arjun open.
"""


CANON_SETTINGS = """\
Mumbai (chapters 1–2, 6) — corporate skyscrapers, glass atriums,
auto-rickshaws, markets selling marigolds and jalebis, the smart-home
apartment Arjun shares with Elena.

The hinterland temple town (chapters 3–5) — smaller, warmer, slower.
A small Krishna-Radha temple where kirtan is sung; a kitchen behind
the temple where dal and rice are served by volunteers. Marigolds,
brass bell, banana leaf, the smell of incense.
"""


CANON_MOTIFS = """\
Recurring objects: marigolds, the brass bell, a banana leaf, the
smart-home's blue light, the rickshaw, the ceiling fan, the mala
(prayer beads), the empty chair at Mr. Das's desk.

Recurring sense-memories: the smell of marigolds; the click of the
ceiling fan; the hum of the office air conditioning; the scent of old
paper and dried tea leaves (Mr. Das); the taste of dal and rice
(temple kitchen).

Recurring inner states: the question "what was I doing?"; the hollow
behind the sternum; the gap between Arjun and the people around him.
"""


def story_so_far_summary(
    chapters_outline: list[dict],
    chapter_idx: int,
    scene_idx: int,
    scene_bodies: dict[tuple[int, int], str],
) -> str:
    """Build a one-sentence-per-completed-scene running summary of
    everything that has already happened in the book up to this scene.

    For scenes that have been rewritten this run (or that already exist
    on disk), use the first sentence of the actual prose as the
    summary. For scenes that haven't been rewritten yet, fall back to
    the outline synopsis.
    """
    lines: list[str] = []
    for ci, ch in enumerate(chapters_outline, start=1):
        outlined_scenes = ch.get("scenes", [])
        for si in range(1, len(outlined_scenes) + 1):
            if (ci, si) == (chapter_idx, scene_idx):
                return "\n".join(lines) if lines else "[this is the opening scene of the book]"
            body = scene_bodies.get((ci, si), "")
            if body:
                # First sentence of the rewritten scene.
                first = body.strip().split("\n", 1)[0].strip()
                first_sent_match = re.match(r"^([^.!?]+[.!?])", first)
                summary = first_sent_match.group(1) if first_sent_match else first[:140]
            else:
                summary = outlined_scenes[si - 1].get("synopsis", "")
            lines.append(f"  - Ch{ci} S{si}: {summary}")
    return "\n".join(lines) if lines else "[this is the opening scene of the book]"


def build_prompts(
    *,
    brief: dict,
    chapter: dict,
    scene: dict,
    scene_idx: int,
    chapter_idx: int,
    current_draft: str,
    prev_tail: str,
    next_head: str,
    story_so_far: str = "",
) -> tuple[str, str]:
    target = int(scene.get("target_word_count") or 1200)
    synopsis = scene.get("synopsis", "")
    purpose = chapter.get("purpose", "")
    beat = scene.get("beat", "")
    genre = brief.get("genre", "Literary Fiction")
    tone = brief.get("tone", "")

    system = f"""You are a literary fiction editor working on a novella titled "{(brief.get("title_suggestions") or ["My Confused Life"])[0]}".

GENRE: {genre}
TONE: {tone}
SETTING: contemporary urban India — Mumbai for the corporate-life chapters; a smaller town in the hinterland for the temple-and-recovery chapters.

You are rewriting ONE scene of the novella. Your output is the finished prose of that scene, ready for publication.

ABSOLUTE CONSTRAINTS — every one of these is a publication-blocker if violated:

1. POV: FIRST-PERSON PAST TENSE THROUGHOUT.
   - The narrator is the protagonist, Arjun. He tells the story in his own voice.
   - Use "I", "me", "my", "mine", "myself".
   - DO NOT write "Arjun did X", "he felt Y", "his hand", "his eyes". Those forms are forbidden in narration.
   - The only exception: other characters MAY address Arjun by name in dialogue ("Arjun, listen…").

2. LENGTH: target ~{target} words. Acceptable range: {int(target * 0.85)}–{int(target * 1.15)} words. Do NOT exceed {int(target * 1.2)} words.

3. NO COMMA-SPLICED ACTION LISTS. Forbidden pattern: "walked to elevator, pressed button, doors opened, stepped in, pressed lobby". Each clause must be a complete sentence with a subject and a verb, joined by periods or proper conjunctions.

4. NO REPETITION. If the draft below shows Arjun doing the same thing twice (e.g., walking to the elevator twice, opening the door twice, sitting down twice), include it only ONCE.

5. STAY ON BEAT. The scene must dramatise THIS scene's synopsis and no other. Do not pull in plot from later chapters. Do not invent characters not in the existing draft.

6. CONTINUITY. The opening sentence must flow naturally from the previous scene's tail (given below). The closing sentence must hand off into the next scene's head (given below).

7. PRESERVE the strongest sensory details from the draft below: specific smells (marigolds, ozone, diesel, jasmine, tea leaves), specific sounds (bells, fans, rickshaw engines, kirtan chant), specific objects (the brass bell, the banana leaf, the smart-home blue light). Discard the filler around them.

8. NO AI TELLS. Forbidden: "the truth was…", "in that moment…", three-adjective stacks ("warm, soft, and gentle"), over-explained metaphors ("like a bird in a cage of his own making, trapped and yearning").

9. NO FORBIDDEN TROPES. No chosen-one revelation, no saintly guru fixing Arjun's life in one sentence, no romance-as-rescue, no grand conversion. The devotional content (Radha-Krishna, kirtan) is shown as Arjun encounters it — half-skeptical, faintly embarrassed, surprised by feeling.

10. OUTPUT FORMAT. Only the prose of the scene. No headings, no scene labels like "Scene 1", no commentary, no notes. Paragraphs separated by blank lines.
"""

    user = f"""BOOK CONTEXT (canonical facts — do not contradict):

CHARACTERS:
{CANON_CHARACTERS}
SETTINGS:
{CANON_SETTINGS}
RECURRING MOTIFS:
{CANON_MOTIFS}

STORY SO FAR (every scene before this one, summarised in one line):
{story_so_far or "[this is the opening scene of the book]"}

────────────────────────────────────────────────────────────────────

CHAPTER {chapter_idx} PURPOSE:
{purpose}

SCENE {scene_idx} BEAT: {beat}

SCENE {scene_idx} SYNOPSIS (what this scene must dramatise — stay inside it; do not invent new plot):
{synopsis}

PREVIOUS-SCENE TAIL (end your scene on a beat that follows this):
{prev_tail or "[OPENING OF BOOK — start cold]"}

NEXT-SCENE HEAD (lead into this):
{next_head or "[END OF BOOK — close definitively]"}

EXISTING DRAFT OF THIS SCENE (raw material — use the imagery, fix the POV and prose, drop repetition):
---
{current_draft}
---

Write the finished scene now. First-person past tense ("I", "me", "my"). ~{target} words. Plain paragraphs. Output only the prose.
"""
    return system, user


def rewrite_scene(
    *,
    brief: dict,
    chapter: dict,
    chapter_idx: int,
    scene_idx: int,
    current_draft: str,
    prev_tail: str,
    next_head: str,
    story_so_far: str = "",
) -> tuple[str, dict]:
    scene = chapter["scenes"][scene_idx - 1]
    target = int(scene.get("target_word_count") or 1200)
    system, user = build_prompts(
        brief=brief,
        chapter=chapter,
        scene=scene,
        scene_idx=scene_idx,
        chapter_idx=chapter_idx,
        current_draft=trim_input_for_bloat(current_draft),
        prev_tail=prev_tail,
        next_head=next_head,
        story_so_far=story_so_far,
    )
    # max_tokens: target × 2.0 gives the model headroom but caps the
    # runaway loop behaviour we saw in the original draft.
    max_tok = int(target * 2.0) + 256
    out, meta = chat_no_thinking(
        MODEL_HEAVY, system, user,
        temperature=0.45, max_tokens=max_tok,
    )
    return out.strip(), meta


def collect_all_scenes(
    chapters_dir: Path,
) -> dict[tuple[int, int], str]:
    """Read every chapter file and return {(ch, sc): body}."""
    scenes: dict[tuple[int, int], str] = {}
    for path in sorted(chapters_dir.glob("chapter-*.md")):
        _heading, scene_list = split_chapter_into_scenes(path)
        for ch, sc, body in scene_list:
            scenes[(ch, sc)] = body
    return scenes


def parse_scene_filter(spec: str | None) -> set[tuple[int, int]] | None:
    if not spec:
        return None
    out: set[tuple[int, int]] = set()
    for token in spec.split(","):
        token = token.strip().lower()
        m = re.match(r"ch(\d+)\.s(\d+)", token)
        if not m:
            raise ValueError(f"bad scene spec: {token!r}; want ch1.s2")
        out.add((int(m.group(1)), int(m.group(2))))
    return out


def main() -> int:
    p = argparse.ArgumentParser()
    p.add_argument("book_dir", nargs="?", default="my-confused-life")
    p.add_argument("--scenes", help="comma-separated list like ch1.s2,ch3.s1")
    p.add_argument("--all", action="store_true", help="rewrite every scene")
    p.add_argument("--dry-run", action="store_true")
    args = p.parse_args()

    book_dir = Path(args.book_dir)
    if not book_dir.is_absolute():
        book_dir = Path(__file__).resolve().parent / book_dir
    chapters_dir = book_dir / "canonical" / "chapters"
    if not chapters_dir.exists():
        print(f"FATAL: {chapters_dir} missing", file=sys.stderr)
        return 2

    brief, chapters = load_outline(book_dir)
    scene_bodies = collect_all_scenes(chapters_dir)

    # Build the (ch, sc) execution order (chapters and scenes in
    # sequence) so we can compute prev/next continuity hooks against
    # the LATEST available text — which means rewrites done earlier in
    # the run feed forward as context to later rewrites.
    all_scene_keys = sorted(scene_bodies.keys())
    filter_set = parse_scene_filter(args.scenes)
    if args.all:
        targets = list(all_scene_keys)
    elif filter_set:
        targets = [k for k in all_scene_keys if k in filter_set]
    else:
        # Default: rewrite every scene. The brief's first-person POV
        # mandate plus the bloat in 1.3/5.1/6.1 means a full pass is
        # the right shape.
        targets = list(all_scene_keys)

    print(f"book_dir:    {book_dir}")
    print(f"scenes total: {len(all_scene_keys)}  targets: {len(targets)}")
    print(f"model:        {MODEL_HEAVY}")
    if args.dry_run:
        for ch, sc in targets:
            print(f"  would rewrite ch{ch}.s{sc} (current {len(scene_bodies[(ch, sc)].split())} words)")
        return 0

    # Backup the originals once (idempotent).
    backup_dir = book_dir / "canonical" / "chapters.pre-surgery"
    if not backup_dir.exists():
        shutil.copytree(chapters_dir, backup_dir)
        print(f"backed up originals to {backup_dir.relative_to(book_dir)}")

    started = time.time()
    log_path = book_dir / "canonical" / "scene-surgery.log"
    log_lines: list[str] = []

    def log(msg: str) -> None:
        ts = time.strftime("%H:%M:%S")
        line = f"[{ts}] {msg}"
        print(line, flush=True)
        log_lines.append(line)
        log_path.write_text("\n".join(log_lines))

    for ch, sc in targets:
        chapter = chapters[ch - 1]
        scene = chapter["scenes"][sc - 1]
        target = int(scene.get("target_word_count") or 1200)
        current = scene_bodies[(ch, sc)]
        # prev_tail: from the same scene index minus one (or last scene
        # of previous chapter, or empty for the book's first scene).
        prev_key: tuple[int, int] | None = None
        if sc > 1:
            prev_key = (ch, sc - 1)
        elif ch > 1:
            # last scene of previous chapter
            prev_scenes_of_prev = [k for k in all_scene_keys if k[0] == ch - 1]
            prev_key = prev_scenes_of_prev[-1] if prev_scenes_of_prev else None
        # next_head: same scene + 1 (or first scene of next chapter).
        next_key: tuple[int, int] | None = None
        if (ch, sc + 1) in scene_bodies:
            next_key = (ch, sc + 1)
        elif (ch + 1, 1) in scene_bodies:
            next_key = (ch + 1, 1)

        prev_tail = last_paragraph(scene_bodies[prev_key]) if prev_key else ""
        next_head = first_paragraph(scene_bodies[next_key]) if next_key else ""

        # Build running "story so far" — every completed scene gets one
        # summary line, so the model has full book context for this rewrite.
        story = story_so_far_summary(chapters, ch, sc, scene_bodies)

        log(f"ch{ch}.s{sc} — target {target}w, current {len(current.split())}w  →  rewriting via {MODEL_HEAVY}...")
        t0 = time.time()
        try:
            new_body, meta = rewrite_scene(
                brief=brief,
                chapter=chapter,
                chapter_idx=ch,
                scene_idx=sc,
                current_draft=current,
                prev_tail=prev_tail,
                next_head=next_head,
                story_so_far=story,
            )
        except Exception as e:  # noqa: BLE001
            log(f"  !! ch{ch}.s{sc} failed: {type(e).__name__}: {e}")
            continue
        new_wc = len(new_body.split())
        elapsed = time.time() - t0
        log(f"  done in {elapsed:.1f}s  →  {new_wc} words  (target {target}, drift {new_wc - target:+d})")

        # In-memory update so subsequent scenes pick up the new prev_tail.
        scene_bodies[(ch, sc)] = new_body

        # Persist this chapter immediately so a ctrl-C leaves us in a
        # consistent state.
        write_chapter(chapters_dir, ch, chapters, scene_bodies)

    write_all_chapters(chapters_dir, chapters, scene_bodies)
    total = time.time() - started
    log(f"\nfinished {len(targets)} scene(s) in {total / 60:.1f} min")
    log(f"\nNext: python compose_book.py {book_dir.name} && ./export_book.sh {book_dir.name}")
    return 0


def write_chapter(
    chapters_dir: Path,
    ch_num: int,
    chapters: list[dict],
    scene_bodies: dict[tuple[int, int], str],
) -> None:
    """Write a single chapter-NN.md file with its current scenes."""
    chapter = chapters[ch_num - 1]
    title = chapter.get("title", f"Chapter {ch_num}")
    # Strip "Chapter N: " or "Chapter N — " prefix from outline title.
    clean_title = re.sub(
        r"^\s*chapter\s+\d+\s*[—–\-:]\s*",
        "",
        title.strip(),
        flags=re.IGNORECASE,
    ).strip()
    lines = [f"## Chapter {ch_num} — {clean_title}", ""]
    scene_keys = sorted(k for k in scene_bodies if k[0] == ch_num)
    for ch, sc in scene_keys:
        # Each scene gets a labelled H2 so the composer's parse_chapter_file
        # can convert it to a * * * ornament. The label is just structural —
        # it never appears in the final PDF/EPUB output.
        lines.append(f"## Ch{ch} S{sc}")
        lines.append("")
        lines.append(scene_bodies[(ch, sc)].strip())
        lines.append("")
    (chapters_dir / f"chapter-{ch_num:02d}.md").write_text(
        "\n".join(lines).rstrip() + "\n", encoding="utf-8"
    )


def write_all_chapters(
    chapters_dir: Path,
    chapters: list[dict],
    scene_bodies: dict[tuple[int, int], str],
) -> None:
    chapter_nums = sorted({k[0] for k in scene_bodies})
    for ch in chapter_nums:
        write_chapter(chapters_dir, ch, chapters, scene_bodies)


if __name__ == "__main__":
    sys.exit(main())
