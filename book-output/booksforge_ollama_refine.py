#!/usr/bin/env python3
"""
BooksForge two-tier refine pass.

Reads scenes drafted by the cheap fast model (e.g. qwen3.5:9b) from a prior
driver run and pipes each through the BooksForge `final-polish` template
on a heavy model (e.g. qwen3.5:27b). The polish prompt forbids invention,
forbids voice change, and tells the editor to tighten, vary cadence, fix
dull verbs, and smooth transitions. This matches the user's proposed
"draft-on-9B, refine-on-27B" pipeline and uses BooksForge's actual
production prompt for the polish step.

Privacy: only outbound network call is to 127.0.0.1:11434.
"""

from __future__ import annotations

import json
import sys
import time
from pathlib import Path

# Reuse the helpers from the main driver.
sys.path.insert(0, str(Path(__file__).parent))
from booksforge_ollama_driver import (  # type: ignore
    OUTPUT_DIR as DEFAULT_OUTPUT_DIR,
    REPO,
    ollama_chat,
    ollama_probe,
    render_template,
)

DRAFT_RUN = REPO / "book-output/booksforge-ollama-run-qwen9b"
REFINE_RUN = REPO / "book-output/booksforge-ollama-run-refined"


def main() -> int:
    args = sys.argv[1:]
    refine_model = args[0] if args else "qwen3.5:27b"
    draft_run = Path(args[1]) if len(args) > 1 else DRAFT_RUN
    out_dir = Path(args[2]) if len(args) > 2 else REFINE_RUN

    out_dir.mkdir(parents=True, exist_ok=True)
    (out_dir / "raw").mkdir(exist_ok=True)
    (out_dir / "before-after").mkdir(exist_ok=True)

    log_lines: list[str] = []

    def log(msg: str) -> None:
        ts = time.strftime("%H:%M:%S")
        line = f"[{ts}] {msg}"
        print(line, flush=True)
        log_lines.append(line)

    log(f"BooksForge refine driver | model={refine_model} | draft_run={draft_run}")
    probe = ollama_probe()
    log(f"Ollama version={probe['version']} | {len(probe['models'])} local models")
    if refine_model not in probe["models"]:
        log(f"WARNING: refine model '{refine_model}' not local: {probe['models']}")

    brief_path = draft_run / "01-brief.json"
    if not brief_path.exists():
        log(f"ERROR: missing draft brief at {brief_path}")
        return 1
    brief = json.loads(brief_path.read_text())
    genre = brief.get("genre", "")
    audience = brief.get("audience", "")
    tone = brief.get("tone", "")

    style_book = {
        "em_dash": "em",
        "quote_style": "smart",
        "oxford_comma": True,
        "locale": "en-US",
    }

    scene_files = sorted((draft_run / "scenes").glob("ch*.json"))
    log(f"Found {len(scene_files)} drafted scenes to refine")

    summary_rows = []
    for sp in scene_files:
        scene_id = sp.stem
        log(f"Refining {scene_id}...")
        scene = json.loads(sp.read_text())
        pm = scene.get("pm_doc", {"type": "doc", "content": []})
        # Reassemble paragraphs into prose
        before_paragraphs: list[str] = []
        for node in pm.get("content", []):
            if node.get("type") == "paragraph":
                text = "".join(c.get("text", "") for c in node.get("content", []))
                before_paragraphs.append(text.strip())
        before_prose = "\n\n".join(p for p in before_paragraphs if p)
        before_words = len(before_prose.split())

        rendered = render_template(
            "final-polish",
            {
                "scope_text": before_prose,
                "genre": genre,
                "audience": audience,
                "tone": tone,
                "style_book_json": style_book,
            },
        )
        # final-polish returns raw prose, not JSON
        # Allow extra room because polish may keep length similar but with rewriting.
        max_tok = max(2048, int(before_words * 2.0))
        try:
            polished, meta = ollama_chat(
                refine_model,
                rendered.system,
                rendered.user,
                temperature=0.3,
                max_tokens=max_tok,
                json_mode=False,
                timeout=900,
            )
        except Exception as e:  # noqa: BLE001 — surface root cause
            log(f"  !! polish failed: {type(e).__name__}: {e}")
            continue

        polished = polished.strip()
        polished_words = len(polished.split())
        delta = polished_words - before_words
        log(
            f"  {scene_id}: before={before_words}w after={polished_words}w "
            f"(Δ{delta:+d}) elapsed={meta['elapsed_s']}s"
        )

        (out_dir / "raw" / f"{scene_id}.refine.txt").write_text(polished)
        (out_dir / "before-after" / f"{scene_id}.md").write_text(
            f"# {scene_id} — refine pass\n\n"
            f"- Draft model: qwen3.5:9b → Refine model: {refine_model}\n"
            f"- Words before: {before_words} | Words after: {polished_words} (Δ{delta:+d})\n"
            f"- Refine elapsed: {meta['elapsed_s']}s\n\n"
            f"## Before\n\n{before_prose}\n\n## After\n\n{polished}\n"
        )
        summary_rows.append(
            {
                "scene": scene_id,
                "before_words": before_words,
                "after_words": polished_words,
                "delta_words": delta,
                "elapsed_s": meta["elapsed_s"],
                "tokens_out": meta.get("eval_count"),
            }
        )

    (out_dir / "refine-summary.json").write_text(
        json.dumps(
            {"refine_model": refine_model, "draft_run": str(draft_run), "scenes": summary_rows},
            indent=2,
        )
    )
    (out_dir / "refine.log").write_text("\n".join(log_lines))
    log("Done. See before-after/ for paired comparison files.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
