#!/usr/bin/env python3
"""
book_review.py — Stage 1 of the publication-readiness loop.

Runs a developmental editor over every chapter of a completed book
and emits a structured findings document the next stage acts on.

Pipeline:
  1. Read brief from 01-brief.json.
  2. Build prior_chapter_summaries (derived from chapter purposes
     in 02-outline.json — the orchestrator's own summary substitute).
  3. For each chapter:
       call dev-editor/v1 → DevelopmentalNotes JSON (per the locked
       schema). 6 axes: pacing, stakes, character, pov_tension,
       theme, structural_balance.
  4. Compute mechanical metrics across the book:
       - dialogue density per chapter
       - sentence-opener "I" rate per chapter
       - first-line-of-paragraph repetition
       - templated metaphor count (recurring image-clichés)
  5. Merge into book-review.json — both the LLM editorial notes and
     the mechanical metrics. This file is the contract Stage 2
     (targeted_redraft.py) consumes.

Default model: qwen3.5:27b. qwen3.6 is the user's largest local
model and is a valid alternative; pass it as `--model qwen3.6:latest`.

Usage:
    python book_review.py [book-dir] [--model qwen3.5:27b]
"""

from __future__ import annotations

import argparse
import json
import re
import statistics
import sys
import time
from pathlib import Path
from collections import Counter

sys.path.insert(0, str(Path(__file__).parent))
from booksforge_ollama_driver import REPO, extract_json, ollama_probe, render_template
from book_helpers import CREATIVE_PROFILE, chat_no_thinking  # type: ignore


# ── Mechanical metrics (no LLM) ───────────────────────────────────────────


def _sentences(text: str) -> list[str]:
    # Light sentence split that respects ?, !, .
    return [s.strip() for s in re.split(r"(?<=[.!?])\s+", text) if len(s.strip()) > 4]


def chapter_metrics(chapter_text: str) -> dict:
    """Mechanical craft signals the dev-editor LLM doesn't compute well.

    These are what flagged the dialogue/voice issues in the manuscript
    audit. The LLM editor will catch *interpretive* issues (theme,
    character arc); these metrics catch *measurable* ones (dialogue
    density, sentence-opener monotony).
    """
    # Strip leading heading
    body = re.sub(r"^##.*$", "", chapter_text, count=1, flags=re.MULTILINE).strip()
    words = body.split()
    sents = _sentences(body)

    # Dialogue lines — count straight or smart double-quotes that contain text.
    dialogue_lines = len(re.findall(r'(?:"[^"]+")|(?:"[^"]+")', body))

    # "I"-opener rate
    i_open = sum(
        1 for s in sents
        if (s.split() or [""])[0].rstrip(",.;:") in ("I", "I'd", "I've", "I'll", "I'm")
    )
    i_open_rate = i_open / max(len(sents), 1)

    # Sentence-length variability
    sent_lens = [len(s.split()) for s in sents]
    stdev = statistics.stdev(sent_lens) if len(sent_lens) >= 2 else 0.0

    # Templated imagery — count recurring N-grams that suggest cliché
    # patterns. We look for these specific phrases the audit flagged.
    image_cliches = [
        "dust motes",
        "ghost in",
        "ghost haunting",
        "drop of water",
        "drop in the ocean",
        "soft, indistinct",
        "the cursor blinked",
        "the weight of",
        "I felt a strange",
        "a kind of peace",
    ]
    cliche_hits = sum(body.lower().count(phrase.lower()) for phrase in image_cliches)

    return {
        "word_count": len(words),
        "sentence_count": len(sents),
        "dialogue_lines": dialogue_lines,
        "dialogue_per_1000_words": round(dialogue_lines / max(len(words), 1) * 1000, 1),
        "i_opener_rate_pct": round(i_open_rate * 100, 1),
        "sentence_length_stdev": round(stdev, 1),
        "cliche_phrase_hits": cliche_hits,
    }


# ── dev-editor invocation ─────────────────────────────────────────────────


def review_chapter(*, model: str, chapter_id: str, chapter_text: str, brief: dict,
                   prior_summaries: list[dict], log) -> dict:
    """Run dev-editor/v1 on one chapter; return parsed DevelopmentalNotes."""
    rendered = render_template(
        "dev-editor",
        {
            "chapter_id": chapter_id,
            "chapter_text": chapter_text,
            "project_brief": brief,
            "prior_chapter_summaries": prior_summaries,
            "known_entities": [],
            "prompt_guard": "",
            "creative_profile": CREATIVE_PROFILE,
        },
    )
    # The chapter + brief together rarely exceed 6-8k tokens. Allow a
    # generous output budget for 25 notes worth of editorial JSON.
    out, meta = chat_no_thinking(
        model, rendered.system, rendered.user,
        temperature=0.3, max_tokens=8192, json_mode=True,
    )
    notes = extract_json(out)
    log(f"     dev-editor returned {len(notes.get('notes', []))} notes "
        f"({meta['elapsed_s']}s, {meta['eval_count']} tokens)")
    return notes


# ── Main ──────────────────────────────────────────────────────────────────


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("book_dir", nargs="?", default="my-confused-life")
    parser.add_argument("--model", default="qwen3.5:27b",
                        help="LLM for the dev-editor pass (default qwen3.5:27b)")
    args = parser.parse_args()

    root = Path(args.book_dir)
    if not root.is_absolute():
        root = Path(__file__).resolve().parent / root
    brief_path = root / "01-brief.json"
    chapters_dir = root / "chapters"
    if not brief_path.exists() or not chapters_dir.is_dir():
        print(f"missing {brief_path} or {chapters_dir}", file=sys.stderr)
        return 2

    log_lines: list[str] = []
    log_path = root / "book-review.log"

    def log(msg: str) -> None:
        ts = time.strftime("%H:%M:%S")
        line = f"[{ts}] {msg}"
        print(line, flush=True)
        log_lines.append(line)
        log_path.write_text("\n".join(log_lines))

    log("=" * 72)
    log(f"BooksForge book-review pass | model={args.model}")
    log(f"Template: dev-editor/v1")
    log(f"Root:     {root}")
    log("=" * 72)

    probe = ollama_probe()
    if args.model not in probe["models"]:
        log(f"FATAL: model '{args.model}' not pulled locally.")
        return 1

    brief = json.loads(brief_path.read_text())
    # If we have a chapters-v2 dir, prefer revised chapter text for review.
    chapters_v2_dir = root / "chapters-v2"
    use_v2 = chapters_v2_dir.is_dir() and any(chapters_v2_dir.glob("chapter-*.md"))
    if use_v2:
        log("Using chapters-v2/ (revised text) for review")
    else:
        log("Using chapters/ (original text) for review")

    chapter_files = sorted(chapters_dir.glob("chapter-*.md"))
    review_results: list[dict] = []
    prior_summaries: list[dict] = []

    started = time.time()
    for i, ch_path in enumerate(chapter_files, start=1):
        log("")
        active = (chapters_v2_dir / ch_path.name) if use_v2 and (chapters_v2_dir / ch_path.name).exists() else ch_path
        log(f"── Chapter {i:02d} ({active.relative_to(root)})")
        text = active.read_text()
        # Strip the heading for the metrics calculation; pass full text
        # to the LLM (the dev-editor handles structure).
        metrics = chapter_metrics(text)
        log(f"     metrics: dialogue/1000w={metrics['dialogue_per_1000_words']}  "
            f"I-open%={metrics['i_opener_rate_pct']}  cliche_hits={metrics['cliche_phrase_hits']}")

        chapter_id = f"ch{i:02d}"
        try:
            notes = review_chapter(
                model=args.model,
                chapter_id=chapter_id,
                chapter_text=text,
                brief=brief,
                prior_summaries=prior_summaries,
                log=log,
            )
        except Exception as e:  # noqa: BLE001
            log(f"     !! dev-editor failed: {type(e).__name__}: {str(e)[:200]}")
            notes = {"chapter_id": chapter_id, "notes": [], "summary": f"[review failed: {e}]"}

        # Compose a chapter summary for the prior_summaries pipeline —
        # taken from the dev-editor's own summary if present, else from
        # the outline's chapter purpose.
        summary = notes.get("summary") or ""
        prior_summaries.append({"chapter_id": chapter_id, "summary": summary[:400]})

        review_results.append({
            "chapter_id": chapter_id,
            "chapter_path": str(active.relative_to(root)),
            "metrics": metrics,
            "dev_editor_notes": notes,
        })

        # Persist progress every chapter — safe under interruption.
        (root / "book-review.json").write_text(json.dumps({
            "model": args.model,
            "templates": ["dev-editor/v1"],
            "completed_chapters": i,
            "total_chapters": len(chapter_files),
            "wall_clock_s": round(time.time() - started, 1),
            "chapters": review_results,
        }, indent=2))

    # Cross-book metrics aggregation.
    cross_book = {
        "total_words": sum(c["metrics"]["word_count"] for c in review_results),
        "total_dialogue_lines": sum(c["metrics"]["dialogue_lines"] for c in review_results),
        "dialogue_per_1000_words": round(
            sum(c["metrics"]["dialogue_lines"] for c in review_results) /
            max(sum(c["metrics"]["word_count"] for c in review_results), 1) * 1000, 1
        ),
        "avg_i_opener_rate_pct": round(
            statistics.mean(c["metrics"]["i_opener_rate_pct"] for c in review_results), 1
        ),
        "chapters_with_zero_dialogue": [
            c["chapter_id"] for c in review_results if c["metrics"]["dialogue_lines"] == 0
        ],
        "total_cliche_hits": sum(c["metrics"]["cliche_phrase_hits"] for c in review_results),
    }

    final = {
        "model": args.model,
        "templates": ["dev-editor/v1"],
        "wall_clock_s": round(time.time() - started, 1),
        "cross_book_metrics": cross_book,
        "chapters": review_results,
    }
    (root / "book-review.json").write_text(json.dumps(final, indent=2))

    log("")
    log("=" * 72)
    log(f"DONE. wall-clock {round((time.time() - started) / 60, 1)} min")
    log(f"Cross-book metrics:")
    log(f"  total words:            {cross_book['total_words']}")
    log(f"  dialogue/1000 words:    {cross_book['dialogue_per_1000_words']}")
    log(f"  avg I-opener rate %:    {cross_book['avg_i_opener_rate_pct']}")
    log(f"  zero-dialogue chapters: {cross_book['chapters_with_zero_dialogue']}")
    log(f"Report saved → {root / 'book-review.json'}")
    log("=" * 72)
    return 0


if __name__ == "__main__":
    sys.exit(main())
