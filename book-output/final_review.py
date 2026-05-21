#!/usr/bin/env python3
"""
final_review.py — Stage 3 of the publication-readiness loop.

Reads the revised (v2) chapter text and emits two artifacts:

  1. publication-readiness.json — a structured scorecard:
       - per-chapter metrics (dialogue density, I-opener rate, …)
       - per-chapter dev-editor pass scores (re-run on v2 text)
       - a delta block comparing v1 vs v2 metrics
       - an overall publication-readiness verdict + grade

  2. PUBLICATION_READINESS.md — the human-readable version of the
     above, suitable for showing the author alongside the manuscript.

This stage does NOT modify the manuscript. It is a measurement and
scoring pass only.

Usage:
    python final_review.py [book-dir] [--model qwen3.5:27b]
"""

from __future__ import annotations

import argparse
import json
import statistics
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from booksforge_ollama_driver import ollama_probe
from book_review import chapter_metrics, review_chapter  # type: ignore


GRADING = [
    # min_score, label, description
    (8.5, "A — publishable as-is", "Strong debut quality. Send to agents."),
    (7.5, "B+ — close, one more pass", "Minor revisions; high-leverage edits only."),
    (6.5, "B — viable with revision", "Real bones; a structural pass would lift it."),
    (5.5, "C — competent draft", "Workable foundation; significant craft work to publish."),
    (4.5, "D — early draft", "Structural and craft issues need sustained revision."),
    (0.0, "F — needs new draft", "More gap than bones; consider a from-scratch pass."),
]


def grade(score: float) -> tuple[str, str]:
    for threshold, label, desc in GRADING:
        if score >= threshold:
            return label, desc
    return GRADING[-1][1], GRADING[-1][2]


def compute_score(metrics_per_chapter: list[dict]) -> dict:
    """Translate raw metrics into a 0-10 publication-readiness sub-scores.

    The sub-scoring is intentionally simple and traceable to the user's
    own diagnostic criteria — dialogue density, I-opener variety, image
    freshness, voice variability.
    """
    avg_dialogue_per_1000 = statistics.mean(
        m["dialogue_per_1000_words"] for m in metrics_per_chapter
    )
    avg_i_open = statistics.mean(m["i_opener_rate_pct"] for m in metrics_per_chapter)
    total_cliche = sum(m["cliche_phrase_hits"] for m in metrics_per_chapter)
    zero_dialogue_chapters = sum(1 for m in metrics_per_chapter if m["dialogue_lines"] == 0)
    avg_sent_stdev = statistics.mean(m["sentence_length_stdev"] for m in metrics_per_chapter)

    # Dialogue: literary-fiction healthy zone is ~10-30 lines / 1000 words.
    # Score linearly between those.
    if avg_dialogue_per_1000 >= 25:
        s_dialogue = 9.5
    elif avg_dialogue_per_1000 >= 15:
        s_dialogue = 8.0
    elif avg_dialogue_per_1000 >= 10:
        s_dialogue = 6.5
    elif avg_dialogue_per_1000 >= 5:
        s_dialogue = 5.0
    elif avg_dialogue_per_1000 >= 2:
        s_dialogue = 3.5
    else:
        s_dialogue = 1.5
    # Penalise zero-dialogue chapters explicitly.
    s_dialogue = max(0.0, s_dialogue - zero_dialogue_chapters * 0.5)

    # I-opener variety: <18% is healthy; >30% is poor.
    if avg_i_open <= 18:
        s_voice = 9.0
    elif avg_i_open <= 25:
        s_voice = 7.0
    elif avg_i_open <= 32:
        s_voice = 5.0
    elif avg_i_open <= 40:
        s_voice = 3.0
    else:
        s_voice = 1.5

    # Cliché freshness: 0-2 hits per book is publishable; >10 is bad.
    if total_cliche <= 2:
        s_imagery = 9.0
    elif total_cliche <= 5:
        s_imagery = 7.5
    elif total_cliche <= 10:
        s_imagery = 5.5
    elif total_cliche <= 20:
        s_imagery = 3.0
    else:
        s_imagery = 1.5

    # Sentence-length variability: stdev 9-13 is healthy.
    if 9 <= avg_sent_stdev <= 13:
        s_rhythm = 8.5
    elif 7 <= avg_sent_stdev < 9 or 13 < avg_sent_stdev <= 15:
        s_rhythm = 7.0
    else:
        s_rhythm = 5.0

    overall = round(statistics.mean([s_dialogue, s_voice, s_imagery, s_rhythm]), 1)

    return {
        "dialogue": round(s_dialogue, 1),
        "voice_variety": round(s_voice, 1),
        "imagery_freshness": round(s_imagery, 1),
        "rhythm": round(s_rhythm, 1),
        "overall_mechanical": overall,
        "evidence": {
            "avg_dialogue_per_1000_words": round(avg_dialogue_per_1000, 1),
            "avg_i_opener_rate_pct": round(avg_i_open, 1),
            "total_cliche_phrase_hits": total_cliche,
            "zero_dialogue_chapters": zero_dialogue_chapters,
            "avg_sentence_length_stdev": round(avg_sent_stdev, 1),
        },
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("book_dir", nargs="?", default="my-confused-life")
    parser.add_argument("--model", default="qwen3.5:27b")
    parser.add_argument("--skip-llm", action="store_true",
                        help="Compute metrics only (skip dev-editor re-pass)")
    args = parser.parse_args()

    root = Path(args.book_dir)
    if not root.is_absolute():
        root = Path(__file__).resolve().parent / root

    log_lines: list[str] = []
    log_path = root / "final-review.log"

    def log(msg: str) -> None:
        ts = time.strftime("%H:%M:%S")
        line = f"[{ts}] {msg}"
        print(line, flush=True)
        log_lines.append(line)
        log_path.write_text("\n".join(log_lines))

    log("=" * 72)
    log(f"BooksForge final-review (publication readiness)")
    log("=" * 72)

    if not args.skip_llm:
        probe = ollama_probe()
        if args.model not in probe["models"]:
            log(f"FATAL: model '{args.model}' not pulled.")
            return 1

    # Prefer v2 chapters; fall back to v1 if v2 isn't present yet.
    v2_dir = root / "chapters-v2"
    v1_dir = root / "chapters"
    chapter_files_v1 = sorted(v1_dir.glob("chapter-*.md"))
    if not chapter_files_v1:
        log(f"FATAL: no chapters in {v1_dir}")
        return 2

    brief = json.loads((root / "01-brief.json").read_text())

    v1_metrics = [chapter_metrics(p.read_text()) for p in chapter_files_v1]
    v2_metrics = []
    for p in chapter_files_v1:
        v2_path = v2_dir / p.name
        if v2_path.exists():
            v2_metrics.append(chapter_metrics(v2_path.read_text()))
        else:
            v2_metrics.append(None)
    has_any_v2 = any(m is not None for m in v2_metrics)

    log(f"v1 chapters: {len(chapter_files_v1)}  |  v2 chapters: {sum(1 for m in v2_metrics if m)}")

    # ── LLM dev-editor pass on the final text (v2 where available) ──
    chapter_reviews = []
    started = time.time()
    if not args.skip_llm:
        prior_summaries = []
        for i, p in enumerate(chapter_files_v1, start=1):
            active = (v2_dir / p.name) if (v2_dir / p.name).exists() else p
            log(f"  dev-editor on {active.relative_to(root)} ...")
            text = active.read_text()
            try:
                notes = review_chapter(
                    model=args.model,
                    chapter_id=f"ch{i:02d}",
                    chapter_text=text,
                    brief=brief,
                    prior_summaries=prior_summaries,
                    log=log,
                )
            except Exception as e:  # noqa: BLE001
                log(f"     !! dev-editor failed: {type(e).__name__}: {str(e)[:200]}")
                notes = {"chapter_id": f"ch{i:02d}", "notes": [], "summary": ""}
            chapter_reviews.append({"chapter_id": f"ch{i:02d}", "notes": notes})
            prior_summaries.append({"chapter_id": f"ch{i:02d}", "summary": (notes.get("summary") or "")[:400]})

    # ── Score the final text (v2 where available, v1 otherwise) ──
    final_metrics = [v2 if v2 else v1 for v1, v2 in zip(v1_metrics, v2_metrics)]
    final_scores = compute_score(final_metrics)

    # If v2 exists, also score v1 for a delta block.
    delta = None
    if has_any_v2:
        v1_scores = compute_score(v1_metrics)
        delta = {
            "dialogue":          round(final_scores["dialogue"]          - v1_scores["dialogue"], 1),
            "voice_variety":     round(final_scores["voice_variety"]     - v1_scores["voice_variety"], 1),
            "imagery_freshness": round(final_scores["imagery_freshness"] - v1_scores["imagery_freshness"], 1),
            "rhythm":            round(final_scores["rhythm"]            - v1_scores["rhythm"], 1),
            "overall_mechanical": round(final_scores["overall_mechanical"] - v1_scores["overall_mechanical"], 1),
            "v1_scores": v1_scores,
        }

    grade_label, grade_desc = grade(final_scores["overall_mechanical"])

    report = {
        "model": args.model,
        "wall_clock_s": round(time.time() - started, 1),
        "chapter_metrics_v1": v1_metrics,
        "chapter_metrics_v2": v2_metrics,
        "final_scores": final_scores,
        "delta_v1_to_v2": delta,
        "grade": grade_label,
        "grade_description": grade_desc,
        "chapter_reviews": chapter_reviews,
    }
    (root / "publication-readiness.json").write_text(json.dumps(report, indent=2))

    # ── Markdown summary ──
    md_lines = [
        "# Publication Readiness Report",
        "",
        f"**Title:** {(brief.get('title_suggestions') or ['(untitled)'])[0]}",
        f"**Model used for review:** `{args.model}`",
        f"**Pipeline stages:** book_review → targeted_redraft → final_review",
        "",
        "## Overall verdict",
        "",
        f"- **Grade:** {grade_label}",
        f"- **{grade_desc}**",
        f"- **Mechanical score (0-10):** {final_scores['overall_mechanical']}",
        "",
        "## Sub-scores",
        "",
        "| Dimension | Score | Evidence |",
        "|---|---|---|",
        f"| Dialogue density | {final_scores['dialogue']} | {final_scores['evidence']['avg_dialogue_per_1000_words']} lines / 1000 words; {final_scores['evidence']['zero_dialogue_chapters']} zero-dialogue chapter(s) |",
        f"| Voice variety | {final_scores['voice_variety']} | {final_scores['evidence']['avg_i_opener_rate_pct']}% sentences start with 'I' |",
        f"| Imagery freshness | {final_scores['imagery_freshness']} | {final_scores['evidence']['total_cliche_phrase_hits']} cliché-phrase hits across book |",
        f"| Sentence rhythm | {final_scores['rhythm']} | stdev {final_scores['evidence']['avg_sentence_length_stdev']} words |",
        "",
    ]
    if delta:
        md_lines.extend([
            "## v1 → v2 delta",
            "",
            "| Dimension | v1 | v2 | Δ |",
            "|---|---|---|---|",
            f"| Dialogue density | {delta['v1_scores']['dialogue']} | {final_scores['dialogue']} | {delta['dialogue']:+} |",
            f"| Voice variety | {delta['v1_scores']['voice_variety']} | {final_scores['voice_variety']} | {delta['voice_variety']:+} |",
            f"| Imagery freshness | {delta['v1_scores']['imagery_freshness']} | {final_scores['imagery_freshness']} | {delta['imagery_freshness']:+} |",
            f"| Sentence rhythm | {delta['v1_scores']['rhythm']} | {final_scores['rhythm']} | {delta['rhythm']:+} |",
            f"| **Overall** | **{delta['v1_scores']['overall_mechanical']}** | **{final_scores['overall_mechanical']}** | **{delta['overall_mechanical']:+}** |",
            "",
        ])
    md_lines.extend([
        "## Per-chapter mechanical metrics",
        "",
        "| Chapter | Words | Dialogue lines | dlg/1000w | I-open% | cliché hits | sent stdev |",
        "|---|---|---|---|---|---|---|",
    ])
    for i, m in enumerate(final_metrics, start=1):
        md_lines.append(
            f"| ch{i:02d} | {m['word_count']} | {m['dialogue_lines']} | "
            f"{m['dialogue_per_1000_words']} | {m['i_opener_rate_pct']} | "
            f"{m['cliche_phrase_hits']} | {m['sentence_length_stdev']} |"
        )
    md_lines.append("")
    md_lines.extend([
        "## How to read this",
        "",
        "- **Dialogue density** below ~10 lines/1000w is the floor for trade fiction.",
        "- **I-opener rate** above 30% produces the monotone-narrator effect.",
        "- **Cliché hits** counts known templated phrases (dust motes, ghost in own life, drop in ocean, etc.).",
        "- **Mechanical score** is one input among many; the LLM dev-editor notes in `publication-readiness.json` cover the interpretive layer (theme, character, structural balance).",
    ])
    (root / "PUBLICATION_READINESS.md").write_text("\n".join(md_lines) + "\n")

    log("")
    log("=" * 72)
    log(f"Grade: {grade_label}  |  Overall: {final_scores['overall_mechanical']}/10")
    if delta:
        log(f"v1 → v2 delta: {delta['overall_mechanical']:+}")
    log(f"Report:   {root / 'publication-readiness.json'}")
    log(f"Readable: {root / 'PUBLICATION_READINESS.md'}")
    log("=" * 72)
    return 0


if __name__ == "__main__":
    sys.exit(main())
