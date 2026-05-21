#!/usr/bin/env python3
"""
outline_to_scenes.py — one-off data adapter.

Reads `<book-dir>/02-outline.json` and emits a Rust SCENES[] array
literal ready to paste into:

  booksforge/crates/booksforge-orchestrator/examples/multi_chapter_run.rs

This is NOT a pipeline. It is a data conversion that runs once: the
canonical Rust example expects each scene as {goal, conflict, reveal}
triples, while our outline has only synopses. We use the LIGHT model
(qwen3.5:9b) to do the one-shot decomposition per scene.

Output is written to: `<book-dir>/canonical/SCENES.rs.txt`
"""

from __future__ import annotations

import json
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from booksforge_ollama_driver import REPO, extract_json
from book_helpers import chat_no_thinking  # type: ignore


MODEL_LIGHT = "qwen3.5:9b"


def decompose_scene(synopsis: str, chapter_purpose: str, target_words: int) -> dict:
    system = (
        "You decompose a one-line scene synopsis into a structured scene card.\n"
        "Return JSON only, schema: {\"goal\": string, \"conflict\": string, "
        "\"reveal\": string}.\n\n"
        "  - goal:     What the protagonist does in this scene. One sentence, "
        "active voice, no editorial framing.\n"
        "  - conflict: The internal or external resistance to the goal. One sentence.\n"
        "  - reveal:   The thing the reader learns or the moment that shifts. "
        "One sentence.\n\n"
        "Stay inside the synopsis. Do not invent new plot or characters."
    )
    user = (
        f"CHAPTER PURPOSE: {chapter_purpose}\n"
        f"SCENE SYNOPSIS:  {synopsis}\n"
        f"TARGET WORDS:    {target_words}\n\n"
        "Return the JSON object."
    )
    out, _meta = chat_no_thinking(
        MODEL_LIGHT, system, user,
        temperature=0.3, max_tokens=512, json_mode=True,
    )
    return extract_json(out)


def escape_rust_str(s: str) -> str:
    return s.replace("\\", "\\\\").replace('"', '\\"')


def main() -> int:
    args = sys.argv[1:]
    book_dir = Path(args[0]) if args else (REPO / "book-output/my-confused-life")
    if not book_dir.is_absolute():
        book_dir = Path(__file__).resolve().parent / book_dir

    brief = json.loads((book_dir / "01-brief.json").read_text())
    outline = json.loads((book_dir / "02-outline.json").read_text())

    out_dir = book_dir / "canonical"
    out_dir.mkdir(exist_ok=True)
    out_path = out_dir / "SCENES.rs.txt"
    cards_path = out_dir / "scene-cards.json"

    started = time.time()
    cards: list[dict] = []
    chapters: list[dict] = []
    for part in outline.get("parts", []):
        for ch in part.get("chapters", []):
            chapters.append(ch)

    print(f"Decomposing {sum(len(c.get('scenes', [])) for c in chapters)} scenes "
          f"across {len(chapters)} chapters via {MODEL_LIGHT}...")

    for ci, ch in enumerate(chapters, start=1):
        ch_purpose = ch.get("purpose", "")
        for si, scene in enumerate(ch.get("scenes", []), start=1):
            synopsis = scene.get("synopsis", "").strip()
            target = int(scene.get("target_word_count") or 1200)
            t0 = time.time()
            try:
                triple = decompose_scene(synopsis, ch_purpose, target)
            except Exception as e:  # noqa: BLE001
                print(f"  ch{ci:02d}-s{si:02d}: decompose failed: {e}", flush=True)
                triple = {"goal": synopsis, "conflict": "", "reveal": ""}
            title = scene.get("title", f"Scene {si}").strip()
            elapsed = time.time() - t0
            cards.append({
                "chapter": ci,
                "scene": si,
                "title": title,
                "synopsis": synopsis,
                "target_words": target,
                "goal": triple.get("goal", "").strip(),
                "conflict": triple.get("conflict", "").strip(),
                "reveal": triple.get("reveal", "").strip(),
            })
            print(f"  ch{ci:02d}-s{si:02d}: {title!r} ({elapsed:.1f}s)", flush=True)

    cards_path.write_text(json.dumps(cards, indent=2))

    # Emit Rust array literal — drop straight into multi_chapter_run.rs.
    lines = ["const SCENES: &[SceneSpec] = &["]
    for c in cards:
        lines.append("    SceneSpec {")
        lines.append(f"        chapter:  {c['chapter']},")
        lines.append(f"        scene:    {c['scene']},")
        lines.append(f"        title:    \"{escape_rust_str(c['title'])}\",")
        lines.append(f"        goal:     \"{escape_rust_str(c['goal'])}\",")
        lines.append(f"        conflict: \"{escape_rust_str(c['conflict'])}\",")
        lines.append(f"        reveal:   \"{escape_rust_str(c['reveal'])}\",")
        lines.append("    },")
    lines.append("];")
    out_path.write_text("\n".join(lines) + "\n")

    # Also emit the IDEA_TEXT constant from the brief premise.
    premise = brief.get("premise", "").strip().replace("\n", " ")
    while "  " in premise:
        premise = premise.replace("  ", " ")
    idea_path = out_dir / "IDEA_TEXT.rs.txt"
    idea_path.write_text(f'const IDEA_TEXT: &str = "{escape_rust_str(premise)}";\n')

    print(f"\nDone in {round((time.time() - started) / 60, 1)} min")
    print(f"  Scene cards: {cards_path}")
    print(f"  Rust SCENES: {out_path}")
    print(f"  Rust IDEA:   {idea_path}")
    print(f"\nNext step: paste the contents of those two .rs.txt files into")
    print(f"  booksforge/crates/booksforge-orchestrator/examples/multi_chapter_run.rs")
    return 0


if __name__ == "__main__":
    sys.exit(main())
