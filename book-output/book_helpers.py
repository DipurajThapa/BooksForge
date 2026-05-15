#!/usr/bin/env python3
"""
book_helpers.py — shared constants + Ollama chat helper used by the
canonical-flow tooling (book_review.py, outline_to_scenes.py,
final_review.py).

This module is intentionally tiny. The heavy lifting (template render,
JSON repair, pm_doc → markdown) lives in `booksforge_ollama_driver.py`,
which is the canonical home for low-level driver helpers; this file
holds only the project-level constants the surrounding scripts need.
"""

from __future__ import annotations

import time
from typing import Any


# Style book applied to all polish + export passes. Matches the locked
# Rust StyleBook defaults so what the validator scores matches what the
# manuscript was drafted against.
STYLE_BOOK: dict[str, Any] = {
    "em_dash": "em",
    "quote_style": "smart",
    "oxford_comma": True,
    "locale": "en-US",
}

# Default POV for the "My Confused Life" validation project. Override
# per-project if you point this tooling at a different brief.
PROJECT_POV: str = (
    "first-person past tense; protagonist Arjun is the narrator throughout"
)

# Creative profile — passed straight through to the chapter-drafter
# template. Keeps every scene anchored in the same texture: contemporary
# urban India, introspective register, devotional content handled with
# restraint. Matches the brief premise that the canonical Rust pipeline
# also receives.
CREATIVE_PROFILE: str = """\
Voice: introspective first-person past tense, warm without being
sentimental. Long thoughts braided with short observations.
Devotional content (Radha-Krishna iconography, kirtan, the names of
deities) is rendered with respect — used as Arjun encounters it, not
explained for the reader. No glossing. No "as a Hindu reader would
know" parentheticals. The temple sequence in chapter 3 should land
the way it lands in real life: half-skeptical, faintly self-conscious,
and surprised by feeling.

Forbidden tropes: chosen-one revelation; saintly guru who solves
Arjun's life with a single sentence; romance-as-rescue; AI prose tells
(over-explained metaphors, three-adjective stacks, "the truth was…"
sentence openers).

Setting: contemporary urban India (Mumbai for the corporate-life
chapters, a smaller town in the hinterland for the collapse and
recovery arc). Specific enough that a Mumbai reader recognises it;
not a travelogue.
"""


def chat_no_thinking(
    model: str,
    system: str,
    user: str,
    *,
    temperature: float,
    max_tokens: int,
    json_mode: bool = False,
    timeout: int = 1200,
) -> tuple[str, dict]:
    """Direct chat call to a local Ollama server with `think: false`.

    Matches the Rust runtime's request shape for non-thinking agents.
    Wrapped in a 4-attempt connection-retry loop because Ollama
    intermittently drops sockets during model swaps on memory-tight
    machines (the `requests.exceptions.ConnectionError` /
    `RemoteDisconnected` failure mode).

    Sampling options:
      - `repeat_penalty=1.25` — breaks qwen3.5:9b out of self-
        referential "explainer" loops where it writes meta-commentary
        about its own drafting process.
      - `top_p=0.9` — tighter sampling reduces drift into meta-language.
      - `stop` sequences — hard-kill on detected meta-tokens before
        the model nests them into pm_doc text nodes.

    These do not touch the locked Rust templates or the
    `booksforge-ollama` crate — they are driver-level options sent
    with each chat request.
    """
    import requests
    from requests.exceptions import ConnectionError as ReqConnectionError, ReadTimeout

    payload = {
        "model": model,
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": user},
        ],
        "stream": False,
        "think": False,
        "options": {
            "temperature": temperature,
            "num_predict": max_tokens,
            "repeat_penalty": 1.25,
            "top_p": 0.9,
            "stop": [
                "Word count managed",
                "ensuring the prose",
                "the prose remains",
                "internal monologue of the narrator",
                "narrative arc",
                "Note: ",
                "(Note:",
            ],
        },
    }
    if json_mode:
        payload["format"] = "json"

    last_err: Exception | None = None
    for attempt in range(1, 5):
        try:
            t0 = time.time()
            r = requests.post(
                "http://127.0.0.1:11434/api/chat",
                json=payload,
                timeout=timeout,
            )
            elapsed = time.time() - t0
            r.raise_for_status()
            data = r.json()
            content = data.get("message", {}).get("content", "")
            return content, {
                "elapsed_s": round(elapsed, 2),
                "eval_count": data.get("eval_count"),
                "prompt_eval_count": data.get("prompt_eval_count"),
                "model": model,
                "attempt": attempt,
            }
        except (ReqConnectionError, ReadTimeout) as e:
            last_err = e
            print(
                f"        [warn] ollama transport failed "
                f"(attempt {attempt}/4): {type(e).__name__}; sleeping 8s and retrying...",
                flush=True,
            )
            time.sleep(8)

    raise RuntimeError(f"ollama transport failed after 4 attempts: {last_err}")
