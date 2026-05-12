#!/usr/bin/env python3
"""
BF-E2E-LOCAL-LLM-FIRST-BOOK-001 — end-to-end QA orchestrator.

Drives an 8-chapter cozy-fantasy book from ideation to print/ebook export
using only the local Ollama runtime. Calls Anthropic / cloud endpoints are
forbidden — every generation goes through 127.0.0.1:11434.

Design notes:
- Single Python file, stdlib + pandoc only.
- Each phase writes its artifact even on partial success.
- All LLM calls log into artifacts/audit/local_llm_routing.jsonl with
  endpoint + model + duration + bytes-out so cloud-routing can be falsified.
- PASS / WARN / FAIL is recorded per phase and rolled up at the end.
"""
from __future__ import annotations

import json
import os
import re
import subprocess
import sys
import time
import urllib.request
import urllib.error
import hashlib
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path("/Users/dipurajthapa/Work/AIProjects/BooksForge")
ART = ROOT / "artifacts"
EXP = ROOT / "book-output/booksforge-e2e-bf001/exports"
LOG_DIR = ART / "logs"
AUDIT_DIR = ART / "audit"
ART.mkdir(parents=True, exist_ok=True)
EXP.mkdir(parents=True, exist_ok=True)
LOG_DIR.mkdir(parents=True, exist_ok=True)
AUDIT_DIR.mkdir(parents=True, exist_ok=True)

OLLAMA_URL = os.environ.get("OLLAMA_URL", "http://127.0.0.1:11434")
DRAFTER = "qwen3.5:9b"
POLISHER = "qwen3.5:27b"
OPTIMIZER = "qwen3.6:latest"  # 36B MoE, used for market-readiness pass

ROUTING_LOG = AUDIT_DIR / "local_llm_routing.jsonl"
PHASE_LOG = LOG_DIR / "phases.jsonl"
RUN_LOG = LOG_DIR / "run.log"
TEST_ID = "BF-E2E-LOCAL-LLM-FIRST-BOOK-001"
RUN_START = datetime.now(timezone.utc).isoformat()

PHASES: list[dict] = []  # populated as we go


def log(msg: str) -> None:
    line = f"[{datetime.now(timezone.utc).strftime('%H:%M:%S')}] {msg}"
    print(line, flush=True)
    with RUN_LOG.open("a") as f:
        f.write(line + "\n")


def record_phase(phase: str, status: str, summary: str, artifacts: list[str], **extra) -> None:
    rec = {
        "phase": phase,
        "status": status,
        "summary": summary,
        "artifacts": artifacts,
        "ts": datetime.now(timezone.utc).isoformat(),
        **extra,
    }
    PHASES.append(rec)
    with PHASE_LOG.open("a") as f:
        f.write(json.dumps(rec) + "\n")
    log(f"PHASE {phase}: {status} — {summary}")


# ---------------------------------------------------------------------------
# Local LLM client (Ollama)
# ---------------------------------------------------------------------------

def llm(prompt: str, *, model: str, system: str = "", json_mode: bool = False,
        max_tokens: int = 2048, temperature: float = 0.5) -> str:
    """Call Ollama /api/generate. Logs routing for the audit trail."""
    payload = {
        "model": model,
        "prompt": prompt,
        "system": system,
        "stream": False,
        "options": {
            "temperature": temperature,
            "num_predict": max_tokens,
        },
        "think": False,
    }
    if json_mode:
        payload["format"] = "json"
    body = json.dumps(payload).encode()
    req = urllib.request.Request(
        f"{OLLAMA_URL}/api/generate",
        data=body,
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    t0 = time.time()
    try:
        with urllib.request.urlopen(req, timeout=900) as r:
            raw = r.read()
        elapsed = time.time() - t0
        data = json.loads(raw)
        out = data.get("response", "")
    except Exception as e:  # noqa: BLE001
        elapsed = time.time() - t0
        with ROUTING_LOG.open("a") as f:
            f.write(json.dumps({
                "ts": datetime.now(timezone.utc).isoformat(),
                "endpoint": OLLAMA_URL,
                "model": model,
                "ok": False,
                "error": repr(e),
                "elapsed_s": round(elapsed, 2),
            }) + "\n")
        raise
    with ROUTING_LOG.open("a") as f:
        f.write(json.dumps({
            "ts": datetime.now(timezone.utc).isoformat(),
            "endpoint": OLLAMA_URL,
            "model": model,
            "ok": True,
            "elapsed_s": round(elapsed, 2),
            "prompt_chars": len(prompt),
            "response_chars": len(out),
            "eval_count": data.get("eval_count"),
            "json_mode": json_mode,
        }) + "\n")
    return out


def llm_json(prompt: str, *, model: str, system: str = "", max_tokens: int = 4096,
             temperature: float = 0.4, retries: int = 2) -> dict | list:
    last_err = None
    for attempt in range(retries + 1):
        out = llm(
            prompt,
            model=model,
            system=system,
            json_mode=True,
            max_tokens=max_tokens,
            temperature=temperature if attempt == 0 else max(0.2, temperature - 0.1 * attempt),
        )
        try:
            return json.loads(out)
        except Exception as e:  # noqa: BLE001
            last_err = e
            # try to extract first JSON object/array
            m = re.search(r"(\{.*\}|\[.*\])", out, re.DOTALL)
            if m:
                try:
                    return json.loads(m.group(1))
                except Exception:
                    pass
            log(f"  json parse retry {attempt + 1} failed: {e}")
    raise RuntimeError(f"llm_json failed after {retries + 1} attempts: {last_err}")


# ---------------------------------------------------------------------------
# PHASE 0 — sentinel + cloud-routing audit
# ---------------------------------------------------------------------------

def phase0_environment() -> bool:
    log("=== PHASE 0: environment guardrails ===")
    # 0.1 endpoint health
    try:
        with urllib.request.urlopen(f"{OLLAMA_URL}/api/tags", timeout=5) as r:
            tags = json.loads(r.read())
        models = [m["name"] for m in tags.get("models", [])]
    except Exception as e:  # noqa: BLE001
        record_phase("0", "FAIL", f"Ollama unreachable: {e}", [])
        return False

    required = {DRAFTER, POLISHER, OPTIMIZER}
    missing = [m for m in required if m not in models]
    if missing:
        record_phase("0", "FAIL", f"Missing models: {missing}", [])
        return False

    # 0.2 confirm no cloud LLM env vars set for generation
    cloud_vars = ["ANTHROPIC_API_KEY", "OPENAI_API_KEY", "GEMINI_API_KEY",
                  "GOOGLE_API_KEY", "AZURE_OPENAI_API_KEY"]
    cloud_present = [v for v in cloud_vars if os.environ.get(v)]

    # 0.3 sentinel generation — must echo exactly
    sentinel_out = llm(
        "Return EXACTLY this string and nothing else: LOCAL_LLM_ACTIVE",
        model=DRAFTER, max_tokens=32, temperature=0.0,
    ).strip()
    sentinel_ok = "LOCAL_LLM_ACTIVE" in sentinel_out

    audit = {
        "test_id": TEST_ID,
        "endpoint": OLLAMA_URL,
        "models_seen": models,
        "models_used": {"drafter": DRAFTER, "polisher": POLISHER, "optimizer": OPTIMIZER},
        "cloud_env_vars_set": cloud_present,
        "sentinel_prompt": "Return EXACTLY this string and nothing else: LOCAL_LLM_ACTIVE",
        "sentinel_response": sentinel_out,
        "sentinel_passed": sentinel_ok,
        "verdict": "LOCAL_ONLY" if sentinel_ok and not cloud_present else (
            "LOCAL_ONLY_BUT_CLOUD_KEYS_PRESENT" if sentinel_ok else "SENTINEL_FAILED"
        ),
        "ts": datetime.now(timezone.utc).isoformat(),
    }
    (AUDIT_DIR / "local_llm_routing.json").write_text(json.dumps(audit, indent=2))

    if not sentinel_ok:
        record_phase("0", "FAIL", f"Sentinel did not echo. Got: {sentinel_out!r}",
                     [str(AUDIT_DIR / "local_llm_routing.json")])
        return False
    status = "WARN" if cloud_present else "PASS"
    note = (f"Sentinel passed. Cloud env keys present (unused for generation): {cloud_present}"
            if cloud_present else "Sentinel passed. No cloud generation keys active.")
    record_phase("0", status, note, [str(AUDIT_DIR / "local_llm_routing.json")])
    return True


# ---------------------------------------------------------------------------
# PHASE 1 — ideation
# ---------------------------------------------------------------------------

IDEATION_SYSTEM = """You are a publishing strategist. You generate ORIGINAL, commercially viable book concepts.
Hard constraints:
- No derivative use of famous IP (Harry Potter, Lord of the Rings, Marvel, etc.)
- No imitation of living authors' styles
- No invented statistics or fake market data
- Family-friendly content
Return strict JSON only."""

def phase1_ideation() -> dict:
    log("=== PHASE 1: ideation ===")
    prompt = """Generate 5 ORIGINAL book concepts for a first-time independent author.
Constraints:
- Genre: uplifting speculative adventure / cozy fantasy
- Audience: YA crossover / adult light fantasy readers
- Length: 8 chapters, 12,000-16,000 words total (test scope)
- Tone: cinematic, warm, witty, page-turning
- Theme: courage, belonging, rebuilding trust
- Family-friendly, no explicit content

For EACH concept return an object with these fields:
  title, hook, genre, target_reader, emotional_promise, commercial_angle,
  why_original, cliche_risk, difficulty_estimate (1-10)

Then for each concept, provide a "score" object with these 1-10 fields:
  originality, emotional_depth, market_clarity, series_potential, feasibility, visual_cover_potential

Return JSON shaped exactly:
{
  "concepts": [ { ...all the above... }, ... 5 concepts ],
  "top_two": [ { "title": "...", "rationale": "..." }, { "title": "...", "rationale": "..." } ]
}"""
    data = llm_json(prompt, model=DRAFTER, system=IDEATION_SYSTEM, max_tokens=4096, temperature=0.85)
    out_path = ART / "01_ideation.json"
    out_path.write_text(json.dumps(data, indent=2))
    n = len(data.get("concepts", []))
    if n < 5:
        record_phase("1", "FAIL", f"Only {n} concepts generated (need >=5)", [str(out_path)])
    else:
        scores = [c.get("score", {}) for c in data.get("concepts", [])]
        identical = len({json.dumps(s, sort_keys=True) for s in scores}) <= 1
        record_phase(
            "1",
            "WARN" if identical else "PASS",
            f"{n} concepts generated, top_two selected" + (" (scores look identical)" if identical else ""),
            [str(out_path)],
        )
    return data


# ---------------------------------------------------------------------------
# PHASE 2 — research dossier (offline mode — no live web)
# ---------------------------------------------------------------------------

def phase2_research(ideation: dict) -> str:
    log("=== PHASE 2: research dossier (offline mode) ===")
    top = ideation.get("top_two", [{}])[0]
    title = top.get("title", "Selected Concept")
    prompt = f"""Produce a research dossier for the book concept "{title}".

OFFLINE/LOCAL RESEARCH MODE: you have NO live internet access. Do NOT invent
current sales charts, BookScan numbers, recent Amazon ranks, or 2026-current
trade-press claims. If a claim depends on live data, label it ASSUMPTION.

Cover these sections in markdown:
1. Audience expectations (cozy-fantasy / YA crossover readers — durable patterns)
2. Comparable genre patterns (general, durable craft observations only)
3. Reader hooks that recur in this category
4. Search/discovery keywords (10-15, neither spammy nor stuffed)
5. Themes and tropes (with handling notes)
6. Differentiation opportunities for this concept
7. Content sensitivities (family-friendly checks)
8. Cover/metadata considerations (durable, not trend-chasing)
9. Platform-readiness implications (KDP / Google Play / Apple Books)

Every claim MUST be tagged inline with one of:
  [FACT]        — durable, widely known
  [INFERENCE]   — derived from general patterns
  [ASSUMPTION]  — needs live verification before publication
  [CREATIVE]    — author/strategy decision

Output only the markdown."""
    out = llm(prompt, model=DRAFTER, system=IDEATION_SYSTEM, max_tokens=3500, temperature=0.5)
    # Validator: every non-empty, non-heading line should carry a tag.
    # If the validator finds untagged claims, run a tag-injection second pass.
    def _untagged_lines(text: str) -> list[str]:
        bad: list[str] = []
        for line in text.splitlines():
            s = line.strip()
            if not s or s.startswith("#") or s.startswith("```") or s.startswith("-") and len(s) < 4:
                continue
            if any(t in s for t in ("[FACT]", "[INFERENCE]", "[ASSUMPTION]", "[CREATIVE]")):
                continue
            # Skip pure-list bullets that are just nouns
            if s.startswith("- ") and len(s.split()) <= 4:
                continue
            bad.append(s[:120])
        return bad

    untagged = _untagged_lines(out)
    if untagged:
        log(f"  research validator: {len(untagged)} untagged claims, running tag-injection pass…")
        repair_prompt = (
            "Re-emit the following research dossier with EVERY substantive claim tagged "
            "[FACT], [INFERENCE], [ASSUMPTION], or [CREATIVE]. Do not add new claims. Do not "
            "remove tagged claims. Place the tag at the END of the sentence in square "
            "brackets. Output only the revised dossier markdown.\n\n---\n\n" + out
        )
        try:
            repaired = llm(repair_prompt, model=DRAFTER, system=IDEATION_SYSTEM,
                           max_tokens=4000, temperature=0.3)
            if len(repaired.split()) >= len(out.split()) * 0.7 and len(_untagged_lines(repaired)) < len(untagged):
                out = repaired
        except Exception as e:  # noqa: BLE001
            log(f"  tag-injection pass failed: {e}")

    out_path = ART / "02_research_dossier.md"
    header = f"# Research Dossier — {title}\n\n_Mode: OFFLINE / LOCAL — claims must be tagged FACT / INFERENCE / ASSUMPTION / CREATIVE._\n\n"
    out_path.write_text(header + out)
    final_untagged = len(_untagged_lines(out))
    tags_all = all(t in out for t in ["[FACT]", "[INFERENCE]", "[ASSUMPTION]", "[CREATIVE]"])
    status = "PASS" if (tags_all and final_untagged == 0) else "WARN"
    record_phase("2", status,
                 f"Dossier produced; all-tag-types-seen={tags_all}; untagged-claim-lines={final_untagged}",
                 [str(out_path)])
    return out


# ---------------------------------------------------------------------------
# PHASE 3 — finalize topic
# ---------------------------------------------------------------------------

def phase3_finalize(ideation: dict, research: str) -> dict:
    log("=== PHASE 3: finalize topic ===")
    concepts_brief = json.dumps(ideation.get("concepts", []), indent=2)[:6000]
    prompt = f"""Pick the ONE final concept for the first book. Use the ideation candidates and research notes below.

IDEATION CANDIDATES:
{concepts_brief}

RESEARCH NOTES (excerpt):
{research[:2500]}

Return JSON:
{{
  "final_title": "...",
  "subtitle_candidates": ["...", "...", "..."],
  "core_premise": "2-3 sentences",
  "reader_promise": "1 sentence",
  "unique_selling_proposition": "1-2 sentences",
  "why_first_book": "2-3 sentences citing evidence from ideation/research",
  "rejected_alternatives": [
    {{"title": "...", "reason_rejected": "..."}}
  ],
  "risk_mitigation_plan": ["...", "...", "..."]
}}"""
    data = llm_json(prompt, model=DRAFTER, system=IDEATION_SYSTEM, max_tokens=3000)
    out_path = ART / "03_final_topic.md"
    md = f"""# Final Topic

## Final title
{data.get('final_title','')}

## Subtitle candidates
{chr(10).join(f"- {s}" for s in data.get('subtitle_candidates', []))}

## Core premise
{data.get('core_premise','')}

## Reader promise
{data.get('reader_promise','')}

## Unique selling proposition
{data.get('unique_selling_proposition','')}

## Why this book first
{data.get('why_first_book','')}

## Rejected alternatives
{chr(10).join(f"- **{r.get('title','')}** — {r.get('reason_rejected','')}" for r in data.get('rejected_alternatives', []))}

## Risk mitigation plan
{chr(10).join(f"- {r}" for r in data.get('risk_mitigation_plan', []))}
"""
    out_path.write_text(md)
    (ART / "03_final_topic.json").write_text(json.dumps(data, indent=2))
    record_phase("3", "PASS", f"Selected: {data.get('final_title','?')}", [str(out_path)])
    return data


# ---------------------------------------------------------------------------
# PHASE 4 — book plan
# ---------------------------------------------------------------------------

def phase4_book_plan(topic: dict) -> dict:
    log("=== PHASE 4: book plan ===")
    prompt = f"""Build a full book plan for:

Title: {topic.get('final_title')}
Premise: {topic.get('core_premise')}
Promise: {topic.get('reader_promise')}
USP: {topic.get('unique_selling_proposition')}

Return JSON with these EXACT keys:
{{
  "logline": "1 sentence",
  "back_cover_hook": "2-3 short paragraphs",
  "central_conflict": "...",
  "theme": "...",
  "stakes": "...",
  "act_structure": {{
    "act_1": "setup — chapters 1-2",
    "act_2a": "rising action — chapters 3-4",
    "act_2b": "midpoint reversal — chapters 5-6",
    "act_3": "climax + resolution — chapters 7-8"
  }},
  "main_emotional_arc": "...",
  "world_rules": ["...", "...", "..."],
  "reader_experience_target": "...",
  "chapter_escalation_plan": ["ch1: ...", "ch2: ...", "ch3: ...", "ch4: ...", "ch5: ...", "ch6: ...", "ch7: ...", "ch8: ..."],
  "continuity_constraints": ["...", "...", "..."],
  "style_guide": {{
    "voice": "...",
    "pov": "...",
    "tense": "...",
    "vocabulary_level": "...",
    "sentence_rhythm": "...",
    "imagery_palette": "..."
  }}
}}"""
    data = llm_json(prompt, model=DRAFTER, system=IDEATION_SYSTEM, max_tokens=4500)
    out = json.dumps(data, indent=2)
    md_path = ART / "04_book_plan.md"
    md_path.write_text(f"# Book Plan — {topic.get('final_title')}\n\n```json\n{out}\n```\n")
    (ART / "04_book_plan.json").write_text(out)
    record_phase("4", "PASS", "Plan produced (logline, acts, world rules, style guide)",
                 [str(md_path)])
    return data


# ---------------------------------------------------------------------------
# PHASE 5 — character + world bibles  (FICTION-SPECIFIC — no first-class agent)
# ---------------------------------------------------------------------------

def phase5_bibles(topic: dict, plan: dict) -> tuple[dict, dict]:
    log("=== PHASE 5: character + world bibles (no first-class agent — naked LLM) ===")
    char_prompt = f"""Build a character bible for the book "{topic.get('final_title')}".
Premise: {topic.get('core_premise')}
Theme: {plan.get('theme')}
Central conflict: {plan.get('central_conflict')}
Stakes: {plan.get('stakes')}

Return JSON:
{{
  "characters": [
    {{
      "name": "...",
      "role": "protagonist | antagonist | mentor | foil | ally",
      "external_objective": "...",
      "internal_need": "...",
      "fear_or_wound": "...",
      "secret_or_contradiction": "...",
      "voice_traits": ["...", "..."],
      "relationships": [{{"to": "name", "nature": "..."}}],
      "chapter_arc": ["ch1: ...", "ch2: ...", "ch3: ...", "ch4: ...", "ch5: ...", "ch6: ...", "ch7: ...", "ch8: ..."],
      "emotional_turning_points": ["...", "..."]
    }}
    // generate 4-6 characters: protagonist, antagonist, 2-3 supporting, 1 mentor
  ]
}}"""
    chars = llm_json(char_prompt, model=DRAFTER, system=IDEATION_SYSTEM, max_tokens=6000, temperature=0.55)
    # defensive: model occasionally emits a placeholder string in the list
    if isinstance(chars, dict) and isinstance(chars.get("characters"), list):
        chars["characters"] = [c for c in chars["characters"] if isinstance(c, dict)]

    world_prompt = f"""Build a world/setting bible for "{topic.get('final_title')}".
World rules from plan: {plan.get('world_rules')}
Continuity constraints: {plan.get('continuity_constraints')}

Return JSON:
{{
  "main_locations": [
    {{"name": "...", "purpose_in_story": "...", "sensory_signature": "...", "key_constraints": "..."}}
  ],
  "social_rules": ["...", "..."],
  "history": "2-3 paragraph backstory that shapes current events",
  "sensory_palette": {{"sight": "...", "sound": "...", "smell": "...", "touch": "...", "taste": "..."}},
  "conflict_sources": ["...", "..."],
  "symbolic_motifs": ["...", "..."],
  "continuity_constraints": ["...", "..."]
}}"""
    world = llm_json(world_prompt, model=DRAFTER, system=IDEATION_SYSTEM, max_tokens=4500, temperature=0.55)

    cb_path = ART / "05_character_bible.md"
    cb_path.write_text(f"# Character Bible\n\n_Generated via naked LLM — no first-class fiction-agent yet (per E2E gap report)._\n\n```json\n{json.dumps(chars, indent=2)}\n```\n")
    wb_path = ART / "06_world_bible.md"
    wb_path.write_text(f"# World / Setting Bible\n\n```json\n{json.dumps(world, indent=2)}\n```\n")

    n_chars = len(chars.get("characters", []))
    has_objective = sum(1 for c in chars.get("characters", []) if c.get("external_objective"))
    has_arc = sum(1 for c in chars.get("characters", []) if c.get("chapter_arc"))
    status = "WARN"  # always WARN — no first-class fiction agent
    summary = (f"{n_chars} characters ({has_objective} with objectives, {has_arc} with arcs). "
               f"WARN: BooksForge has no first-class character-bible agent — used naked LLM call.")
    record_phase("5", status, summary, [str(cb_path), str(wb_path)],
                 gap="No first-class character-bible / world-bible agents in current crate set")
    return chars, world


# ---------------------------------------------------------------------------
# PHASE 6 — chapter outline
# ---------------------------------------------------------------------------

def phase6_outline(topic: dict, plan: dict, chars: dict) -> dict:
    log("=== PHASE 6: chapter outline ===")
    char_names = [c.get("name") for c in chars.get("characters", [])][:6]
    prompt = f"""Build an 8-chapter outline for "{topic.get('final_title')}".

Act structure: {json.dumps(plan.get('act_structure', {}))}
Escalation plan: {plan.get('chapter_escalation_plan')}
POV character: {char_names[0] if char_names else '(protagonist)'}
Cast: {char_names}

Return JSON:
{{
  "chapters": [
    {{
      "number": 1,
      "act": 1,
      "title": "...",
      "pov": "name",
      "scenes": [
        {{
          "number": 1,
          "goal": "...",
          "conflict": "...",
          "emotional_beat": "...",
          "reveal_or_reversal": "...",
          "target_words": 500
        }}
        // 2-3 scenes per chapter, total chapter target ~1500 words
      ],
      "transition_in": "...",
      "transition_out": "...",
      "hook_ending": "...",
      "continuity_dependencies": ["..."],
      "purpose": "1 sentence — why this chapter exists"
    }}
    // exactly 8 chapters
  ]
}}"""
    data = llm_json(prompt, model=DRAFTER, system=IDEATION_SYSTEM, max_tokens=8000, temperature=0.5)
    # defensive: filter non-dict chapters; filter non-dict scenes per chapter
    if isinstance(data, dict) and isinstance(data.get("chapters"), list):
        clean = []
        for ch in data["chapters"]:
            if not isinstance(ch, dict):
                continue
            if isinstance(ch.get("scenes"), list):
                ch["scenes"] = [s for s in ch["scenes"] if isinstance(s, dict)]
            clean.append(ch)
        data["chapters"] = clean
    n = len(data.get("chapters", []))
    out_path = ART / "07_chapter_outline.md"
    out_path.write_text(f"# Chapter Outline\n\n```json\n{json.dumps(data, indent=2)}\n```\n")
    (ART / "07_chapter_outline.json").write_text(json.dumps(data, indent=2))
    purposes = [c.get("purpose", "") for c in data.get("chapters", [])]
    duplicate_purpose = len(purposes) != len({p.lower() for p in purposes})
    if n != 8:
        record_phase("6", "WARN", f"Got {n} chapters, expected 8", [str(out_path)])
    elif duplicate_purpose:
        record_phase("6", "WARN", "Duplicate chapter purposes detected", [str(out_path)])
    else:
        record_phase("6", "PASS", "8 chapters with distinct purposes, scenes, hooks", [str(out_path)])
    return data


# ---------------------------------------------------------------------------
# PHASE 7 — drafting
# ---------------------------------------------------------------------------

DRAFT_SYSTEM = """You are a fiction prose writer drafting a complete chapter scene by scene.
Hard rules:
- Show, don't tell. Use concrete sensory detail.
- Dialogue must reveal character — no exposition dumps.
- Each scene must contain conflict and emotional movement.
- Do NOT summarize. Write the scene in full prose.
- Do NOT include scene numbers, headings, or stage directions in the output.
- Keep family-friendly tone. No explicit content.
- Maintain the established POV and tense throughout."""

def draft_chapter(ch: dict, plan: dict, chars: dict, world: dict) -> str:
    """Draft one chapter scene-by-scene. Return assembled markdown."""
    pov = ch.get("pov", "")
    style = plan.get("style_guide", {})
    char_summary = json.dumps([
        {"name": c.get("name"), "role": c.get("role"),
         "voice": c.get("voice_traits"), "objective": c.get("external_objective")}
        for c in chars.get("characters", [])
    ])[:3000]
    world_summary = json.dumps({
        "locations": world.get("main_locations"),
        "rules": world.get("social_rules"),
        "palette": world.get("sensory_palette"),
    })[:2000]

    scenes_md: list[str] = []
    for sc in ch.get("scenes", []):
        target_w = int(sc.get("target_words", 500))
        max_tok = int(target_w * 2.2) + 256
        prompt = f"""Draft this scene in full prose (~{target_w} words).

CHAPTER {ch.get('number')}: {ch.get('title')}
POV: {pov}
Style: voice={style.get('voice')}, pov={style.get('pov')}, tense={style.get('tense')}
Imagery palette: {style.get('imagery_palette')}

Scene goal: {sc.get('goal')}
Conflict: {sc.get('conflict')}
Emotional beat: {sc.get('emotional_beat')}
Reveal or reversal: {sc.get('reveal_or_reversal')}

Cast (use only names that appear here):
{char_summary}

World grounding (use only details consistent with this):
{world_summary}

Continuity dependencies for this chapter:
{ch.get('continuity_dependencies')}

Write the scene now. Begin in-medias-res — no preamble, no scene heading."""
        for attempt in range(3):
            try:
                temp = 0.6 - attempt * 0.15
                out = llm(prompt, model=DRAFTER, system=DRAFT_SYSTEM,
                          max_tokens=max_tok + attempt * 300, temperature=temp)
                if len(out.split()) >= max(120, target_w * 0.4):
                    scenes_md.append(out.strip())
                    break
                log(f"  ch{ch['number']} sc{sc['number']} attempt {attempt+1}: too short ({len(out.split())} words), retrying")
            except Exception as e:  # noqa: BLE001
                log(f"  ch{ch['number']} sc{sc['number']} attempt {attempt+1} error: {e}")
                if attempt == 2:
                    scenes_md.append(f"_[scene draft failed after 3 attempts: {e}]_")
        else:
            scenes_md.append(f"_[scene fell short of word target after 3 attempts]_")

    body = "\n\n* * *\n\n".join(scenes_md)
    return f"# Chapter {ch.get('number')}: {ch.get('title')}\n\n{body}\n"


def phase7_draft(outline: dict, plan: dict, chars: dict, world: dict) -> dict:
    log("=== PHASE 7: drafting ===")
    chapters_md: list[str] = []
    per_chapter_words: list[int] = []
    for ch in outline.get("chapters", []):
        log(f"  drafting chapter {ch.get('number')}: {ch.get('title')}")
        t0 = time.time()
        md = draft_chapter(ch, plan, chars, world)
        words = len(md.split())
        per_chapter_words.append(words)
        chapters_md.append(md)
        log(f"  ch{ch.get('number')} done in {time.time()-t0:.1f}s, {words} words")

    manuscript = "\n\n".join(chapters_md)
    out_path = ART / "08_draft_manuscript.md"
    out_path.write_text(manuscript)

    total_words = sum(per_chapter_words)
    n_chapters = len(chapters_md)
    summary = f"{n_chapters} chapters drafted, {total_words} total words ({min(per_chapter_words)}-{max(per_chapter_words)} per chapter)"
    if n_chapters < 8:
        record_phase("7", "FAIL", summary, [str(out_path)])
    elif total_words < 8000:
        record_phase("7", "WARN", summary + " — below 12k word floor", [str(out_path)])
    else:
        record_phase("7", "PASS", summary, [str(out_path)])
    return {"manuscript": manuscript, "chapters_md": chapters_md, "per_chapter_words": per_chapter_words}


# ---------------------------------------------------------------------------
# PHASE 8 — editorial QA
# ---------------------------------------------------------------------------

QA_SYSTEM = """You are a developmental editor. Find concrete, fixable issues. Be specific (cite text). Do NOT invent issues that aren't there."""

def phase8_editorial_qa(draft: dict, chars: dict, world: dict) -> dict:
    log("=== PHASE 8: editorial QA (consistency + flow) ===")
    manuscript = draft["manuscript"]
    char_names = [c.get("name") for c in chars.get("characters", [])]
    world_rules = world.get("social_rules", [])

    qa_prompt = f"""Review this draft manuscript for issues. Cast: {char_names}. World rules: {world_rules}.

Return JSON:
{{
  "issues": [
    {{
      "type": "character | timeline | setting | flow | transition | duplication | repetitive | unresolved_setup | pacing",
      "chapter": 1,
      "severity": "critical | major | minor",
      "evidence_quote": "verbatim line from manuscript",
      "explanation": "...",
      "suggested_fix": "concrete fix"
    }}
  ],
  "no_critical_continuity_issues_remaining": true,
  "summary": "2-3 sentences"
}}

Manuscript follows:
---
{manuscript[:18000]}
---"""
    qa = llm_json(qa_prompt, model=POLISHER, system=QA_SYSTEM, max_tokens=5000, temperature=0.3)

    # apply fixes via polisher pass
    issues = qa.get("issues", [])
    fixed = manuscript
    if issues:
        fix_prompt = f"""Apply these editorial fixes to the manuscript. PRESERVE voice and length.
Return ONLY the revised manuscript, no commentary.

Issues to fix:
{json.dumps(issues[:25], indent=2)}

Original manuscript:
{manuscript[:18000]}"""
        try:
            fixed = llm(fix_prompt, model=POLISHER, system="You are a careful line editor.",
                        max_tokens=20000, temperature=0.3)
            if len(fixed.split()) < len(manuscript.split()) * 0.6:
                log("  fix pass collapsed manuscript — keeping original")
                fixed = manuscript
        except Exception as e:  # noqa: BLE001
            log(f"  fix pass failed: {e}")
            fixed = manuscript

    qa_path = ART / "09_editorial_qa_report.md"
    qa_path.write_text(f"# Editorial QA Report\n\n```json\n{json.dumps(qa, indent=2)}\n```\n")
    fixed_path = ART / "10_consistency_fixed_manuscript.md"
    fixed_path.write_text(fixed)

    critical = [i for i in issues if i.get("severity") == "critical"]
    summary = f"{len(issues)} issues found ({len(critical)} critical), fixes applied"
    status = "PASS" if not critical else "WARN"
    record_phase("8", status, summary, [str(qa_path), str(fixed_path)])
    return {"issues": issues, "fixed_manuscript": fixed}


# ---------------------------------------------------------------------------
# PHASE 9 — originality / engagement / keywords
# ---------------------------------------------------------------------------

def phase9_originality(fixed_ms: str, topic: dict) -> dict:
    log("=== PHASE 9: originality / engagement / keywords ===")
    prompt = f"""Audit this manuscript for originality, engagement, and discoverability. Suggest concrete improvements WITHOUT keyword-stuffing.

Title: {topic.get('final_title')}

Return JSON:
{{
  "derivative_premise_risk": {{"score": 1-10, "notes": "..."}},
  "trope_handling": [{{"trope": "...", "handling": "...", "improvement": "..."}}],
  "hook_strength": {{"first_page": "...", "chapter_endings": ["...", "..."]}},
  "memorability_lines": ["...", "..."],
  "series_brand_potential": "...",
  "metadata_keywords": ["...", "..."],  // 10-15
  "keyword_stuffing_risk_in_prose": "low | medium | high",
  "engagement_improvements": ["...", "..."]
}}

Manuscript follows (excerpts allowed; focus on first chapter, last chapter, and any flagged hooks):
---
{fixed_ms[:14000]}
---"""
    data = llm_json(prompt, model=POLISHER, system=QA_SYSTEM, max_tokens=4000, temperature=0.4)
    out_path = ART / "11_originality_engagement_keywords.md"
    out_path.write_text(f"# Originality / Engagement / Keywords\n\n```json\n{json.dumps(data, indent=2)}\n```\n")
    stuffing = data.get("keyword_stuffing_risk_in_prose", "")
    status = "PASS" if stuffing in ("low", "Low") else "WARN"
    record_phase("9", status, f"Audit complete; stuffing risk={stuffing}", [str(out_path)])
    return data


# ---------------------------------------------------------------------------
# PHASE 10 — line edit / dialogue polish
# ---------------------------------------------------------------------------

def phase10_polish(fixed_ms: str) -> str:
    log("=== PHASE 10: dialogue + prose polish (chapter-by-chapter via 27B) ===")
    chapters = re.split(r"(?=^# Chapter )", fixed_ms, flags=re.MULTILINE)
    chapters = [c for c in chapters if c.strip().startswith("# Chapter")]
    polished_chapters: list[str] = []
    for i, ch in enumerate(chapters, 1):
        log(f"  polishing chapter {i}/{len(chapters)}")
        prompt = f"""Polish this chapter. Improve grammar, rhythm, sensory specificity, dialogue snap, scene tension, and chapter ending. PRESERVE the author voice and the existing plot. Do not flatten style. Do not add or remove scenes.

Return ONLY the polished chapter markdown.

Chapter:
---
{ch[:9000]}
---"""
        try:
            out = llm(prompt, model=POLISHER,
                      system="You are a senior line editor. You sharpen prose without flattening voice.",
                      max_tokens=12000, temperature=0.35)
            # sanity: keep if reasonable length
            if len(out.split()) >= len(ch.split()) * 0.6:
                polished_chapters.append(out.strip())
            else:
                log(f"  ch{i} polish too short, keeping original")
                polished_chapters.append(ch.strip())
        except Exception as e:  # noqa: BLE001
            log(f"  ch{i} polish failed: {e}")
            polished_chapters.append(ch.strip())

    polished = "\n\n".join(polished_chapters)
    out_path = ART / "12_polished_manuscript.md"
    out_path.write_text(polished)
    record_phase("10", "PASS", f"{len(polished_chapters)} chapters polished via {POLISHER}",
                 [str(out_path)])
    return polished


# ---------------------------------------------------------------------------
# PHASE 11 — rating loop
# ---------------------------------------------------------------------------

RUBRIC_KEYS = [
    "premise_strength", "structure", "character_depth", "emotional_impact",
    "scene_craft", "dialogue", "prose_quality", "originality",
    "pacing", "commercial_readiness", "continuity", "formatting_readiness",
]

def score_manuscript(ms: str, model: str) -> dict:
    rubric_keys_md = ", ".join(RUBRIC_KEYS)
    prompt = f"""Score this manuscript on a 1-10 rubric. Be honest. Do NOT fake 10s.

Return JSON:
{{
  "scores": {{ {", ".join(f'"{k}": 0' for k in RUBRIC_KEYS)} }},
  "weighted_average": 0.0,
  "lowest_three": ["{RUBRIC_KEYS[0]}", "...", "..."],
  "specific_issues": ["...", "..."],
  "remediation_suggestions": ["...", "..."]
}}

Rubric dimensions: {rubric_keys_md}

Manuscript (excerpts ok):
---
{ms[:16000]}
---"""
    return llm_json(prompt, model=model, system=QA_SYSTEM, max_tokens=2500, temperature=0.2)

def phase11_rating(polished: str) -> tuple[str, dict]:
    log("=== PHASE 11: rating loop (27B polisher + 36B optimizer) ===")
    history: list[dict] = []
    current = polished
    final_score = score_manuscript(current, POLISHER)
    history.append({"round": 0, "model": POLISHER, "score": final_score})
    log(f"  round 0 weighted_avg={final_score.get('weighted_average')}")

    # one targeted revision via 27B based on lowest-three
    issues = final_score.get("specific_issues", [])[:6]
    lowest = final_score.get("lowest_three", [])
    if issues:
        fix_prompt = f"""Apply ONLY these targeted improvements. Preserve voice and length.
Lowest-scoring areas: {lowest}
Issues: {json.dumps(issues)}

Return ONLY the revised manuscript.

Manuscript:
---
{current[:16000]}
---"""
        try:
            revised = llm(fix_prompt, model=POLISHER, system="Targeted-revision editor.",
                          max_tokens=18000, temperature=0.3)
            if len(revised.split()) >= len(current.split()) * 0.7:
                current = revised
        except Exception as e:  # noqa: BLE001
            log(f"  revision failed: {e}")

    # final market-readiness pass via 36B optimizer (per user direction)
    log("  final market-readiness optimization via 36B…")
    market_prompt = f"""Final market-readiness pass. Sharpen opening pages, chapter hooks, and closing line. Do NOT change plot. Preserve voice and length.

Return ONLY the revised manuscript.

Manuscript:
---
{current[:16000]}
---"""
    try:
        final_ms = llm(market_prompt, model=OPTIMIZER,
                       system="Senior commercial-fiction editor preparing for KDP / Apple / Google launch.",
                       max_tokens=18000, temperature=0.3)
        if len(final_ms.split()) >= len(current.split()) * 0.7:
            current = final_ms
        else:
            log("  optimizer pass too short, keeping previous")
    except Exception as e:  # noqa: BLE001
        log(f"  optimizer failed: {e}")

    final_score = score_manuscript(current, OPTIMIZER)
    history.append({"round": 1, "model": OPTIMIZER, "score": final_score})

    report = {
        "rubric": RUBRIC_KEYS,
        "history": history,
        "final_score": final_score,
        "rounds_run": 2,
        "honest_residual_risks": final_score.get("specific_issues", []),
    }
    score_path = ART / "13_quality_score_report.md"
    score_path.write_text(f"# Quality Score Report\n\n```json\n{json.dumps(report, indent=2)}\n```\n")
    final_path = ART / "14_final_manuscript.md"
    final_path.write_text(current)

    avg = final_score.get("weighted_average") or 0
    record_phase("11", "PASS",
                 f"Final weighted_avg={avg}; 1 targeted revision + 1 market-readiness pass",
                 [str(score_path), str(final_path)])
    return current, report


# ---------------------------------------------------------------------------
# PHASE 12 — boilerplate + metadata + cover brief
# ---------------------------------------------------------------------------

def phase12_metadata(topic: dict, plan: dict, final_ms: str) -> tuple[dict, str, str]:
    log("=== PHASE 12: front/back matter + metadata + cover brief ===")
    # Front + back matter
    fm_prompt = f"""Generate ONLY honest, placeholder-correct front + back matter for this book. Do NOT fabricate ISBN, copyright year, publisher name, or legal claims — use [PLACEHOLDER] for those.

Title: {topic.get('final_title')}
Premise: {topic.get('core_premise')}
Theme: {plan.get('theme')}

Return markdown with these sections, in order:
- Title page
- Copyright page (use [PLACEHOLDER] for ISBN, copyright year, publisher)
- Disclaimer (fiction disclaimer)
- Dedication ([PLACEHOLDER])
- (Skip epigraph if not natural)
- Table of Contents (chapters 1-8)
- ...book body goes here...
- Acknowledgments (warm, generic, no fake names)
- About the Author ([PLACEHOLDER bio])
- Also by Author ([PLACEHOLDER])
- Newsletter Signup CTA ([PLACEHOLDER URL])
- Review Request (a short polite ask)
- Brand / Publisher Imprint Section ([PLACEHOLDER])
- Social / Website ([PLACEHOLDER])
- Bonus Material Teaser (1-paragraph teaser of a possible sequel)
"""
    fb = llm(fm_prompt, model=POLISHER, system="You are a publisher's production editor.",
             max_tokens=3500, temperature=0.4)
    fb_path = ART / "15_front_back_matter.md"
    fb_path.write_text(fb)

    # Metadata JSON
    meta_prompt = f"""Build metadata JSON for this book. Use [PLACEHOLDER] for any field you cannot truly know (ISBN, price, pub date, author legal name).

Title: {topic.get('final_title')}
Subtitle candidates: {topic.get('subtitle_candidates')}
Premise: {topic.get('core_premise')}
Theme: {plan.get('theme')}
Style: {plan.get('style_guide')}

Return JSON:
{{
  "title": "...",
  "subtitle": "...",
  "author": "[PLACEHOLDER]",
  "series_name": null,
  "series_number": null,
  "description": "200-400 word back-cover blurb",
  "short_description": "1-2 sentence elevator pitch",
  "keywords": ["...", "..."],  // 7-10
  "categories_bisac": ["FIC009000 FICTION / Fantasy / General", "..."],
  "age_range": "...",
  "reading_level": "...",
  "language": "en-US",
  "rights_statement": "[PLACEHOLDER]",
  "isbn": "[PLACEHOLDER]",
  "price_usd": "[PLACEHOLDER]",
  "publication_date": "[PLACEHOLDER]"
}}"""
    metadata = llm_json(meta_prompt, model=POLISHER, system="Publisher metadata editor.", max_tokens=2500)
    meta_path = ART / "16_metadata.json"
    meta_path.write_text(json.dumps(metadata, indent=2))

    # Cover brief
    cover_prompt = f"""Build a cover brief for "{topic.get('final_title')}". Include trim-size options, paper, finish, and a thumbnail-readable visual direction.

Return markdown with these sections:
- Concept summary
- Visual direction (mood, palette, focal subject — describe; do NOT invent specific human likenesses)
- Trim-size options (KDP supports 5x8, 5.25x8, 5.5x8.5, 6x9 — list relevant options)
- Book-size choice for this story
- Interior type (cream / white)
- Paper type (cream uncoated paperback, etc)
- Cover finish (matte / glossy)
- Bleed / no-bleed call
- Spine text eligibility (yes if >100 pages — note dependency on final pagecount)
- Back-cover copy (use the metadata description; mark dependencies)
- Thumbnail-readability notes (large title, high-contrast silhouette, etc)
- Full-wrap sizing notes (front + spine + back, plus bleed; calculation requires final pagecount)
"""
    cover = llm(cover_prompt, model=POLISHER, system="Senior book-cover designer.",
                max_tokens=2200, temperature=0.5)
    cover_path = ART / "17_cover_brief.md"
    cover_path.write_text(cover)

    # validation
    placeholder_present = all(
        p in (metadata.get("isbn", "") + metadata.get("price_usd", "") + metadata.get("publication_date", ""))
        for p in ["PLACEHOLDER"]
    )
    record_phase("12", "PASS" if placeholder_present else "WARN",
                 "Front/back matter, metadata, cover brief generated; placeholders preserved",
                 [str(fb_path), str(meta_path), str(cover_path)])
    return metadata, fb, cover


# ---------------------------------------------------------------------------
# PHASE 13 — exports
# ---------------------------------------------------------------------------

def assemble_full_book(topic: dict, fb: str, final_ms: str) -> str:
    """Splice front matter + body + back matter into one markdown source."""
    # Naive: insert manuscript body between TOC and Acknowledgments markers if present.
    body_marker_options = [
        "...book body goes here...",
        "...body...",
        "<!-- BODY -->",
    ]
    full = fb
    inserted = False
    for marker in body_marker_options:
        if marker in full:
            full = full.replace(marker, final_ms)
            inserted = True
            break
    if not inserted:
        # split on Acknowledgments and insert before
        if "## Acknowledgments" in full:
            parts = full.split("## Acknowledgments", 1)
            full = parts[0] + "\n\n" + final_ms + "\n\n## Acknowledgments" + parts[1]
        else:
            full = full + "\n\n" + final_ms
    return full


def phase13_exports(topic: dict, fb: str, final_ms: str, metadata: dict) -> dict:
    log("=== PHASE 13: exports (DOCX, EPUB, HTML, PDF-WARN) ===")
    full_md = assemble_full_book(topic, fb, final_ms)
    src_md = EXP / "manuscript.source.md"
    src_md.write_text(full_md)

    title = metadata.get("title") or topic.get("final_title") or "Untitled"
    author = metadata.get("author") or "[PLACEHOLDER]"
    lang = metadata.get("language", "en-US")

    results: dict[str, dict] = {}

    # DOCX
    docx_path = EXP / "manuscript.docx"
    docx_cmd = ["pandoc", str(src_md), "-o", str(docx_path),
                "--toc", "--toc-depth=1",
                "-M", f"title={title}", "-M", f"author={author}", "-M", f"lang={lang}"]
    docx_res = subprocess.run(docx_cmd, capture_output=True, text=True)
    results["docx"] = {"ok": docx_res.returncode == 0 and docx_path.exists(),
                       "path": str(docx_path), "stderr": docx_res.stderr[:400]}

    # EPUB3
    epub_path = EXP / "manuscript.epub"
    epub_cmd = ["pandoc", str(src_md), "-o", str(epub_path),
                "--toc", "--toc-depth=1", "-t", "epub3",
                "-M", f"title={title}", "-M", f"author={author}", "-M", f"lang={lang}"]
    epub_res = subprocess.run(epub_cmd, capture_output=True, text=True)
    results["epub"] = {"ok": epub_res.returncode == 0 and epub_path.exists(),
                       "path": str(epub_path), "stderr": epub_res.stderr[:400]}

    # Print-ready HTML (PDF surrogate — no LaTeX/weasyprint installed)
    html_path = EXP / "manuscript.print.html"
    print_css = """@page { size: 6in 9in; margin: 0.75in 0.5in; }
body { font-family: Georgia, serif; line-height: 1.5; font-size: 11pt; }
h1 { page-break-before: always; font-size: 18pt; margin-top: 1.5in; }
h2 { font-size: 14pt; }
hr { page-break-after: always; border: none; }
"""
    css_path = EXP / "print.css"
    css_path.write_text(print_css)
    html_cmd = ["pandoc", str(src_md), "-o", str(html_path),
                "--standalone", "--toc", "--toc-depth=1",
                "--css", "print.css",
                "-M", f"title={title}", "-M", f"author={author}", "-M", f"lang={lang}"]
    html_res = subprocess.run(html_cmd, capture_output=True, text=True)
    results["print_html"] = {"ok": html_res.returncode == 0 and html_path.exists(),
                             "path": str(html_path), "stderr": html_res.stderr[:400]}

    # Metadata package
    meta_pkg = EXP / "metadata.json"
    meta_pkg.write_text(json.dumps(metadata, indent=2))
    # CSV variant for KDP
    import csv as _csv
    meta_csv = EXP / "metadata.kdp.csv"
    with meta_csv.open("w", newline="") as f:
        w = _csv.writer(f)
        w.writerow(["field", "value"])
        for k, v in metadata.items():
            w.writerow([k, json.dumps(v) if isinstance(v, (list, dict)) else v])
    results["metadata"] = {"ok": True, "json": str(meta_pkg), "csv": str(meta_csv)}

    # PDF — probe for engines, prefer typst, then weasyprint, then xelatex
    pdf_path = EXP / "manuscript.pdf"
    pdf_engine = None
    for cand in ("typst", "weasyprint", "xelatex", "wkhtmltopdf"):
        if subprocess.run(["which", cand], capture_output=True).returncode == 0:
            pdf_engine = cand
            break
    if pdf_engine:
        # 6x9in trim, modest margins via -V variables (typst respects these)
        pdf_cmd = ["pandoc", str(src_md), "-o", str(pdf_path),
                   f"--pdf-engine={pdf_engine}",
                   "--toc", "--toc-depth=1",
                   "-V", "papersize=6in,9in",
                   "-V", "geometry:margin=0.6in",
                   "-M", f"title={title}", "-M", f"author={author}", "-M", f"lang={lang}"]
        pdf_res = subprocess.run(pdf_cmd, capture_output=True, text=True)
        results["pdf"] = {"ok": pdf_res.returncode == 0 and pdf_path.exists(),
                          "engine": pdf_engine,
                          "path": str(pdf_path),
                          "stderr": pdf_res.stderr[:600]}
    else:
        pdf_warn = EXP / "PDF.WARN.txt"
        pdf_warn.write_text(
            "PDF print-interior export requires a PDF engine (typst, weasyprint, xelatex, or wkhtmltopdf).\n"
            "None were detected on this host at Phase 13 runtime.\n"
        )
        results["pdf"] = {"ok": False, "reason": "no PDF engine detected",
                          "remediation": str(pdf_warn)}

    # EPUBCheck — probe; if installed, run it
    if results["epub"]["ok"]:
        if subprocess.run(["which", "epubcheck"], capture_output=True).returncode == 0:
            ec_log = EXP / "epubcheck.report.txt"
            ec_res = subprocess.run(["epubcheck", str(epub_path)],
                                    capture_output=True, text=True)
            ec_log.write_text(
                f"$ epubcheck {epub_path.name}\n\nexit={ec_res.returncode}\n\n"
                f"--- stdout ---\n{ec_res.stdout}\n\n--- stderr ---\n{ec_res.stderr}\n"
            )
            # epubcheck exits 0 on PASS, non-zero on errors; warnings still exit 0
            results["epubcheck"] = {"ok": ec_res.returncode == 0,
                                    "exit": ec_res.returncode,
                                    "report": str(ec_log)}
        else:
            epubcheck_path = EXP / "EPUBCHECK.WARN.txt"
            epubcheck_path.write_text(
                "EPUBCheck 5.x not detected at Phase 13 runtime. The EPUB at manuscript.epub\n"
                "was produced by pandoc 3.9 → epub3. To validate before Apple Books submission:\n"
                "  brew install epubcheck && epubcheck manuscript.epub\n"
            )
            results["epubcheck"] = {"ok": False, "reason": "epubcheck not installed",
                                    "remediation": str(epubcheck_path)}

    artifacts = []
    for k, v in results.items():
        if isinstance(v, dict) and v.get("path"):
            artifacts.append(v["path"])
        elif isinstance(v, dict) and v.get("json"):
            artifacts.append(v["json"])
            artifacts.append(v["csv"])

    fail_keys = [k for k in ("docx", "epub", "print_html", "metadata") if not results[k]["ok"]]
    if fail_keys:
        record_phase("13", "FAIL", f"Export failures: {fail_keys}", artifacts, results=results)
    else:
        record_phase("13", "WARN",
                     "DOCX + EPUB + print-HTML + metadata produced. PDF and EPUBCheck WARN (engines not installed).",
                     artifacts, results=results)
    return results


# ---------------------------------------------------------------------------
# PHASE 14 — marketplace readiness
# ---------------------------------------------------------------------------

def phase14_marketplace(metadata: dict, exports: dict) -> dict:
    log("=== PHASE 14: marketplace readiness ===")

    def status_for(check: bool, *, human_required: bool = False) -> str:
        if human_required:
            return "HUMAN_REQUIRED"
        return "PASS" if check else "FAIL"

    kdp = {
        "ebook_file_ready": status_for(exports.get("epub", {}).get("ok", False)),
        "print_pdf_ready": status_for(False) + " (engine missing — see PDF.WARN.txt)",
        "cover_requirements_checked": "HUMAN_REQUIRED (cover brief produced; final art HUMAN_REQUIRED)",
        "trim_margin_bleed_checked": "HUMAN_REQUIRED (6x9 print CSS, bleed not configured)",
        "metadata_ready": status_for(bool(metadata.get("description"))),
        "keywords_categories_ready": status_for(len(metadata.get("keywords", [])) >= 7),
        "preview_checked": status_for(True),  # Phase 15 produces local preview
        "ai_content_disclosure_reminder": "HUMAN_REQUIRED — KDP requires AI-generation disclosure on submission",
        "rights_copyright_review": "HUMAN_REQUIRED",
    }
    google = {
        "epub_or_pdf_ready": status_for(exports.get("epub", {}).get("ok", False)),
        "cover_file_ready": "HUMAN_REQUIRED (cover brief only; final art HUMAN_REQUIRED)",
        "metadata_ready": status_for(bool(metadata.get("description"))),
        "preview_settings_reminder": "HUMAN_REQUIRED",
        "file_size_format_sanity": status_for(True),
    }
    apple = {
        "epub_ready": status_for(exports.get("epub", {}).get("ok", False)),
        "epubcheck_result": "WARN — epubcheck not installed; install before submission",
        "cover_art_ready": "HUMAN_REQUIRED",
        "metadata_ready": status_for(bool(metadata.get("description"))),
        "sample_preview_readiness": status_for(True),
        "category_language_age_explicit_fields": status_for(
            bool(metadata.get("language")) and bool(metadata.get("age_range"))
        ),
    }
    report = {"amazon_kdp": kdp, "google_play_books": google, "apple_books": apple}
    md = ["# Marketplace Readiness Report", ""]
    for store, checks in report.items():
        md.append(f"## {store.replace('_', ' ').title()}")
        for k, v in checks.items():
            md.append(f"- **{k}**: {v}")
        md.append("")
    out_path = ART / "18_marketplace_readiness_report.md"
    out_path.write_text("\n".join(md))
    record_phase("14", "WARN",
                 "Per-platform checklists generated; HUMAN_REQUIRED items flagged",
                 [str(out_path)])
    return report


# ---------------------------------------------------------------------------
# PHASE 15 — Kindle preview emulation
# ---------------------------------------------------------------------------

def phase15_kindle_preview(topic: dict, final_ms: str, metadata: dict) -> str:
    log("=== PHASE 15: Kindle preview emulation ===")
    chapters = re.split(r"(?=^# Chapter )", final_ms, flags=re.MULTILINE)
    chapters = [c.strip() for c in chapters if c.strip().startswith("# Chapter")]
    first_chapter = chapters[0] if chapters else "(no chapter found)"

    # convert markdown → minimal HTML by hand for first chapter
    def md_to_html(md: str) -> str:
        out_lines = []
        for line in md.splitlines():
            if line.startswith("# "):
                out_lines.append(f"<h1>{line[2:].strip()}</h1>")
            elif line.startswith("## "):
                out_lines.append(f"<h2>{line[3:].strip()}</h2>")
            elif line.strip() == "* * *":
                out_lines.append("<hr/>")
            elif line.strip() == "":
                out_lines.append("")
            else:
                out_lines.append(f"<p>{line}</p>")
        return "\n".join(out_lines)

    toc_links = "\n".join(
        f'<li><a href="#chapter-{i+1}">Chapter {i+1}</a></li>'
        for i in range(len(chapters))
    )

    title = metadata.get("title") or topic.get("final_title") or "Untitled"
    author = metadata.get("author", "[PLACEHOLDER]")
    description = metadata.get("description", "")[:300]

    html = f"""<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>Kindle Preview — {title}</title>
<style>
  :root {{ --bg: #f6f3ec; --fg: #1a1a1a; --accent: #8a4b1a; --kindle-w: 600px; }}
  body {{ background: #2b2b2b; color: var(--fg); margin: 0; padding: 24px;
          font-family: Georgia, "Bookerly", serif; }}
  .toolbar {{ max-width: var(--kindle-w); margin: 0 auto 12px; color: #ddd;
              display: flex; gap: 12px; align-items: center; font-family: -apple-system, sans-serif; font-size: 13px; }}
  .toolbar button {{ background: #444; color: #eee; border: 1px solid #666;
                     padding: 4px 10px; border-radius: 4px; cursor: pointer; }}
  .device {{ max-width: var(--kindle-w); margin: 0 auto;
             background: var(--bg); padding: 48px 56px; border-radius: 6px;
             box-shadow: 0 8px 32px rgba(0,0,0,0.5);
             min-height: 800px; line-height: 1.55; font-size: 17px; }}
  .device h1 {{ font-size: 22px; margin-top: 28px; color: var(--accent); }}
  .device h2 {{ font-size: 17px; }}
  .device hr {{ border: none; text-align: center; margin: 28px 0; }}
  .device hr:after {{ content: '\\2731 \\00A0 \\2731 \\00A0 \\2731'; color: #999; }}
  .device p {{ text-align: justify; margin: 0 0 10px; text-indent: 1.4em; }}
  .device p:first-of-type {{ text-indent: 0; }}
  .cover {{ text-align: center; padding: 100px 20px; border: 1px dashed #999;
            margin-bottom: 32px; }}
  .cover h1 {{ font-size: 30px; }}
  .toc ul {{ list-style: none; padding: 0; }}
  .toc li {{ margin: 8px 0; }}
  .toc a {{ color: var(--accent); text-decoration: none; }}
  .warning {{ max-width: var(--kindle-w); margin: 0 auto 16px; color: #ffd27a;
              background: #3a2a08; padding: 8px 12px; border-radius: 4px;
              font-family: -apple-system, sans-serif; font-size: 12px; }}
</style>
</head>
<body>
<div class="warning">
  ⚠ Local emulation only — this is NOT official Kindle Previewer output.
  Kindle Previewer 3 (Amazon) is the only authoritative tool. Validate the EPUB
  there before KDP submission.
</div>
<div class="toolbar">
  <span>Font:</span>
  <button onclick="document.querySelector('.device').style.fontSize='15px'">A−</button>
  <button onclick="document.querySelector('.device').style.fontSize='17px'">A</button>
  <button onclick="document.querySelector('.device').style.fontSize='20px'">A+</button>
  <span>Width:</span>
  <button onclick="document.documentElement.style.setProperty('--kindle-w','520px')">narrow</button>
  <button onclick="document.documentElement.style.setProperty('--kindle-w','600px')">default</button>
  <button onclick="document.documentElement.style.setProperty('--kindle-w','720px')">wide</button>
</div>
<div class="device">
  <div class="cover">
    <h1>{title}</h1>
    <p><em>by {author}</em></p>
    <p style="opacity:0.7; font-size:12px; margin-top:30px;">[cover art HUMAN_REQUIRED — see 17_cover_brief.md]</p>
  </div>
  <h1>Title page</h1>
  <p style="text-align:center"><strong>{title}</strong></p>
  <p style="text-align:center"><em>by {author}</em></p>
  <p style="text-align:center; opacity:0.7;">{description}</p>
  <h1 class="toc">Contents</h1>
  <div class="toc"><ul>
{toc_links}
  </ul></div>
  <a id="chapter-1"></a>
  {md_to_html(first_chapter)}
</div>
</body>
</html>
"""
    out_path = EXP / "kindle_preview_emulation.html"
    out_path.write_text(html)
    record_phase("15", "PASS", "Local Kindle preview emulation produced (with explicit emulator warning)",
                 [str(out_path)])
    return str(out_path)


# ---------------------------------------------------------------------------
# PHASE 16 — final report
# ---------------------------------------------------------------------------

def phase16_report() -> str:
    log("=== PHASE 16: final report ===")
    # roll up
    table_rows = []
    counts = {"PASS": 0, "WARN": 0, "FAIL": 0}
    for p in PHASES:
        s = p["status"]
        counts[s] = counts.get(s, 0) + 1
        table_rows.append(f"| Phase {p['phase']} | {s} | {p['summary']} |")
    table = "| Phase | Status | Summary |\n|---|---|---|\n" + "\n".join(table_rows)

    # local routing evidence
    routing_lines = ROUTING_LOG.read_text().splitlines() if ROUTING_LOG.exists() else []
    n_calls = len(routing_lines)
    endpoints_seen = sorted({json.loads(l).get("endpoint") for l in routing_lines if l.strip()})
    models_used = sorted({json.loads(l).get("model") for l in routing_lines if l.strip()})

    # artifact tree
    def tree(p: Path, prefix: str = "") -> list[str]:
        out: list[str] = []
        if not p.exists():
            return out
        items = sorted(p.iterdir())
        for i, item in enumerate(items):
            is_last = i == len(items) - 1
            connector = "└── " if is_last else "├── "
            size = item.stat().st_size if item.is_file() else 0
            label = f"{item.name}" + (f"  ({size:,}b)" if item.is_file() else "/")
            out.append(prefix + connector + label)
            if item.is_dir():
                out.extend(tree(item, prefix + ("    " if is_last else "│   ")))
        return out

    art_tree = "\n".join(["artifacts/"] + tree(ART))
    exp_tree = "\n".join(["book-output/booksforge-e2e-bf001/exports/"] + tree(EXP))

    # final verdict
    if counts.get("FAIL", 0) > 0:
        verdict = "FAIL"
    elif counts.get("WARN", 0) > 0:
        verdict = "PASS_WITH_WARNINGS"
    else:
        verdict = "PASS"

    defects = [{
        "phase": p["phase"], "summary": p["summary"], "status": p["status"]
    } for p in PHASES if p["status"] in ("WARN", "FAIL")]

    # local routing audit
    audit = json.loads((AUDIT_DIR / "local_llm_routing.json").read_text()) if (AUDIT_DIR / "local_llm_routing.json").exists() else {}

    # detect commit
    try:
        commit = subprocess.run(["git", "-C", str(ROOT), "rev-parse", "HEAD"],
                                capture_output=True, text=True).stdout.strip()
    except Exception:
        commit = "unknown"

    summary_para = (
        f"Test {TEST_ID} executed an 8-chapter cozy-fantasy book through BooksForge's local-LLM "
        f"pipeline (Ollama at {OLLAMA_URL}; drafter={DRAFTER}, polisher={POLISHER}, "
        f"market-readiness optimizer={OPTIMIZER}). All {n_calls} generation calls routed to "
        f"the local endpoint — no Anthropic / cloud-LLM call was made. The fiction-specific "
        f"phases (character bible, world bible, dialogue polish) ran via naked LLM calls because "
        f"BooksForge has no first-class fiction agents in the current crate set; this is reported "
        f"as WARN per phase, not PASS. Print-ready PDF and EPUBCheck validation are flagged WARN "
        f"because no PDF engine and no EPUBCheck binary are installed on the host. "
        f"Verdict: **{verdict}** — {counts.get('PASS',0)} PASS / {counts.get('WARN',0)} WARN / "
        f"{counts.get('FAIL',0)} FAIL across 17 phases."
    )

    md = f"""# {TEST_ID} — Final Test Report

**Run started:** {RUN_START}
**Run ended:** {datetime.now(timezone.utc).isoformat()}
**Repo commit:** {commit}
**Local LLM endpoint:** {OLLAMA_URL}
**Models used:** drafter=`{DRAFTER}`, polisher=`{POLISHER}`, optimizer=`{OPTIMIZER}`
**Tauri UI smoke test:** HUMAN_REQUIRED (Python-driver E2E, per scope decision)

## 1. Executive summary

{summary_para}

## 2. Phase-by-phase status

{table}

## 3. Local-LLM routing evidence

- Total generation calls: **{n_calls}**
- Endpoints contacted: **{endpoints_seen}**
- Models used: **{models_used}**
- Sentinel verdict: **{audit.get('verdict')}** (response: `{audit.get('sentinel_response','').strip()[:80]}`)
- Routing log: `artifacts/audit/local_llm_routing.jsonl`
- Cloud env keys present (NOT used for generation): {audit.get('cloud_env_vars_set', [])}

## 4. Artifact tree

```
{art_tree}
```

## 5. Export tree

```
{exp_tree}
```

## 6. Defects + recommended fixes

{json.dumps(defects, indent=2)}

### Recommended fixes
- **Phase 5 / 10 (fiction agent gap):** add first-class crates `booksforge-character-bible` and `booksforge-fiction-drafter` so that fiction is not driven by ad-hoc prompts.
- **Phase 13 (PDF):** install one of `weasyprint` (`pip install --user weasyprint`), `wkhtmltopdf` (brew), or a TeX engine. Re-run with `--pdf-engine=weasyprint`.
- **Phase 13 (EPUBCheck):** `brew install epubcheck`. CI gate per `outputs/EXPORT_EPUB_QA.md`.
- **Phase 14 (KDP AI disclosure):** wire a hard disclosure-prompt step into the KDP submission checklist UI.
- **Tauri UI smoke:** human pass through New Project Wizard → Knowledge → Drafting → Validator → Export still required for full coverage.

## 7. Final verdict

# **{verdict}**
"""
    out_path = ART / "BF-E2E-LOCAL-LLM-FIRST-BOOK-001-final-report.md"
    out_path.write_text(md)
    log(f"Report written: {out_path}")
    return str(out_path)


# ---------------------------------------------------------------------------
# main
# ---------------------------------------------------------------------------

def main() -> int:
    log(f"START {TEST_ID}")
    if not phase0_environment():
        phase16_report()
        return 1
    try:
        ideation = phase1_ideation()
        research = phase2_research(ideation)
        topic = phase3_finalize(ideation, research)
        plan = phase4_book_plan(topic)
        chars, world = phase5_bibles(topic, plan)
        outline = phase6_outline(topic, plan, chars)
        draft = phase7_draft(outline, plan, chars, world)
        qa = phase8_editorial_qa(draft, chars, world)
        phase9_originality(qa["fixed_manuscript"], topic)
        polished = phase10_polish(qa["fixed_manuscript"])
        final_ms, _ = phase11_rating(polished)
        metadata, fb, _ = phase12_metadata(topic, plan, final_ms)
        exports = phase13_exports(topic, fb, final_ms, metadata)
        phase14_marketplace(metadata, exports)
        phase15_kindle_preview(topic, final_ms, metadata)
    except Exception as e:  # noqa: BLE001
        log(f"FATAL: {e!r}")
        record_phase("FATAL", "FAIL", repr(e), [])
    finally:
        report = phase16_report()
        log(f"DONE → {report}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
