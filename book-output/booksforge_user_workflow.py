#!/usr/bin/env python3
"""
BooksForge end-to-end user workflow — final pass + multi-format export.

This is what a user does AFTER drafting + polishing + humanizing: a final
editorial pass on the largest available model, then Pandoc-driven exports
to publication-ready formats. Mirrors the BooksForge desktop flow:
  Edit → Final Review → Export (DOCX / EPUB / Markdown).

Per-stage model selection (locked, with rationale):
  intake / outline / drafter        → qwen3.5:9b      (cheap, fast, schema-OK)
  final-polish-merge                → qwen3.5:27b    (cuts redundancy, holds voice)
  humanization                      → qwen3.5:27b    (anti-AI-tell judgment)
  final-review-editor               → qwen3.6:latest (world-class polish; the
                                                       agent registry pins FRE
                                                       to qwen3.6 specifically)

Privacy: only outbound network call is to 127.0.0.1:11434 (Ollama).
Pandoc runs locally; no network access.

Usage:
  python booksforge_user_workflow.py [fre_model] [input_run] [out_dir]
Defaults:
  fre_model = qwen3.6:latest
  input_run = book-output/booksforge-ollama-full-run-humanized
  out_dir   = book-output/booksforge-final-export
"""

from __future__ import annotations

import json
import re
import subprocess
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from booksforge_ollama_driver import (  # type: ignore
    REPO,
    extract_json,
    ollama_probe,
    render_template,
)
from booksforge_full_pipeline import STYLE_BOOK, chat_no_thinking  # type: ignore

DEFAULT_INPUT = REPO / "book-output/booksforge-ollama-full-run-humanized"
DEFAULT_OUT = REPO / "book-output/booksforge-final-export"


def run_fre_on_chapter(chapter_md: str, model: str, brief: dict) -> tuple[str, dict]:
    """Run final-polish (the raw-prose-out variant of FRE) on a whole chapter.

    The agent registry pins final-review-editor to qwen3.6 specifically;
    we use the production `final-polish/v1.toml` template for raw prose
    output (it's the same world-class-polish-with-voice-preservation
    contract, just without the JSON-schema-edit-proposal envelope).
    """
    rendered = render_template(
        "final-polish",
        {
            "scope_text": chapter_md,
            "genre": brief.get("genre", ""),
            "audience": brief.get("audience", ""),
            "tone": brief.get("tone", ""),
            "style_book_json": STYLE_BOOK,
        },
    )
    word_count = len(chapter_md.split())
    # FRE quality matters; give plenty of token slack — 27B/36B-class models
    # rarely run away with output if the prompt's length-discipline is honored.
    max_tok = max(4096, int(word_count * 2.0))
    polished, meta = chat_no_thinking(
        model,
        rendered.system,
        rendered.user,
        temperature=0.3,
        max_tokens=max_tok,
        json_mode=False,
        timeout=1200,
    )
    return polished.strip(), meta


def assemble_combined(chapters_dir: Path, brief: dict, title: str) -> str:
    """Assemble per-chapter Markdown into a single manuscript with front matter."""
    parts: list[str] = []
    parts.append(f"% {title}")
    parts.append(f"% {brief.get('audience','')}")
    parts.append("% " + time.strftime("%Y"))
    parts.append("")
    parts.append(f"# {title}")
    parts.append("")
    parts.append(f"*Audience:* {brief.get('audience','')}")
    parts.append(f"*Tone:* {brief.get('tone','')}")
    parts.append("")
    parts.append("---")
    parts.append("")
    for ch_path in sorted(chapters_dir.glob("ch*.md")):
        parts.append(ch_path.read_text().rstrip())
        parts.append("")
    return "\n".join(parts).rstrip() + "\n"


def pandoc_export(
    input_md: Path,
    out_path: Path,
    fmt: str,
    *,
    title: str,
    author: str = "BooksForge",
    extra_args: list[str] | None = None,
) -> tuple[float, int]:
    """Export Markdown → fmt via Pandoc. Returns (elapsed_s, bytes)."""
    args = [
        "pandoc",
        str(input_md),
        "-o", str(out_path),
        "--standalone",
        "--metadata", f"title={title}",
        "--metadata", f"author={author}",
        "--metadata", f"date={time.strftime('%Y-%m-%d')}",
        "--toc",
        "--toc-depth=2",
    ]
    if fmt == "epub":
        args += ["-t", "epub3"]
    elif fmt == "docx":
        args += ["-t", "docx"]
    if extra_args:
        args += extra_args
    t0 = time.time()
    res = subprocess.run(args, capture_output=True, text=True, timeout=300)
    elapsed = time.time() - t0
    if res.returncode != 0:
        raise RuntimeError(f"pandoc failed ({fmt}): {res.stderr.strip()}")
    return elapsed, out_path.stat().st_size


def main() -> int:
    args = sys.argv[1:]
    fre_model = args[0] if args else "qwen3.6:latest"
    in_dir = Path(args[1]) if len(args) > 1 else DEFAULT_INPUT
    out_dir = Path(args[2]) if len(args) > 2 else DEFAULT_OUT

    if not (in_dir / "chapters").exists():
        print(f"FATAL: no chapters/ at {in_dir}", file=sys.stderr)
        return 1

    out_dir.mkdir(parents=True, exist_ok=True)
    (out_dir / "chapters-fre").mkdir(exist_ok=True)
    (out_dir / "exports").mkdir(exist_ok=True)

    # Pull the brief from the upstream run.
    upstream_brief = REPO / "book-output/booksforge-ollama-full-run/01-brief.json"
    brief = json.loads(upstream_brief.read_text()) if upstream_brief.exists() else {}
    title = brief.get("title_suggestions", ["How to Make Money During the AI Boom"])[0]

    log_lines: list[str] = []

    def log(msg: str) -> None:
        ts = time.strftime("%H:%M:%S")
        line = f"[{ts}] {msg}"
        print(line, flush=True)
        log_lines.append(line)
        with open(out_dir / "workflow.log", "w") as f:
            f.write("\n".join(log_lines))

    log("=" * 78)
    log(f"BooksForge user workflow | FRE={fre_model}")
    log(f"Input:  {in_dir}")
    log(f"Output: {out_dir}")
    log("=" * 78)

    probe = ollama_probe()
    log(f"Ollama version={probe['version']} | {len(probe['models'])} local models")
    if fre_model not in probe["models"]:
        log(f"FATAL: FRE model '{fre_model}' not pulled locally.")
        return 1

    # ── STAGE: Final-Review-Editor pass on each humanized chapter ──────────────
    log("─" * 78)
    log("STAGE 1: Final-Review-Editor on each humanized chapter (qwen3.6:latest)")
    log("─" * 78)

    chapters = sorted((in_dir / "chapters").glob("ch*.md"))
    log(f"Found {len(chapters)} chapters to FRE-pass")

    fre_summary: list[dict] = []
    pipeline_start = time.time()

    for idx, ch_path in enumerate(chapters, start=1):
        md = ch_path.read_text().rstrip()
        words_in = len(md.split())
        log(f"  Chapter {idx}/{len(chapters)} {ch_path.name} | {words_in} words → FRE")
        try:
            polished, meta = run_fre_on_chapter(md, fre_model, brief)
        except Exception as e:  # noqa: BLE001
            log(f"    !! FRE failed: {type(e).__name__}: {e}")
            polished = md  # fall back to humanized version
            meta = {"elapsed_s": 0, "eval_count": 0}
        words_out = len(polished.split())
        delta = words_out - words_in
        log(f"    -> {words_out} words (Δ{delta:+d}) elapsed={meta['elapsed_s']}s eval={meta.get('eval_count')}")
        out_path = out_dir / "chapters-fre" / ch_path.name
        out_path.write_text(polished + "\n")
        fre_summary.append({
            "chapter": idx,
            "name": ch_path.name,
            "words_in": words_in,
            "words_out": words_out,
            "delta": delta,
            "elapsed_s": meta["elapsed_s"],
            "tokens_out": meta.get("eval_count"),
        })

    fre_elapsed_min = (time.time() - pipeline_start) / 60.0
    log(f"FRE pass complete in {fre_elapsed_min:.1f} min")

    # ── STAGE: Assemble combined manuscript ───────────────────────────────────
    log("─" * 78)
    log("STAGE 2: Assemble combined manuscript")
    log("─" * 78)

    combined = assemble_combined(out_dir / "chapters-fre", brief, title)
    md_path = out_dir / "exports" / "manuscript.md"
    md_path.write_text(combined)
    md_words = len(combined.split())
    log(f"  Combined manuscript: {md_path.name} ({md_words} words, {md_path.stat().st_size} bytes)")

    # ── STAGE: Multi-format export via Pandoc ────────────────────────────────
    log("─" * 78)
    log("STAGE 3: Multi-format export (Pandoc → DOCX, EPUB-3)")
    log("─" * 78)

    exports: list[dict] = []

    # DOCX
    try:
        elapsed, size = pandoc_export(
            md_path,
            out_dir / "exports" / "manuscript.docx",
            "docx",
            title=title,
        )
        log(f"  DOCX: manuscript.docx ({size:,} bytes, {elapsed:.1f}s)")
        exports.append({"format": "docx", "size_bytes": size, "elapsed_s": elapsed})
    except Exception as e:  # noqa: BLE001
        log(f"  !! DOCX failed: {e}")

    # EPUB-3
    try:
        elapsed, size = pandoc_export(
            md_path,
            out_dir / "exports" / "manuscript.epub",
            "epub",
            title=title,
        )
        log(f"  EPUB: manuscript.epub ({size:,} bytes, {elapsed:.1f}s)")
        exports.append({"format": "epub3", "size_bytes": size, "elapsed_s": elapsed})
    except Exception as e:  # noqa: BLE001
        log(f"  !! EPUB failed: {e}")

    # Markdown is already on disk via md_path; symlink as canonical output too.
    exports.append({
        "format": "markdown",
        "size_bytes": md_path.stat().st_size,
        "elapsed_s": 0.0,
    })

    # ── Summary ───────────────────────────────────────────────────────────────
    log("=" * 78)
    log("SUMMARY")
    log("=" * 78)
    log(f"  FRE wall-clock:    {fre_elapsed_min:.1f} min")
    log(f"  FRE input words:   {sum(r['words_in']  for r in fre_summary):,}")
    log(f"  FRE output words:  {sum(r['words_out'] for r in fre_summary):,}")
    log(f"  Manuscript words:  {md_words:,}")
    for ex in exports:
        log(f"  {ex['format'].upper():<10} → {ex['size_bytes']:>10,} bytes ({ex['elapsed_s']:.1f}s)")

    summary = {
        "fre_model": fre_model,
        "ollama_version": probe["version"],
        "input_run": str(in_dir),
        "manuscript_words": md_words,
        "fre_chapters": fre_summary,
        "exports": exports,
        "title": title,
        "wall_clock_min": round(fre_elapsed_min, 2),
    }
    (out_dir / "workflow-summary.json").write_text(json.dumps(summary, indent=2))
    log(f"  workflow-summary.json saved")
    log("=" * 78)

    return 0


if __name__ == "__main__":
    sys.exit(main())
