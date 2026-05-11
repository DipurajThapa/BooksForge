#!/usr/bin/env python3
"""
BooksForge scene-recovery driver.

Reads a completed (or partially completed) full-pipeline run and finds
scenes where the draft step failed (missing scenes-draft/chXX-sYY.json
file). For each missing scene, re-runs the chapter-drafter-nf prompt
with retry + lower-temperature fallback, then runs final-polish-merge,
and rewrites the per-chapter Markdown file with the recovered scene
inserted in the right position.

This closes the gap between the Python driver and the Rust orchestrator's
built-in ≤3-attempt retry loop. The Rust orchestrator handles this
natively; the demo Python driver does not.

Usage:
    python booksforge_recover_failed_scenes.py [draft_model] [polish_model] [run_dir]

Defaults:
    draft_model  = qwen3.5:9b
    polish_model = qwen3.5:27b
    run_dir      = book-output/booksforge-ollama-full-run
"""

from __future__ import annotations

import json
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from booksforge_full_pipeline import KEY_PRINCIPLES, STYLE_BOOK, chat_no_thinking  # type: ignore
from booksforge_ollama_driver import (  # type: ignore
    REPO,
    extract_json,
    pm_doc_to_markdown,
    render_template,
)

DEFAULT_RUN = REPO / "book-output/booksforge-ollama-full-run"


def draft_with_retry(
    *,
    drafter_model: str,
    scene: dict,
    chapter_purpose: str,
    prior_summary: str,
    brief: dict,
    max_attempts: int = 3,
    log,
) -> tuple[dict, dict]:
    """Mirrors the Rust orchestrator's ≤3-attempt retry loop."""
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

    last_err = None
    for attempt in range(1, max_attempts + 1):
        # Per-attempt schedule:
        #   1: temp 0.55, json_mode=True, max_tok base
        #   2: temp 0.40, json_mode=True, max_tok ×1.5  (cures late-truncation)
        #   3: temp 0.25, json_mode=True, max_tok ×2.0  (extra slack + lower temp)
        # The hardened extract_json in booksforge_ollama_driver tolerates
        # mid-string truncation and unbalanced JSON via balance-prefix
        # repair, so even a partial response can yield a usable proposal.
        temp = max(0.2, 0.55 - 0.15 * (attempt - 1))
        max_tok = int(base_max_tok * (1.0 + 0.5 * (attempt - 1)))
        try:
            raw, meta = chat_no_thinking(
                drafter_model,
                rendered.system,
                rendered.user,
                temperature=temp,
                max_tokens=max_tok,
                json_mode=True,
            )
            parsed = extract_json(raw)
            log(f"      attempt {attempt}/{max_attempts} ok (temp={temp}, max_tok={max_tok})")
            return parsed, meta
        except Exception as e:  # noqa: BLE001 — surface root cause
            last_err = e
            log(
                f"      attempt {attempt}/{max_attempts} failed: {type(e).__name__}: "
                f"{str(e)[:120]} (temp={temp}, max_tok={max_tok})"
            )
            time.sleep(0.5)
    raise RuntimeError(f"all {max_attempts} draft attempts failed; last={last_err}")


def polish_prose(*, polish_model: str, prose: str, brief: dict) -> tuple[str, dict]:
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
        polish_model,
        rendered.system,
        rendered.user,
        temperature=0.3,
        max_tokens=max_tok,
        json_mode=False,
        timeout=900,
    )
    return polished.strip(), meta


def main() -> int:
    args = sys.argv[1:]
    drafter_model = args[0] if args else "qwen3.5:9b"
    polish_model = args[1] if len(args) > 1 else "qwen3.5:27b"
    run_dir = Path(args[2]) if len(args) > 2 else DEFAULT_RUN

    if not (run_dir / "01-brief.json").exists():
        print(f"FATAL: no brief at {run_dir}/01-brief.json", file=sys.stderr)
        return 1

    brief = json.loads((run_dir / "01-brief.json").read_text())
    outline = json.loads((run_dir / "02-outline.json").read_text())

    chapters: list[dict] = []
    for part in outline.get("parts", []):
        for ch in part.get("chapters", []):
            chapters.append(ch)

    log_lines: list[str] = []

    def log(msg: str) -> None:
        ts = time.strftime("%H:%M:%S")
        line = f"[{ts}] {msg}"
        print(line, flush=True)
        log_lines.append(line)
        with open(run_dir / "recovery.log", "w") as f:
            f.write("\n".join(log_lines))

    log(f"Scene-recovery pass | drafter={drafter_model} polish={polish_model} run={run_dir}")

    # Find missing scenes: for each chapter c (1-indexed), each scene s (1-indexed)
    # where scenes-draft/chCC-sSS.json does not exist.
    missing: list[tuple[int, int, dict, dict]] = []  # (ci, si, chapter, scene)
    for ci, ch in enumerate(chapters, start=1):
        for si, scene in enumerate(ch.get("scenes", []), start=1):
            draft_path = run_dir / "scenes-draft" / f"ch{ci:02d}-s{si:02d}.json"
            if not draft_path.exists():
                missing.append((ci, si, ch, scene))

    log(f"Found {len(missing)} missing scenes to recover")
    if not missing:
        log("Nothing to do.")
        return 0

    prior_summary = ""
    recovered = 0
    failed = 0

    # Build a lookup of prior_summary as the original driver would have built it,
    # so the recovered scenes see the same context.
    prior_summaries_by_chapter: dict[int, str] = {1: ""}
    for ci, ch in enumerate(chapters, start=1):
        if ci > 1:
            prev = chapters[ci - 2]
            prior_summaries_by_chapter[ci] = (
                f"Chapter {ci - 1} ({prev.get('title', '')}): {prev.get('purpose', '')}"
            )

    for ci, si, ch, scene in missing:
        ch_title = ch.get("title", f"Chapter {ci}")
        ch_purpose = ch.get("purpose", "")
        synopsis = scene.get("synopsis", "")[:80]
        target = int(scene.get("target_word_count") or 700)
        prior_summary = prior_summaries_by_chapter.get(ci, "")

        log(f"Recovering ch{ci:02d}-s{si:02d} | target={target}w | synopsis={synopsis!r}")

        try:
            draft, dmeta = draft_with_retry(
                drafter_model=drafter_model,
                scene=scene,
                chapter_purpose=ch_purpose,
                prior_summary=prior_summary,
                brief=brief,
                max_attempts=3,
                log=log,
            )
        except Exception as e:  # noqa: BLE001
            log(f"  !! all draft attempts failed: {e}")
            failed += 1
            continue

        # Save draft.
        (run_dir / "scenes-draft" / f"ch{ci:02d}-s{si:02d}.json").write_text(json.dumps(draft, indent=2))
        pm = draft.get("pm_doc") or {"type": "doc", "content": []}
        draft_md = pm_doc_to_markdown(pm)
        draft_words = len(draft_md.split())
        log(f"  DRAFT  {draft_words}w (target {target}, drafter said {draft.get('word_count')}) elapsed={dmeta['elapsed_s']}s")

        # Polish.
        try:
            polished, pmeta = polish_prose(polish_model=polish_model, prose=draft_md, brief=brief)
        except Exception as e:  # noqa: BLE001
            log(f"  !! polish failed: {e}")
            polished = draft_md
        polished_words = len(polished.split())
        delta = polished_words - draft_words
        log(f"  POLISH {polished_words}w (Δ{delta:+d})")

        # Save polished scene.
        (run_dir / "scenes-polished" / f"ch{ci:02d}-s{si:02d}.md").write_text(polished + "\n")
        recovered += 1

    # ── Reassemble per-chapter Markdown for chapters that had recoveries ─────
    affected = {ci for ci, _, _, _ in missing}
    for ci in sorted(affected):
        ch = chapters[ci - 1]
        ch_title = ch.get("title", f"Chapter {ci}")
        ch_purpose = ch.get("purpose", "")
        chapter_md = [f"## Chapter {ci} — {ch_title}", ""]
        if ch_purpose:
            chapter_md.append(f"*{ch_purpose}*")
            chapter_md.append("")
        for si, scene in enumerate(ch.get("scenes", []), start=1):
            scene_path = run_dir / "scenes-polished" / f"ch{ci:02d}-s{si:02d}.md"
            if scene_path.exists():
                chapter_md.append(scene_path.read_text().rstrip())
                chapter_md.append("")
            else:
                chapter_md.append(f"> *[scene draft still failed after recovery: ch{ci:02d}-s{si:02d}]*")
                chapter_md.append("")
        (run_dir / "chapters" / f"chapter-{ci:02d}.md").write_text("\n".join(chapter_md).rstrip() + "\n")

    log(f"Done. Recovered {recovered}, still failed {failed}, of {len(missing)} missing scenes.")
    log(f"Reassembled chapters: {sorted(affected)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
