#!/usr/bin/env python3
"""
BooksForge full two-tier pipeline driver — production run.

Pipeline:
  1. Intake (qwen3.5:9b, think=false)        — idea -> ProjectBrief
  2. Outline (qwen3.5:9b, think=false)        — ProjectBrief -> 15-chapter OutlineProposal
  3. For each scene (in order):
     a. Draft on chapter-drafter-nf v1 (qwen3.5:9b, think=false)
     b. Polish on final-polish-merge v1 (qwen3.5:27b, think=false)
  4. Assemble per-chapter Markdown + a single combined manuscript.

Uses the new templates added in this iteration:
  - chapter-drafter-nf/v1.toml  (non-fiction sibling of chapter-drafter)
  - final-polish-merge/v1.toml  (allows paragraph merging)

All HTTP calls go to 127.0.0.1:11434. No outbound network beyond Ollama.

Usage:
    python booksforge_full_pipeline.py [draft_model] [polish_model] [out_dir]

Defaults:
    draft_model  = qwen3.5:9b
    polish_model = qwen3.5:27b
    out_dir      = book-output/booksforge-ollama-full-run
"""

from __future__ import annotations

import json
import os
import re
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from booksforge_ollama_driver import (  # type: ignore
    REPO,
    extract_json,
    ollama_chat,
    ollama_probe,
    pm_doc_to_markdown,
    render_template,
)

DEFAULT_OUT = REPO / "book-output/booksforge-ollama-full-run"

BOOK_IDEA = """\
A non-fiction strategy book titled 'How to Make Money During the AI Boom',
written from the perspective of a senior capital allocator. Target audience
is ambitious professionals, founders, investors, creators, and operators
who want to benefit from the AI economy without falling for hype. Tone:
direct, confident, unsentimental, allocator-grade. The book should explain
where AI money is being made (infrastructure, platforms, applications,
data, distribution), how individuals and businesses can monetize AI
through wage leverage, consulting, productized services, and acquisitions,
how to invest in the theme without being destroyed by hype, and how to
survive the bubbles and shake-outs. Target length 40,000 words across
15 chapters. The book must avoid invented statistics, fabricated case
studies, and personalized financial advice. It should feel commercially
publishable.
"""

KEY_PRINCIPLES = [
    "Position upstream in the value chain.",
    "Compound skill, capital, and distribution.",
    "Price paid is the dominant determinant of return.",
    "Cash flow over narrative.",
    "Survive the bust; buy through it.",
    "Optionality is an asset class.",
    "Think in decades, execute in quarters.",
    "Avoid permanent loss of capital.",
]

STYLE_BOOK = {
    "em_dash": "em",
    "quote_style": "smart",
    "oxford_comma": True,
    "locale": "en-US",
}


def chat_no_thinking(model, system, user, *, temperature, max_tokens, json_mode=False, timeout=900):
    """Wrapper that always sends think:false — exercises the new ThinkingMode flag end-to-end."""
    import requests

    payload = {
        "model": model,
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": user},
        ],
        "stream": False,
        "think": False,
        "options": {"temperature": temperature, "num_predict": max_tokens},
    }
    if json_mode:
        payload["format"] = "json"
    t0 = time.time()
    r = requests.post("http://127.0.0.1:11434/api/chat", json=payload, timeout=timeout)
    elapsed = time.time() - t0
    r.raise_for_status()
    data = r.json()
    content = data.get("message", {}).get("content", "")
    return content, {
        "elapsed_s": round(elapsed, 2),
        "eval_count": data.get("eval_count"),
        "prompt_eval_count": data.get("prompt_eval_count"),
        "model": model,
    }


def draft_section(*, drafter_model, scene, chapter_purpose, prior_summary, brief, log=None):
    """Draft a single section with retry + repair.

    Mirrors the BooksForge Rust orchestrator's ≤3-attempt retry loop.

    Per attempt schedule (escalates aggressively on attempt 3 — the prior
    fixed schedule lost one scene per ~27 because attempt 3 only doubled
    the budget; the 9B occasionally produces 5,000+ output tokens for a
    1,500-word scene because of JSON-mode whitespace bloat):
      1: temp 0.55, max_tok = target × 2.0,   json_mode=True      (default)
      2: temp 0.40, max_tok = target × 3.5,   json_mode=True      (late-truncation cure)
      3: temp 0.20, max_tok = target × 5.0,   json_mode=False     (last-ditch — drops
                                                                    JSON mode entirely so
                                                                    the model can emit
                                                                    looser output that
                                                                    the repaired
                                                                    extract_json picks up
                                                                    via its balance-prefix
                                                                    walker)

    The hardened ``extract_json`` (in booksforge_ollama_driver) tolerates
    mid-string truncation and unbalanced JSON via balance-prefix repair,
    so even a partial response can yield a usable proposal — but we
    still prefer a clean draft, so we retry first and only let the
    repair logic catch the truly truncated ones.
    """
    rendered = render_template(
        "chapter-drafter-nf",
        {
            "section_synopsis": scene.get("synopsis", ""),
            "chapter_purpose": chapter_purpose,
            "target_words": int(scene.get("target_word_count") or 700),
            "known_entities": [],
            "prior_summary": prior_summary or "",
            "voice_fingerprint": {"corpus_tokens": 0},
            "audience": brief.get("audience", ""),
            "key_principles": KEY_PRINCIPLES,
            "genre": brief.get("genre", ""),
            "tone": brief.get("tone", ""),
            "prompt_guard": "",
        },
    )
    target = int(scene.get("target_word_count") or 700)
    base_max_tok = max(2048, int(target * 2.0))
    max_attempts = 3
    last_err: Exception | None = None

    # Per-attempt schedule. Attempt 3 escalates aggressively and drops
    # json_mode so the model can emit free-form output that the repaired
    # `extract_json` picks up via its balance-prefix walker.
    schedule = [
        (0.55, 2.0, True),   # attempt 1: default
        (0.40, 3.5, True),   # attempt 2: bigger budget, cooler sampling
        (0.20, 5.0, False),  # attempt 3: 5× budget, no json_mode (last-ditch)
    ]
    for attempt in range(1, max_attempts + 1):
        temp, mult, json_mode = schedule[attempt - 1]
        max_tok = int(base_max_tok * mult)
        try:
            raw, meta = chat_no_thinking(
                drafter_model,
                rendered.system,
                rendered.user,
                temperature=temp,
                max_tokens=max_tok,
                json_mode=json_mode,
            )
            parsed = extract_json(raw)
            if attempt > 1 and log is not None:
                log(
                    f"        recovered on attempt {attempt} "
                    f"(temp={temp}, max_tok={max_tok}, json_mode={json_mode})"
                )
            return parsed, meta
        except Exception as e:  # noqa: BLE001
            last_err = e
            if log is not None:
                log(
                    f"        attempt {attempt}/{max_attempts} failed: {type(e).__name__}: "
                    f"{str(e)[:120]} (temp={temp}, max_tok={max_tok}, json_mode={json_mode})"
                )
            time.sleep(0.5)

    raise RuntimeError(f"all {max_attempts} draft attempts failed; last={last_err}")


def polish_prose(*, polish_model, prose, brief):
    rendered = render_template(
        "final-polish-merge",
        {
            "scope_text": prose,
            "genre": brief.get("genre", ""),
            "audience": brief.get("audience", ""),
            "tone": brief.get("tone", ""),
            "style_book_json": STYLE_BOOK,
        },
    )
    word_count = len(prose.split())
    max_tok = max(2048, int(word_count * 2.0))
    polished, meta = chat_no_thinking(
        polish_model, rendered.system, rendered.user,
        temperature=0.3, max_tokens=max_tok, json_mode=False,
    )
    return polished.strip(), meta


def main() -> int:
    args = sys.argv[1:]
    drafter_model = args[0] if args else "qwen3.5:9b"
    polish_model = args[1] if len(args) > 1 else "qwen3.5:27b"
    out_dir = Path(args[2]) if len(args) > 2 else DEFAULT_OUT

    out_dir.mkdir(parents=True, exist_ok=True)
    (out_dir / "raw").mkdir(exist_ok=True)
    (out_dir / "scenes-draft").mkdir(exist_ok=True)
    (out_dir / "scenes-polished").mkdir(exist_ok=True)
    (out_dir / "chapters").mkdir(exist_ok=True)

    log_lines: list[str] = []

    def log(msg: str) -> None:
        ts = time.strftime("%H:%M:%S")
        line = f"[{ts}] {msg}"
        print(line, flush=True)
        log_lines.append(line)
        with open(out_dir / "run.log", "w") as f:
            f.write("\n".join(log_lines))

    log("=" * 72)
    log(f"BooksForge full two-tier pipeline | draft={drafter_model} polish={polish_model}")
    log(f"Templates: chapter-drafter-nf/v1, final-polish-merge/v1")
    log(f"Output: {out_dir}")
    log("=" * 72)

    # Probe Ollama.
    probe = ollama_probe()
    log(f"Ollama version={probe['version']} | {len(probe['models'])} local models")
    for mdl in (drafter_model, polish_model):
        if mdl not in probe["models"]:
            log(f"FATAL: model '{mdl}' not pulled locally. Pull and retry.")
            return 1

    # ── Step 1: Intake ────────────────────────────────────────────────────────
    log("─" * 72)
    log("STEP 1/3: Intake (idea -> ProjectBrief)")
    rendered = render_template("intake", {"idea_text": BOOK_IDEA, "preferred_mode": "non_fiction", "prompt_guard": ""})
    out, meta = chat_no_thinking(drafter_model, rendered.system, rendered.user, temperature=0.3, max_tokens=2048, json_mode=True)
    (out_dir / "raw" / "01-intake.txt").write_text(out)
    brief = extract_json(out)
    (out_dir / "01-brief.json").write_text(json.dumps(brief, indent=2))
    log(f"  -> mode={brief.get('mode')} target_words={brief.get('target_word_count')} elapsed={meta['elapsed_s']}s eval_count={meta['eval_count']}")

    # ── Step 2: Outline ───────────────────────────────────────────────────────
    log("─" * 72)
    log("STEP 2/3: Outline (ProjectBrief -> 15-chapter OutlineProposal)")
    rendered = render_template("outline-architect", {"brief": brief, "target_chapter_count": 15, "genre_overlay": ""})
    out, meta = chat_no_thinking(drafter_model, rendered.system, rendered.user, temperature=0.4, max_tokens=8192, json_mode=True)
    (out_dir / "raw" / "02-outline.txt").write_text(out)
    outline = extract_json(out)
    (out_dir / "02-outline.json").write_text(json.dumps(outline, indent=2))
    chapters: list[dict] = []
    for part in outline.get("parts", []):
        for ch in part.get("chapters", []):
            chapters.append(ch)
    log(f"  -> parts={len(outline.get('parts', []))} chapters={len(chapters)} elapsed={meta['elapsed_s']}s eval_count={meta['eval_count']}")

    if len(chapters) < 1:
        log("FATAL: outline produced no chapters")
        return 2

    # ── Step 3: Draft + Polish each scene ────────────────────────────────────
    log("─" * 72)
    log(f"STEP 3/3: Draft + polish for {len(chapters)} chapters")

    prior_summary = ""
    chapter_summaries: list[dict] = []
    total_draft_words = 0
    total_polish_words = 0
    pipeline_start = time.time()

    for ci, ch in enumerate(chapters, start=1):
        ch_title = ch.get("title", f"Chapter {ci}")
        ch_purpose = ch.get("purpose", "")
        scenes = ch.get("scenes", [])
        log(f"")
        log(f"  ── Chapter {ci}/{len(chapters)}: {ch_title!r}")
        log(f"     scenes: {len(scenes)} | purpose: {ch_purpose[:80]}…")

        # Strip any leading "Chapter N:" or "Chapter N -" the outline-architect
        # may have already emitted, so we don't end up with "Chapter 1 — Chapter 1: Title".
        clean_title = re.sub(r"^\s*Chapter\s+\d+\s*[:\-—]\s*", "", ch_title, flags=re.IGNORECASE)
        chapter_md_lines = [f"## Chapter {ci} — {clean_title}", ""]
        if ch_purpose:
            chapter_md_lines.append(f"*{ch_purpose}*")
            chapter_md_lines.append("")

        for si, scene in enumerate(scenes, start=1):
            target = int(scene.get("target_word_count") or 700)
            syn = scene.get("synopsis", "")[:80]
            log(f"     scene {si}/{len(scenes)}: target={target}w  synopsis={syn!r}")

            # ── Draft (9B) ──
            try:
                draft, dmeta = draft_section(
                    drafter_model=drafter_model,
                    scene=scene,
                    chapter_purpose=ch_purpose,
                    prior_summary=prior_summary,
                    brief=brief,
                    log=log,
                )
            except Exception as e:  # noqa: BLE001
                log(f"        !! DRAFT failed: {type(e).__name__}: {e}")
                chapter_md_lines.append(f"> *[scene draft failed]*\n")
                continue

            (out_dir / "scenes-draft" / f"ch{ci:02d}-s{si:02d}.json").write_text(json.dumps(draft, indent=2))
            pm = draft.get("pm_doc") or {"type": "doc", "content": []}
            draft_md = pm_doc_to_markdown(pm)
            draft_words = len(draft_md.split())
            total_draft_words += draft_words
            log(f"        DRAFT  {draft_words}w (target {target}, drafter said {draft.get('word_count')}) elapsed={dmeta['elapsed_s']}s")

            # ── Polish (27B) ──
            try:
                polished, pmeta = polish_prose(polish_model=polish_model, prose=draft_md, brief=brief)
            except Exception as e:  # noqa: BLE001
                log(f"        !! POLISH failed: {type(e).__name__}: {e}")
                polished = draft_md
                pmeta = {"elapsed_s": 0, "eval_count": 0}

            (out_dir / "scenes-polished" / f"ch{ci:02d}-s{si:02d}.md").write_text(polished + "\n")
            polished_words = len(polished.split())
            total_polish_words += polished_words
            delta = polished_words - draft_words
            log(f"        POLISH {polished_words}w (Δ{delta:+d}) elapsed={pmeta['elapsed_s']}s")

            chapter_md_lines.append(polished)
            chapter_md_lines.append("")

        chapter_md = "\n".join(chapter_md_lines).rstrip() + "\n"
        ch_path = out_dir / "chapters" / f"chapter-{ci:02d}.md"
        ch_path.write_text(chapter_md)
        ch_words = len(chapter_md.split())
        chapter_summaries.append({"chapter": ci, "title": ch_title, "scenes": len(scenes), "words": ch_words, "path": str(ch_path)})
        log(f"     -> chapter saved: {ch_words} words")

        prior_summary = f"Chapter {ci} ({ch_title}): {ch_purpose}"

        # Persist running summary after every chapter so partial progress is recoverable.
        (out_dir / "run-summary.json").write_text(json.dumps({
            "drafter_model": drafter_model,
            "polish_model": polish_model,
            "ollama_version": probe["version"],
            "templates": ["intake/v1", "outline-architect/v1", "chapter-drafter-nf/v1", "final-polish-merge/v1"],
            "total_draft_words": total_draft_words,
            "total_polish_words": total_polish_words,
            "chapters_completed": len(chapter_summaries),
            "chapters_planned": len(chapters),
            "wall_clock_s": round(time.time() - pipeline_start, 1),
            "chapter_summaries": chapter_summaries,
        }, indent=2))

    # ── Assemble combined manuscript ─────────────────────────────────────────
    log("─" * 72)
    log("Assembling combined manuscript…")
    combined: list[str] = [
        f"# {brief.get('title_suggestions', ['How to Make Money During the AI Boom'])[0]}",
        "",
        f"**Audience:** {brief.get('audience','')}",
        f"**Tone:** {brief.get('tone','')}",
        f"**Mode:** {brief.get('mode','')}",
        "",
        f"> *Disclaimer.* {brief.get('premise','')}",
        "",
        "---",
        "",
    ]
    for s in chapter_summaries:
        combined.append(Path(s["path"]).read_text().rstrip())
        combined.append("")
        combined.append("---")
        combined.append("")
    combined_text = "\n".join(combined).rstrip() + "\n"
    (out_dir / "FULL_MANUSCRIPT.md").write_text(combined_text)
    total_words = len(combined_text.split())
    log(f"Combined manuscript: {total_words} words")

    log("=" * 72)
    log(f"DONE. Total wall-clock: {round((time.time() - pipeline_start)/60, 1)} min")
    log(f"Final manuscript: {out_dir / 'FULL_MANUSCRIPT.md'}")
    log(f"Run summary:     {out_dir / 'run-summary.json'}")
    log("=" * 72)

    return 0


if __name__ == "__main__":
    sys.exit(main())
