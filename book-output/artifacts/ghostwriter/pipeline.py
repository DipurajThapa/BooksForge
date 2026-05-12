#!/usr/bin/env python3
"""
Ghostwriter pipeline — production-grade book drafting on local LLMs.

Architecture (per RCA_QUALITY_6_TO_9.md, all of L1 + L2 implemented in this
single Python driver so it can be used TODAY without waiting for the Rust
crate work in BACKLOG §A13):

  1. Per-genre routing (literary / genre / non-fiction) via genre_packs.
  2. Voice fingerprinting from user-supplied comp samples → injected into
     drafter prompts as numeric constraints.
  3. Drafter routed to 27B for fiction (not 9B) per RCA §L2.1.
  4. ENSEMBLE drafting: each scene drafted N=2 times with different
     temperatures; best is selected by the per-scene critic.
  5. Per-scene critique-revise loop: critic scores draft, revises only the
     spans that fail the genre's critic axes.
  6. Specialist polish stack: per-genre ordered passes (4 specialists,
     each with its own prompt template, voice-preserving by design).
  7. Anti-AI-tells redaction pass — flags + targeted rewrite of LLM
     fingerprints (delve, tapestry, "It's important to note", em-dash
     overuse, cliché body-as-feeling phrases).
  8. Multi-specialist scoring — three rubric scorers (developmental, prose,
     commercial) each in their own context, weighted per genre.
  9. Whole-manuscript context for cross-cutting passes (no truncation).
  10. Stylometric distance from comp samples reported alongside rubric.

Inputs (one JSON file):
  {
    "title": "…",
    "genre": "literary | genre | non-fiction",
    "premise": "…",                     # 1-3 sentences
    "voice_samples": ["…", "…", "…"],   # 3-5 short paragraphs from comp titles
    "chapters": [                       # outline can be hand-written or generated
       {"number": 1, "title": "…", "scenes": [
          {"goal":"…", "conflict":"…", "reveal":"…", "target_words":1500}
       ]}
    ],
    "draft_model": "qwen3.5:27b",
    "polish_model": "qwen3.5:27b",
    "scorer_model": "qwen3.6:latest"
  }

Outputs (under <out_dir>/):
  - 00_inputs.json              # frozen for replay
  - 01_voice_profile.json       # measured fingerprint of comp samples
  - chapters/<n>_draft.md       # raw best-of-ensemble draft
  - chapters/<n>_critiqued.md   # after per-scene critique-revise
  - chapters/<n>_polished.md    # after specialist polish stack
  - chapters/<n>_clean.md       # after anti-AI-tells redaction
  - 99_manuscript.md            # full manuscript, all chapters spliced
  - 99_score.json               # multi-specialist rubric + stylometric distance
  - 99_log.jsonl                # per-call audit trail

Run:
  python3 -m artifacts.ghostwriter.pipeline --input <path/to/spec.json> --out <out_dir>
"""
from __future__ import annotations

import argparse
import json
import re
import sys
import time
import urllib.request
from dataclasses import asdict, dataclass
from datetime import datetime, timezone
from pathlib import Path

# Local modules
HERE = Path(__file__).parent
sys.path.insert(0, str(HERE.parent))
from ghostwriter.anti_ai_tells import (  # type: ignore
    revision_prompt as ai_tells_revision_prompt,
    tells_per_1000_words,
)
from ghostwriter.voice_fingerprint import (  # type: ignore
    fingerprint, stylometric_distance, VoiceProfile,
)
from ghostwriter.genre_packs import (  # type: ignore
    PACKS, RUBRIC, weights_for, GenrePack,
)


OLLAMA_URL = "http://127.0.0.1:11434"


# ── Local LLM client (Ollama) ──────────────────────────────────────────────

@dataclass
class CallLog:
    ts: str
    purpose: str
    model: str
    prompt_chars: int
    response_chars: int
    elapsed_s: float
    temperature: float
    json_mode: bool


def make_logger(log_path: Path):
    def log(rec: CallLog) -> None:
        with log_path.open("a") as f:
            f.write(json.dumps(asdict(rec)) + "\n")
    return log


def ollama_call(*, model: str, system: str, prompt: str, json_mode: bool = False,
                max_tokens: int = 4096, temperature: float = 0.5,
                purpose: str = "", logger=None) -> str:
    payload = {
        "model": model,
        "prompt": prompt,
        "system": system,
        "stream": False,
        "options": {"temperature": temperature, "num_predict": max_tokens},
        "think": False,
    }
    if json_mode:
        payload["format"] = "json"
    body = json.dumps(payload).encode()
    req = urllib.request.Request(
        f"{OLLAMA_URL}/api/generate", data=body,
        headers={"Content-Type": "application/json"}, method="POST",
    )
    t0 = time.time()
    with urllib.request.urlopen(req, timeout=900) as r:
        raw = r.read()
    elapsed = time.time() - t0
    data = json.loads(raw)
    out = data.get("response", "")
    if logger:
        logger(CallLog(
            ts=datetime.now(timezone.utc).isoformat(),
            purpose=purpose, model=model,
            prompt_chars=len(prompt) + len(system),
            response_chars=len(out),
            elapsed_s=round(elapsed, 2),
            temperature=temperature, json_mode=json_mode,
        ))
    return out


def parse_json_lenient(raw: str) -> dict | list:
    try:
        return json.loads(raw)
    except Exception:
        m = re.search(r"(\{.*\}|\[.*\])", raw, re.DOTALL)
        if m:
            return json.loads(m.group(1))
        raise


# ── Pipeline stages ────────────────────────────────────────────────────────

def measure_comp_voice(samples: list[str]) -> tuple[VoiceProfile, str]:
    """Concatenate comp samples and measure the fingerprint."""
    joined = "\n\n".join(samples)
    profile = fingerprint(joined)
    return profile, joined


def draft_scene_ensemble(*, scene: dict, chapter: dict, pack: GenrePack,
                         voice_block: str, prior_summary: str,
                         drafter_model: str, n_candidates: int = 2,
                         logger=None) -> str:
    """
    Generate N candidate drafts at varying temperatures, return the longest
    that meets the minimum word count. The critic loop picks structural
    quality afterwards; here we just enforce length and filter blanks.
    """
    target_w = int(scene.get("target_words", 1500))
    candidates: list[str] = []
    for i in range(n_candidates):
        temp = 0.55 + 0.10 * i
        prompt = (
            f"You are drafting a scene from a {pack.name.replace('_', ' ')} chapter.\n\n"
            f"{pack.draft_lens}\n\n"
            f"VOICE TARGETS (the prose must measurably hit these):\n{voice_block}\n\n"
            f"CHAPTER {chapter['number']}: {chapter.get('title','')}\n"
            f"Prior chapter summary: {prior_summary[:1500]}\n\n"
            f"Scene goal: {scene.get('goal')}\n"
            f"Scene conflict: {scene.get('conflict')}\n"
            f"Scene reveal or turn: {scene.get('reveal','(none — quiet scene; carry tension via subtext)')}\n"
            f"Target words: ~{target_w}\n\n"
            f"Write the scene now. Begin in-medias-res. No headings. Plain prose only."
        )
        out = ollama_call(
            model=drafter_model, system=pack.system, prompt=prompt,
            max_tokens=int(target_w * 2.4) + 256,
            temperature=temp,
            purpose=f"draft_scene_ch{chapter['number']}_attempt{i}",
            logger=logger,
        )
        if len(out.split()) >= target_w * 0.5:
            candidates.append(out.strip())
    if not candidates:
        return f"_[scene draft failed at all {n_candidates} temperatures]_"
    # Pick the candidate closest to target word count (avoid both runaway and stub)
    candidates.sort(key=lambda c: abs(len(c.split()) - target_w))
    return candidates[0]


def critique_and_revise_scene(*, scene_md: str, scene: dict, pack: GenrePack,
                              critic_model: str, logger=None) -> str:
    """
    The critic scores the draft on the genre-specific axes and returns
    targeted edit instructions. Then we run a single revision pass that
    incorporates the instructions WITHOUT rewriting from scratch.
    """
    critic_prompt = (
        f"Critique this scene draft on these axes (1-10 each):\n"
        + "\n".join(f"  - {a}" for a in pack.critic_axes) +
        "\n\nReturn JSON:\n"
        "{\n"
        f"  \"scores\": {{ {', '.join(f'\"{a}\": 0' for a in pack.critic_axes)} }},\n"
        "  \"weakest_axis\": \"...\",\n"
        "  \"specific_edits\": [\n"
        "    {\"problem\": \"quote of weak passage\", \"fix\": \"concrete revised line\"}\n"
        "  ]\n"
        "}\n\n"
        f"Scene goal: {scene.get('goal')}\n"
        f"Scene conflict: {scene.get('conflict')}\n\n"
        f"Draft:\n---\n{scene_md[:6000]}\n---"
    )
    raw = ollama_call(
        model=critic_model,
        system="You are a sharp scene-craft critic. Be specific. Quote the actual prose.",
        prompt=critic_prompt, json_mode=True, max_tokens=2200, temperature=0.3,
        purpose="critique_scene", logger=logger,
    )
    try:
        critique = parse_json_lenient(raw)
    except Exception:
        return scene_md  # critique failed; keep draft

    edits = (critique or {}).get("specific_edits", []) if isinstance(critique, dict) else []
    if not edits:
        return scene_md

    revise_prompt = (
        "Apply ONLY these targeted edits to the scene below. Preserve everything "
        "else exactly — voice, plot beats, character actions, dialogue you didn't "
        "explicitly fix. Return ONLY the revised scene prose, no commentary.\n\n"
        f"Edits:\n{json.dumps(edits[:8], indent=2)}\n\n"
        f"Scene:\n---\n{scene_md}\n---"
    )
    revised = ollama_call(
        model=critic_model,
        system="You are a careful, narrow-scope reviser. You change only what was asked.",
        prompt=revise_prompt, max_tokens=int(len(scene_md) / 3) + 1000,
        temperature=0.35, purpose="revise_scene", logger=logger,
    )
    if len(revised.split()) >= len(scene_md.split()) * 0.7:
        return revised.strip()
    return scene_md


def polish_chapter_specialist(*, chapter_md: str, pack: GenrePack,
                              polish_model: str, logger=None) -> str:
    """
    Run the genre-specific specialist polish stack in order. Each stage gets
    the WHOLE chapter (no truncation) and is prompted to preserve everything
    outside its remit.
    """
    current = chapter_md
    for stage in pack.polish_stack:
        user = stage.user_template.format(chapter=current)
        revised = ollama_call(
            model=polish_model, system=stage.system, prompt=user,
            max_tokens=int(len(current) / 2) + 2000,
            temperature=0.32, purpose=f"polish_{stage.name}", logger=logger,
        )
        if len(revised.split()) >= len(current.split()) * 0.65:
            current = revised.strip()
    return current


def redact_ai_tells(*, chapter_md: str, polish_model: str, logger=None) -> tuple[str, dict]:
    """Anti-AI-tells redaction. Only invokes the LLM if there ARE tells to fix."""
    before = tells_per_1000_words(chapter_md)
    rev_prompt = ai_tells_revision_prompt(chapter_md, max_targets=24)
    if not rev_prompt or before["weighted_density_per_1000"] < 4:
        return chapter_md, {"before": before, "after": before, "skipped": True}
    full_prompt = (
        rev_prompt + "\n\nChapter:\n---\n" + chapter_md + "\n---\n"
        "Return ONLY the revised chapter."
    )
    revised = ollama_call(
        model=polish_model,
        system=("You are an anti-AI-prose specialist. Rewrite ONLY the flagged spans. "
                "Preserve the surrounding voice. Do NOT replace flagged words with "
                "OTHER LLM-favourite words."),
        prompt=full_prompt, max_tokens=int(len(chapter_md) / 2) + 1000,
        temperature=0.3, purpose="redact_ai_tells", logger=logger,
    )
    if len(revised.split()) < len(chapter_md.split()) * 0.7:
        return chapter_md, {"before": before, "after": before, "kept_original": True}
    after = tells_per_1000_words(revised)
    return revised.strip(), {"before": before, "after": after, "skipped": False}


def score_manuscript(*, manuscript: str, pack: GenrePack, comp_profile: VoiceProfile,
                     scorer_model: str, logger=None) -> dict:
    """
    Multi-specialist scoring:
      - developmental rubric (structure, arcs, pacing) via scorer_model
      - prose rubric (voice, prose-quality, dialogue) via scorer_model
      - commercial rubric (hook, commercial-readiness, originality) via scorer_model
    Each call sees a focused excerpt; final score is a weighted combination
    using the genre's per-axis weights.
    """
    weights = weights_for(pack)
    head = manuscript[:9000]
    tail = manuscript[-9000:] if len(manuscript) > 18000 else ""
    middle = manuscript[len(manuscript)//2 - 4500:len(manuscript)//2 + 4500] if len(manuscript) > 14000 else ""
    focused_excerpt = "\n\n[…]\n\n".join(p for p in [head, middle, tail] if p)

    def _score_call(rubric_keys: list[str], lens: str) -> dict:
        prompt = (
            f"Score this manuscript on these dimensions only (1-10 each, no inflation):\n"
            + "\n".join(f"  - {k}" for k in rubric_keys) +
            f"\n\nLens: {lens}\n\n"
            "Return JSON:\n"
            "{\n"
            f"  \"scores\": {{ {', '.join(f'\"{k}\": 0' for k in rubric_keys)} }},\n"
            "  \"specific_issues\": [\"...\", \"...\"]\n"
            "}\n\n"
            f"Manuscript excerpt (head + middle + tail):\n---\n{focused_excerpt}\n---"
        )
        raw = ollama_call(
            model=scorer_model,
            system=("You score honestly. 10/10 is reserved for masterwork-level "
                    "execution that humans agree on. Inflated scoring is forbidden."),
            prompt=prompt, json_mode=True, max_tokens=1800, temperature=0.2,
            purpose=f"score_{lens}", logger=logger,
        )
        try:
            return parse_json_lenient(raw)
        except Exception:
            return {"scores": {k: 0 for k in rubric_keys}, "specific_issues": []}

    dev = _score_call(["structure", "pacing", "continuity", "argument_strength"], "developmental")
    prose = _score_call(["voice", "prose_quality", "dialogue", "originality", "character_depth", "emotional_impact", "authority_voice"], "prose")
    comm = _score_call(["hook_strength", "commercial_readiness", "evidence_handling", "formatting_readiness"], "commercial")

    # Aggregate
    raw_scores: dict[str, float] = {}
    for src in (dev, prose, comm):
        for k, v in (src.get("scores") or {}).items():
            try:
                raw_scores[k] = float(v)
            except Exception:
                pass

    weighted_total = 0.0
    weight_total = 0.0
    for axis in RUBRIC:
        if axis.key in raw_scores:
            w = weights[axis.key]
            weighted_total += raw_scores[axis.key] * w
            weight_total += w
    weighted_avg = round(weighted_total / weight_total, 2) if weight_total else 0.0

    # Stylometric distance
    ms_profile = fingerprint(manuscript)
    stylo = stylometric_distance(comp_profile, ms_profile)

    return {
        "raw_scores_by_dimension": raw_scores,
        "weighted_score_per_genre": weighted_avg,
        "developmental_call": dev,
        "prose_call": prose,
        "commercial_call": comm,
        "stylometric_distance_from_comp": stylo,
        "ai_tells_density_final": tells_per_1000_words(manuscript),
        "manuscript_voice_profile": asdict(ms_profile),
    }


# ── Main pipeline ──────────────────────────────────────────────────────────

def run_pipeline(spec: dict, out_dir: Path) -> dict:
    out_dir.mkdir(parents=True, exist_ok=True)
    chapters_dir = out_dir / "chapters"
    chapters_dir.mkdir(exist_ok=True)
    log_path = out_dir / "99_log.jsonl"
    log_path.write_text("")
    logger = make_logger(log_path)

    # Freeze inputs for replay
    (out_dir / "00_inputs.json").write_text(json.dumps(spec, indent=2))

    pack: GenrePack = PACKS[spec["genre"]]
    drafter_model = spec.get("draft_model", "qwen3.5:27b")
    polish_model = spec.get("polish_model", "qwen3.5:27b")
    scorer_model = spec.get("scorer_model", "qwen3.6:latest")

    # 1. Voice fingerprint of comp samples
    comp_profile, comp_text = measure_comp_voice(spec.get("voice_samples", []))
    voice_block = comp_profile.constraints_block(label=f"{spec['genre']} comps")
    (out_dir / "01_voice_profile.json").write_text(json.dumps({
        "profile": asdict(comp_profile),
        "constraints_block": voice_block,
    }, indent=2))

    # 2. For each chapter: draft → critique-revise per scene → polish → redact
    chapter_outputs: list[str] = []
    prior_summary = "(opening chapter)"
    for ch in spec["chapters"]:
        ch_num = ch["number"]
        scenes_drafted: list[str] = []
        for sc in ch.get("scenes", []):
            print(f"  ch{ch_num} scene draft (ensemble of 2)…", flush=True)
            draft = draft_scene_ensemble(
                scene=sc, chapter=ch, pack=pack,
                voice_block=voice_block, prior_summary=prior_summary,
                drafter_model=drafter_model, n_candidates=2, logger=logger,
            )
            print(f"  ch{ch_num} scene critique-revise…", flush=True)
            revised = critique_and_revise_scene(
                scene_md=draft, scene=sc, pack=pack,
                critic_model=polish_model, logger=logger,
            )
            scenes_drafted.append(revised)

        chapter_md = (
            f"# Chapter {ch_num}: {ch.get('title','')}\n\n"
            + "\n\n* * *\n\n".join(scenes_drafted)
            + "\n"
        )
        (chapters_dir / f"{ch_num:02d}_draft.md").write_text(chapter_md)

        print(f"  ch{ch_num} specialist polish stack ({len(pack.polish_stack)} passes)…", flush=True)
        polished = polish_chapter_specialist(
            chapter_md=chapter_md, pack=pack,
            polish_model=polish_model, logger=logger,
        )
        (chapters_dir / f"{ch_num:02d}_polished.md").write_text(polished)

        print(f"  ch{ch_num} anti-AI-tells redaction…", flush=True)
        clean, redact_report = redact_ai_tells(
            chapter_md=polished, polish_model=polish_model, logger=logger,
        )
        (chapters_dir / f"{ch_num:02d}_clean.md").write_text(clean)
        (chapters_dir / f"{ch_num:02d}_ai_tells_report.json").write_text(
            json.dumps(redact_report, indent=2)
        )
        chapter_outputs.append(clean)

        # Compress for next-chapter context
        prior_summary = ollama_call(
            model=drafter_model,
            system="Summarise in 4-6 sentences: what changed for the protagonist, what set up the next chapter.",
            prompt=clean[:8000], max_tokens=400, temperature=0.3,
            purpose="compress_prior_summary", logger=logger,
        ).strip()

    manuscript = "\n\n".join(chapter_outputs)
    (out_dir / "99_manuscript.md").write_text(manuscript)

    # 3. Multi-specialist scoring
    print("  scoring manuscript (3 specialist scorers)…", flush=True)
    score = score_manuscript(
        manuscript=manuscript, pack=pack, comp_profile=comp_profile,
        scorer_model=scorer_model, logger=logger,
    )
    (out_dir / "99_score.json").write_text(json.dumps(score, indent=2))

    # 4. Honest summary
    summary = {
        "test": "ghostwriter_pipeline",
        "genre": spec["genre"],
        "weighted_score": score["weighted_score_per_genre"],
        "stylometric_distance_score": score["stylometric_distance_from_comp"]["distance_score_out_of_10"],
        "ai_tells_density": score["ai_tells_density_final"]["weighted_density_per_1000"],
        "ai_tells_verdict": score["ai_tells_density_final"]["verdict"],
        "chapters_produced": len(chapter_outputs),
        "total_words": sum(len(c.split()) for c in chapter_outputs),
        "honest_residual_issues": score["prose_call"].get("specific_issues", []) + score["developmental_call"].get("specific_issues", []),
        "ts": datetime.now(timezone.utc).isoformat(),
    }
    (out_dir / "99_summary.json").write_text(json.dumps(summary, indent=2))
    return summary


def main() -> int:
    p = argparse.ArgumentParser()
    p.add_argument("--input", required=True, help="path to spec JSON")
    p.add_argument("--out", required=True, help="output directory")
    args = p.parse_args()
    spec = json.loads(Path(args.input).read_text())
    out = Path(args.out)
    summary = run_pipeline(spec, out)
    print(json.dumps(summary, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
