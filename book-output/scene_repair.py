#!/usr/bin/env python3
"""
scene_repair.py — targeted second-pass surgery for scenes that came
out of the main `scene_surgery.py` run with specific content defects.

Used when `consistency_check.py` flags scene-level issues that need
more than a stylistic rewrite — e.g.:

  - A scene was rewritten into the wrong setting (e.g., ch5.s2 ended
    up in the Mumbai office instead of the temple town).
  - A scene duplicates another scene's content (model riffed on the
    wrong reference).
  - A scene is severely off-beat (synopsis coverage 0%).

Each repair task specifies the scene to fix and an explicit
"REPAIR INSTRUCTION" injected into the rewrite prompt above the
existing constraints. The model uses the current draft as raw
material but must obey the repair instruction.

Hand-curated repair list lives in REPAIRS at the bottom of the file —
edit it to add new repair tasks as the consistency check surfaces them.
"""

from __future__ import annotations

import argparse
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from book_helpers import chat_no_thinking  # type: ignore
from scene_surgery import (  # type: ignore
    MODEL_HEAVY,
    CANON_CHARACTERS,
    CANON_SETTINGS,
    CANON_MOTIFS,
    collect_all_scenes,
    first_paragraph,
    last_paragraph,
    load_outline,
    story_so_far_summary,
    trim_input_for_bloat,
    write_chapter,
)


def build_repair_prompts(
    *,
    brief: dict,
    chapter: dict,
    chapter_idx: int,
    scene_idx: int,
    current_draft: str,
    prev_tail: str,
    next_head: str,
    story_so_far: str,
    repair_instruction: str,
    forbidden_openers: list[str],
) -> tuple[str, str]:
    scene = chapter["scenes"][scene_idx - 1]
    target = int(scene.get("target_word_count") or 1200)
    synopsis = scene.get("synopsis", "")
    purpose = chapter.get("purpose", "")
    beat = scene.get("beat", "")

    forbidden_block = ""
    if forbidden_openers:
        forbidden_block = "\nFORBIDDEN OPENERS — your scene must NOT open with any of these sentences (they are already used by other scenes):\n"
        for i, opener in enumerate(forbidden_openers, 1):
            forbidden_block += f"  ({i}) {opener.strip()[:200]}…\n"

    system = f"""You are a literary fiction editor performing TARGETED REPAIR on one scene of a novella titled "{(brief.get("title_suggestions") or ["My Confused Life"])[0]}".

The scene below was rewritten in a previous pass and has a specific defect described in the REPAIR INSTRUCTION. Your job is to produce a corrected version that obeys both the repair instruction AND every absolute constraint.

ABSOLUTE CONSTRAINTS — every one is a publication-blocker:

1. POV: FIRST-PERSON PAST TENSE. The narrator is the protagonist, Arjun. Use "I", "me", "my". DO NOT write "Arjun did X", "he felt Y" in narration. Other characters may name Arjun in dialogue.

2. LENGTH: target ~{target} words. Acceptable range: {int(target * 0.85)}–{int(target * 1.15)}. Hard ceiling: {int(target * 1.2)}.

3. NO COMMA-SPLICED ACTION LISTS. Each clause must be a complete sentence.

4. NO REPETITION. Within this scene, no sentence may appear twice. Across the book, do not duplicate another scene's opening sentence.

5. STAY ON BEAT. Dramatise THIS scene's synopsis and the REPAIR INSTRUCTION above all else.

6. CONTINUITY. Open in a way that follows the previous-scene tail; close in a way that hands off to the next-scene head.

7. NO AI TELLS, NO FORBIDDEN TROPES (chosen-one, saintly-guru, romance-as-rescue, "the truth was…", three-adjective stacks).

8. OUTPUT FORMAT: prose only, paragraphs separated by blank lines. No headings, scene labels, or commentary.
"""

    user = f"""BOOK CONTEXT:

CHARACTERS:
{CANON_CHARACTERS}
SETTINGS:
{CANON_SETTINGS}
RECURRING MOTIFS:
{CANON_MOTIFS}

STORY SO FAR:
{story_so_far}

────────────────────────────────────────────────────────────────────

REPAIR INSTRUCTION (highest priority — overrides everything else):
{repair_instruction}
{forbidden_block}
────────────────────────────────────────────────────────────────────

CHAPTER {chapter_idx} PURPOSE:
{purpose}

SCENE {scene_idx} BEAT: {beat}

SCENE {scene_idx} SYNOPSIS (what this scene must dramatise):
{synopsis}

PREVIOUS-SCENE TAIL:
{prev_tail or "[OPENING OF BOOK — start cold]"}

NEXT-SCENE HEAD:
{next_head or "[END OF BOOK — close definitively]"}

CURRENT DRAFT OF THIS SCENE (may be wrong; rewrite per REPAIR INSTRUCTION):
---
{current_draft}
---

Write the repaired scene now. First-person past tense. ~{target} words. Plain paragraphs. Obey the REPAIR INSTRUCTION.
"""
    return system, user


def repair_scene(
    *,
    brief: dict,
    chapters: list[dict],
    chapter_idx: int,
    scene_idx: int,
    scene_bodies: dict,
    repair_instruction: str,
    forbidden_openers: list[str],
) -> str:
    chapter = chapters[chapter_idx - 1]
    scene = chapter["scenes"][scene_idx - 1]
    target = int(scene.get("target_word_count") or 1200)

    current = scene_bodies[(chapter_idx, scene_idx)]
    all_keys = sorted(scene_bodies.keys())

    prev_key = None
    if scene_idx > 1:
        prev_key = (chapter_idx, scene_idx - 1)
    elif chapter_idx > 1:
        prev = [k for k in all_keys if k[0] == chapter_idx - 1]
        prev_key = prev[-1] if prev else None
    next_key = None
    if (chapter_idx, scene_idx + 1) in scene_bodies:
        next_key = (chapter_idx, scene_idx + 1)
    elif (chapter_idx + 1, 1) in scene_bodies:
        next_key = (chapter_idx + 1, 1)

    prev_tail = last_paragraph(scene_bodies[prev_key]) if prev_key else ""
    next_head = first_paragraph(scene_bodies[next_key]) if next_key else ""
    story = story_so_far_summary(chapters, chapter_idx, scene_idx, scene_bodies)

    system, user = build_repair_prompts(
        brief=brief,
        chapter=chapter,
        chapter_idx=chapter_idx,
        scene_idx=scene_idx,
        current_draft=trim_input_for_bloat(current),
        prev_tail=prev_tail,
        next_head=next_head,
        story_so_far=story,
        repair_instruction=repair_instruction,
        forbidden_openers=forbidden_openers,
    )
    max_tok = int(target * 2.0) + 256
    out, _meta = chat_no_thinking(
        MODEL_HEAVY, system, user,
        temperature=0.45, max_tokens=max_tok,
    )
    return out.strip()


# ── Repair task list ──────────────────────────────────────────────────────
# Each entry: (chapter, scene, repair_instruction, [keys whose openings
# this scene must NOT duplicate]).

REPAIRS: list[tuple[int, int, str, list[tuple[int, int]]]] = [
    # ch1.s3: outline says "Arjun receives news of his mentor's death,
    # a moment that shatters his remaining sense of professional order".
    # The current rewrite drifts off this beat.
    (
        1, 3,
        "This scene MUST dramatise the news of Mr. Das's death and how it reaches Arjun. The phone rings, the call comes, or someone tells him in person — pick one and stay with it. The beat is the *catalyst event*: this is the moment that fractures Arjun's professional self-image. Show:\n"
        "  - the moment Arjun learns (a specific scene — not a montage)\n"
        "  - the small physical detail he fixates on (a coffee ring on the desk, the cursor still blinking, the cardigan smell that lingers)\n"
        "  - the failure to react in the way work would have him react\n"
        "  - the closing image: Arjun stepping away from the office (toward the elevator, toward the car, toward something he cannot yet name)\n"
        "Stay in the corporate setting (Mumbai, office tower). End the scene before Arjun arrives anywhere else.",
        [],
    ),
    # ch3.s2: outline beat is "Observation of icons" — Arjun watching
    # Radha and Krishna iconography. The current rewrite starts with a
    # paragraph identical to ch3.s1's opener.
    (
        3, 2,
        "OPEN this scene with Arjun ALREADY INSIDE the temple — past the threshold, no bell, no brass cord, no entrance description. The opening sentence should be about WHAT HE SEES on the altar (the iconography, the candles, the deity). The previous scene already covered the entry. This scene is about *observation*: how Arjun studies the painted faces of Radha and Krishna, notes the imperfect symmetry, the kohl, the vermilion, the candles, and tries to read meaning into them with his corporate-analyst eye. It ends just before the chant starts (the chant is the next scene's beat).",
        [(3, 1)],
    ),
    # ch5.s2: outline beat is "Internal integration" — Arjun begins
    # using devotional names in daily life. The current rewrite drifts
    # back to the Mumbai corporate office, which is the wrong setting.
    (
        5, 2,
        "This scene is set in the TEMPLE TOWN (the hinterland town from chapter 3 onward), NOT in the Mumbai office. Arjun is staying in the town, not commuting back. The beat is *internal integration*: he begins to use the names of Radha and Krishna in his daily moments — walking the lanes, eating his morning meal at a small tea stall, watching the dawn over the temple roof, doing some small chore. The names soften his old corporate-anxiety reflexes. Show this with specific, small daily images: the cup of tea, the worn step, the lane vendor, the dog at the temple gate. ABSOLUTELY DO NOT put Arjun at a computer, in a boardroom, near a spreadsheet, or near Elena. Do not mention Mr. Das, the merger, or the office.",
        [(2, 1), (5, 1)],
    ),
    # ch5.s3: outline beat is "Epiphany" — Arjun realises peace is a
    # by-product of service. The current rewrite duplicates ch5.s1's
    # kirtan-bench opening.
    (
        5, 3,
        "This scene is the EPIPHANY of Part II. Arjun, at the end of an ordinary day in the temple town, has the realisation that he has been chasing the wrong thing — peace is not a destination, but the side-effect of giving. The scene should NOT replay the kirtan-bench setting of ch5.s1. Choose a quieter setting: late evening on the temple steps, or a walk back to his lodging through the empty lanes, or a moment alone in the dormitory with the lamp turned low. The realisation must land *without dialogue*, *without a guru figure stating it*. It comes through Arjun's own thinking, perhaps as he remembers the day's small services (a chore for the kitchen, a meal he helped serve, an old woman he helped onto a bench). Close with a sentence that signals he is changed but does not announce it.",
        [(5, 1)],
    ),
    # ch4.s1: outline beat is "Clumsy service" — Arjun's first attempt
    # at serving food. Current rewrite has the sentence "I picked up
    # the ladle again" three times.
    (
        4, 1,
        "Eliminate repetition in this scene. The sentence \"I picked up the ladle again\" appears multiple times in the current draft — that is repetition, not rhythm. Each pickup of the ladle should be a different beat (first contact / a misstep / a recovery / the final pour). Use varied physical detail (the wood handle, the curved bowl, the weight of the lentils, the steam) so each beat is distinct. Otherwise, keep the scene's content: Arjun's first volunteer shift in the temple kitchen, his hands shaking, the contrast between his corporate poise and the simplicity of the task.",
        [],
    ),
    # ch4.s3: outline beat is "Subtle shift" — Arjun feels a small
    # lightness after his first service. Current rewrite repeats "I
    # wanted to be anywhere else" twice.
    (
        4, 3,
        "Eliminate the duplicate sentence \"I wanted to be anywhere else\" — it appears twice in the current draft. Keep only the first occurrence; replace the second with a different beat (a small physical observation, or a moment of quiet that contradicts the want-to-leave). Otherwise keep the scene: the very small, almost-imperceptible sense of lightness Arjun feels after the food has been distributed. The beat is restraint — he cannot yet articulate why he feels lighter, so neither should the prose.",
        [],
    ),
    # ch6.s1: outline beat is "Return to city" — Arjun walks back into
    # Mumbai. Current rewrite repeats one introspective sentence three
    # times and is 67% over target.
    (
        6, 1,
        "Cut the scene to ~1200 words. Eliminate repetition: the sentence \"I asked the question in the hollow space behind my sternum, where the anxiety usually lived\" or close variants appear multiple times — keep only the first instance. The beat is Arjun's RETURN to Mumbai traffic after the temple-town arc: he sees the city as it is, but feels himself a participant in its flow rather than a ghost. Show one specific image (the rickshaw, the office tower facade, a familiar street vendor) and end the scene with him walking toward his apartment — but do not enter it (that is the next scene).",
        [],
    ),
    # ch1.s1: minor repeat — "The rickshaw stopped at a red light."
    (
        1, 1,
        "Eliminate the duplicate sentence \"The rickshaw stopped at a red light.\" — it appears twice in the current draft. Keep one instance; replace the second with a different progression (the light changes, traffic moves, the rickshaw lurches forward, etc.). Otherwise keep the scene exactly as it is.",
        [],
    ),
]


def main() -> int:
    p = argparse.ArgumentParser()
    p.add_argument("book_dir", nargs="?", default="my-confused-life")
    p.add_argument("--scenes", help="comma-separated list ch1.s1,ch5.s2,... (default: all REPAIRS)")
    p.add_argument("--dry-run", action="store_true")
    args = p.parse_args()

    book_dir = Path(args.book_dir)
    if not book_dir.is_absolute():
        book_dir = Path(__file__).resolve().parent / book_dir
    chapters_dir = book_dir / "canonical" / "chapters"

    brief, chapters = load_outline(book_dir)
    scene_bodies = collect_all_scenes(chapters_dir)

    if args.scenes:
        import re
        wanted: set[tuple[int, int]] = set()
        for token in args.scenes.split(","):
            m = re.match(r"ch(\d+)\.s(\d+)", token.strip().lower())
            if not m:
                raise ValueError(f"bad spec: {token!r}")
            wanted.add((int(m.group(1)), int(m.group(2))))
        tasks = [r for r in REPAIRS if (r[0], r[1]) in wanted]
    else:
        tasks = REPAIRS

    print(f"book_dir:    {book_dir}")
    print(f"repairs:     {len(tasks)}")
    print(f"model:       {MODEL_HEAVY}")

    if args.dry_run:
        for ch, sc, instr, forb in tasks:
            print(f"  would repair ch{ch}.s{sc}: {instr[:80]}…")
            for k in forb:
                print(f"    forbidden opener: ch{k[0]}.s{k[1]}")
        return 0

    log_path = book_dir / "canonical" / "scene-repair.log"
    log_lines: list[str] = []

    def log(msg: str) -> None:
        ts = time.strftime("%H:%M:%S")
        line = f"[{ts}] {msg}"
        print(line, flush=True)
        log_lines.append(line)
        log_path.write_text("\n".join(log_lines))

    started = time.time()
    for ch, sc, instruction, forbidden_keys in tasks:
        if (ch, sc) not in scene_bodies:
            log(f"!! ch{ch}.s{sc} not present — skipping")
            continue
        forbidden_openers = [
            first_paragraph(scene_bodies[k]) for k in forbidden_keys if k in scene_bodies
        ]
        target = int(chapters[ch - 1]["scenes"][sc - 1].get("target_word_count") or 1200)
        before_wc = len(scene_bodies[(ch, sc)].split())
        log(f"ch{ch}.s{sc} — target {target}w, current {before_wc}w  →  repairing via {MODEL_HEAVY}...")
        t0 = time.time()
        try:
            new_body = repair_scene(
                brief=brief,
                chapters=chapters,
                chapter_idx=ch,
                scene_idx=sc,
                scene_bodies=scene_bodies,
                repair_instruction=instruction,
                forbidden_openers=forbidden_openers,
            )
        except Exception as e:  # noqa: BLE001
            log(f"  !! ch{ch}.s{sc} failed: {type(e).__name__}: {e}")
            continue
        after_wc = len(new_body.split())
        elapsed = time.time() - t0
        log(f"  done in {elapsed:.1f}s  →  {after_wc} words  (target {target}, drift {after_wc - target:+d})")

        scene_bodies[(ch, sc)] = new_body
        write_chapter(chapters_dir, ch, chapters, scene_bodies)

    log(f"\nrepairs complete in {(time.time() - started) / 60:.1f} min")
    log("Next: ./.venv/bin/python consistency_check.py my-confused-life --gate")
    return 0


if __name__ == "__main__":
    sys.exit(main())
