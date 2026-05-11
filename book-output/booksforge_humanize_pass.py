#!/usr/bin/env python3
"""
BooksForge humanization post-pass.

Reads the polished scene Markdown files from a prior full-pipeline run,
runs each through BooksForge's `humanization/v1.toml` template on a heavy
model (qwen3.5:27b), parses the returned `HumanizationProposals` JSON,
and applies the highest-leverage edits in order. Produces a third-tier
refined manuscript next to the two-tier baseline.

The humanization agent flags AI-tells (cliché vocabulary, stock discourse
markers, triad terms, uniform sentence cadence, generic emotion words,
system-prompt residue) and proposes concrete `before/after` replacements
within ±10% word-count budget per scene. We apply the proposals as
literal find-and-replace, in declaration order; conflicting overlapping
edits are skipped.

Privacy: only outbound network call is to 127.0.0.1:11434.
"""

from __future__ import annotations

import json
import re
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

DEFAULT_INPUT = REPO / "book-output/booksforge-ollama-full-run"
DEFAULT_OUTPUT = REPO / "book-output/booksforge-ollama-full-run-humanized"


def chat_no_thinking(model, system, user, *, temperature, max_tokens, json_mode=False, timeout=900):
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
    return data.get("message", {}).get("content", ""), {
        "elapsed_s": round(elapsed, 2),
        "eval_count": data.get("eval_count"),
    }


def apply_edits(prose: str, edits: list[dict]) -> tuple[str, int, int]:
    """Apply humanization edits in declaration order. Skip overlaps and edits where `before` doesn't appear.

    Returns (new_prose, applied, skipped).
    """
    applied = 0
    skipped = 0
    out = prose
    used_spans: list[tuple[int, int]] = []
    for edit in edits:
        before = (edit.get("before") or "").strip()
        after = edit.get("after") or ""
        if not before:
            skipped += 1
            continue
        # Locate the first occurrence not overlapping anything we've already touched.
        idx = 0
        while True:
            found = out.find(before, idx)
            if found == -1:
                skipped += 1
                break
            end = found + len(before)
            overlaps = any(not (end <= s or found >= e) for s, e in used_spans)
            if overlaps:
                idx = end
                continue
            out = out[:found] + after + out[end:]
            used_spans.append((found, found + len(after)))
            # Shift later spans
            shift = len(after) - len(before)
            used_spans = [(s + shift if s >= end else s, e + shift if e >= end else e) for s, e in used_spans[:-1]] + [used_spans[-1]]
            applied += 1
            break
    return out, applied, skipped


def humanize_scene(*, scene_path: Path, polish_model: str, brief: dict) -> tuple[str, dict]:
    prose = scene_path.read_text().strip()
    rendered = render_template(
        "humanization",
        {
            "scene_text": prose,
            "scene_title": scene_path.stem,
            "active_avoid_rules": [],
            "voice_fingerprint": {"corpus_tokens": 0},
            "prompt_guard": "",
        },
    )
    word_count = len(prose.split())
    max_tok = max(2048, int(word_count * 1.5))
    raw, meta = chat_no_thinking(
        polish_model, rendered.system, rendered.user,
        temperature=0.2, max_tokens=max_tok, json_mode=True,
    )
    try:
        proposals = extract_json(raw)
    except Exception as e:
        return prose, {"applied": 0, "skipped": 0, "elapsed_s": meta["elapsed_s"], "error": f"{type(e).__name__}: {e}"}
    edits = proposals.get("edits", []) if isinstance(proposals, dict) else []
    new_prose, applied, skipped = apply_edits(prose, edits)
    return new_prose, {
        "applied": applied,
        "skipped": skipped,
        "edits_proposed": len(edits),
        "elapsed_s": meta["elapsed_s"],
        "eval_count": meta["eval_count"],
    }


def main() -> int:
    args = sys.argv[1:]
    polish_model = args[0] if args else "qwen3.5:27b"
    in_dir = Path(args[1]) if len(args) > 1 else DEFAULT_INPUT
    out_dir = Path(args[2]) if len(args) > 2 else DEFAULT_OUTPUT

    if not (in_dir / "01-brief.json").exists():
        print(f"FATAL: no brief at {in_dir}/01-brief.json — point at a completed full-pipeline run", file=sys.stderr)
        return 1
    brief = json.loads((in_dir / "01-brief.json").read_text())

    out_dir.mkdir(parents=True, exist_ok=True)
    (out_dir / "scenes-humanized").mkdir(exist_ok=True)
    (out_dir / "chapters").mkdir(exist_ok=True)

    log_lines: list[str] = []

    def log(msg: str) -> None:
        ts = time.strftime("%H:%M:%S")
        line = f"[{ts}] {msg}"
        print(line, flush=True)
        log_lines.append(line)
        with open(out_dir / "humanize.log", "w") as f:
            f.write("\n".join(log_lines))

    log(f"Humanization post-pass | polish_model={polish_model}")
    log(f"Input:  {in_dir}")
    log(f"Output: {out_dir}")

    probe = ollama_probe()
    log(f"Ollama version={probe['version']} | {len(probe['models'])} local models")

    polished_dir = in_dir / "scenes-polished"
    scene_files = sorted(polished_dir.glob("ch*.md"))
    log(f"Found {len(scene_files)} polished scenes")

    summary: list[dict] = []
    chapter_buckets: dict[str, list[str]] = {}

    for sp in scene_files:
        ch_id = sp.stem.split("-")[0]  # ch01-s01 -> ch01
        log(f"Humanizing {sp.stem}…")
        new_prose, meta = humanize_scene(scene_path=sp, polish_model=polish_model, brief=brief)
        target_path = out_dir / "scenes-humanized" / sp.name
        target_path.write_text(new_prose + "\n")
        summary.append({"scene": sp.stem, **meta})
        chapter_buckets.setdefault(ch_id, []).append(new_prose)
        log(f"  applied={meta.get('applied')} proposed={meta.get('edits_proposed')} skipped={meta.get('skipped')} elapsed={meta.get('elapsed_s')}s")

    # Reassemble per-chapter Markdown using the same titles as the input run.
    outline = json.loads((in_dir / "02-outline.json").read_text())
    chapters_in_order: list[tuple[str, str]] = []
    ci = 0
    for part in outline.get("parts", []):
        for ch in part.get("chapters", []):
            ci += 1
            chapters_in_order.append((f"ch{ci:02d}", ch.get("title", f"Chapter {ci}")))

    combined: list[str] = [f"# {brief.get('title_suggestions',['Untitled'])[0]} (humanized)\n"]
    for ch_id, ch_title in chapters_in_order:
        scenes = chapter_buckets.get(ch_id)
        if not scenes:
            continue
        chapter_md = [f"## {ch_title}", ""]
        chapter_md.extend(scenes)
        chapter_text = "\n\n".join(chapter_md).rstrip() + "\n"
        (out_dir / "chapters" / f"{ch_id}.md").write_text(chapter_text)
        combined.append(chapter_text)
        combined.append("\n---\n")

    (out_dir / "FULL_MANUSCRIPT_HUMANIZED.md").write_text("\n".join(combined).rstrip() + "\n")
    (out_dir / "humanize-summary.json").write_text(json.dumps({
        "polish_model": polish_model,
        "input_run": str(in_dir),
        "scenes_processed": len(summary),
        "scenes": summary,
    }, indent=2))
    log(f"Done. Humanized manuscript at {out_dir / 'FULL_MANUSCRIPT_HUMANIZED.md'}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
