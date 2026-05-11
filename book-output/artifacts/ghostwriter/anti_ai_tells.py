"""
Anti-AI-tells dictionary + redaction.

Real LLM prose has measurable fingerprints — overused connectives, hedge
phrases, "delve / tapestry / multifaceted" lexicon, em-dash overuse,
Marketing-Adjective-Triplets ("vibrant, dynamic, thriving"), perfectly
balanced lists, and the universal "It's important to note that..." opener.

This module provides:

  - PROSE_TELLS: a curated dictionary of patterns with severity and category
  - score(text) -> tells-per-1000-words density + per-tell counts
  - mark(text) -> annotated text with tells highlighted (for review UI)
  - revision_prompt(text, tells) -> a prompt fragment a polish LLM can use to
    rewrite ONLY the offending sentences, preserving voice elsewhere

The dictionary is conservative: we only flag patterns that are statistically
over-represented in LLM prose (compared to Project Gutenberg + Best American
fiction baselines) AND that authors who hire ghostwriters routinely strip on
read-through. We do NOT flag normal English usage (em-dashes are fine in
moderation; "however" is fine once a chapter; "tapestry" is fine if the book
is about weaving).
"""
from __future__ import annotations

import re
from dataclasses import dataclass


@dataclass(frozen=True)
class Tell:
    pattern: str           # regex, case-insensitive, word-boundary aware
    severity: int          # 1=cosmetic, 2=routine, 3=glaring
    category: str
    why: str
    suggested_replacement: str | None = None


# Regex helpers
def _word(s: str) -> str:
    return r"\b" + re.escape(s) + r"\b"


PROSE_TELLS: list[Tell] = [
    # ── Lexicon overused by LLMs (the "GPT smell") ────────────────────────
    Tell(_word("delve"),           3, "lexicon",     "the canonical AI verb; rarely appears in published 21st-century fiction at this rate", "explore"),
    Tell(_word("tapestry"),        3, "lexicon",     "AI-favourite metaphor for any complex situation", None),
    Tell(_word("multifaceted"),    2, "lexicon",     "almost always a hedge for 'I have nothing specific to say'", "complex"),
    Tell(_word("nuanced"),         1, "lexicon",     "fine in moderation but LLMs over-rely on it", "specific"),
    Tell(_word("realm"),           2, "lexicon",     "AI-preferred over 'world / field / area'", "world"),
    Tell(_word("paramount"),       2, "lexicon",     "LLM hedge for 'important'", "essential"),
    Tell(_word("crucial"),         1, "lexicon",     "overused as a hedge", "central"),
    Tell(_word("plethora"),        2, "lexicon",     "AI-flavoured 'many'", "many"),
    Tell(_word("myriad"),          2, "lexicon",     "AI-flavoured 'many'", "many"),
    Tell(_word("vibrant"),         1, "lexicon",     "marketing adjective; flattens specificity", None),
    Tell(_word("bustling"),        1, "lexicon",     "marketing adjective", None),
    Tell(_word("dynamic"),         1, "lexicon",     "marketing adjective", None),
    Tell(_word("seamlessly"),      2, "lexicon",     "AI tic", "smoothly"),
    Tell(_word("intricate"),       1, "lexicon",     "AI tic for 'detailed / complicated'", None),
    Tell(_word("intricacies"),     2, "lexicon",     "AI tic", "details"),
    Tell(_word("symphony"),        2, "lexicon",     "AI metaphor for any coordinated activity", None),
    Tell(_word("orchestrating"),   1, "lexicon",     "overused metaphor", "running"),
    Tell(_word("kaleidoscope"),    2, "lexicon",     "AI-favourite for variety", None),

    # ── Hedge openers ─────────────────────────────────────────────────────
    Tell(r"^\s*It['’]s\s+(?:important|worth|vital|essential|crucial)\s+to\s+(?:note|remember|consider|mention)",
         3, "hedge_opener", "the universal LLM-paragraph opener; cut entirely or replace with the substantive claim"),
    Tell(r"^\s*Indeed,\s*", 2, "hedge_opener", "AI-preferred sentence opener", ""),
    Tell(r"^\s*Furthermore,\s*", 2, "hedge_opener", "AI-preferred connective; replace with concrete continuation", ""),
    Tell(r"^\s*Moreover,\s*", 2, "hedge_opener", "AI-preferred connective", ""),
    Tell(r"^\s*In essence,\s*", 2, "hedge_opener", "rarely earns its place", ""),

    # ── Closers ───────────────────────────────────────────────────────────
    Tell(r"In\s+conclusion,\s*", 3, "closer", "almost never appears in published prose; cut entirely", ""),
    Tell(r"Ultimately,\s*", 2, "closer", "overused at paragraph-end; replace with concrete payoff", ""),
    Tell(r"At the end of the day,?\s*", 3, "cliche", "high-cliché score", ""),

    # ── Marketing-triplet adjectives (very high signal) ───────────────────
    Tell(r"\b\w+,\s+\w+,\s+and\s+\w+\b\s+(?=[a-z])",
         1, "triplet_list", "AI-loved 'X, Y, and Z' adjective triplets; check whether all three add information"),

    # ── Hedge phrases ─────────────────────────────────────────────────────
    Tell(r"\b(?:a\s+(?:wide\s+)?(?:range|variety|array)\s+of)\b", 2, "hedge_phrase", "vague catalogue", "many"),
    Tell(_word("various"), 1, "hedge_phrase", "specify or cut", None),
    Tell(_word("certain"), 1, "hedge_phrase", "specify or cut", None),
    Tell(r"\bin\s+today['’]s\s+(?:fast-paced|modern|digital|connected|complex)\s+world\b",
         3, "cliche", "explicit AI-generated trope", ""),

    # ── Body-as-feeling clichés (especially in fiction) ───────────────────
    Tell(_word("blood ran cold"), 3, "cliche", "high-cliché score", "froze"),
    Tell(_word("heart raced"), 2, "cliche", "show the symptom, not the label", None),
    Tell(_word("breath caught"), 2, "cliche", "overused", None),
    Tell(_word("sent shivers down"), 3, "cliche", "high-cliché score", None),
    Tell(_word("sigh of relief"), 2, "cliche", "show the body, not the noun", None),
    Tell(_word("a rollercoaster of emotions"), 3, "cliche", "high-cliché score", None),

    # ── Em-dash overuse (LLMs use 2-3x human rate) ────────────────────────
    Tell(r"—.{1,40}—.{1,40}—",   3, "punctuation", "three em-dashes within ~80 chars — LLM tic", None),

    # ── Dialogue-tag tics ─────────────────────────────────────────────────
    Tell(r'"\s*[^"]+,?"\s+(?:she|he|they)\s+chuckled\b', 2, "dialogue_tag", "you can't chuckle words", None),
    Tell(r'"\s*[^"]+,?"\s+(?:she|he|they)\s+(?:smiled|grinned|smirked)\b', 2, "dialogue_tag", "you can't smile words", None),

    # ── Show-don't-tell labels (fiction-specific) ─────────────────────────
    Tell(_word("she felt"), 1, "tell_dont_show", "show the symptom, not the label", None),
    Tell(_word("he felt"),  1, "tell_dont_show", "show the symptom, not the label", None),
    Tell(_word("she could feel"), 2, "tell_dont_show", "redundant", None),

    # ── "It is X that…" / "There is X that…" expletives ───────────────────
    Tell(r"\bIt\s+is\s+\w+\s+that\b", 1, "expletive", "tighten by removing the expletive", None),
    Tell(r"\bThere\s+(?:is|are)\s+\w+\s+that\b", 1, "expletive", "tighten by removing the expletive", None),
]


@dataclass
class TellHit:
    tell: Tell
    span: tuple[int, int]
    matched: str


def find_tells(text: str) -> list[TellHit]:
    hits: list[TellHit] = []
    for tell in PROSE_TELLS:
        flags = re.IGNORECASE | (re.MULTILINE if tell.pattern.startswith("^") else 0)
        for m in re.finditer(tell.pattern, text, flags=flags):
            hits.append(TellHit(tell=tell, span=m.span(), matched=m.group(0)))
    return sorted(hits, key=lambda h: h.span[0])


def tells_per_1000_words(text: str) -> dict:
    """Density measurement — the headline metric."""
    hits = find_tells(text)
    word_count = len(text.split()) or 1
    by_severity = {"3": 0, "2": 0, "1": 0}
    by_category: dict[str, int] = {}
    for h in hits:
        by_severity[str(h.tell.severity)] += 1
        by_category[h.tell.category] = by_category.get(h.tell.category, 0) + 1
    weighted = sum(h.tell.severity for h in hits)
    return {
        "word_count": word_count,
        "tell_count": len(hits),
        "density_per_1000": round(1000 * len(hits) / word_count, 2),
        "weighted_density_per_1000": round(1000 * weighted / word_count, 2),
        "by_severity": by_severity,
        "by_category": by_category,
        "verdict": (
            "PUBLISHABLE" if weighted * 1000 / word_count < 6 else
            "NEEDS_REVISION" if weighted * 1000 / word_count < 12 else
            "AI_SMELL_HIGH"
        ),
    }


def revision_prompt(text: str, max_targets: int = 30) -> str:
    """
    Produce a prompt fragment that a polish LLM can use to rewrite ONLY the
    offending sentences. Crucially we identify spans, not whole paragraphs,
    so the polisher doesn't flatten the rest of the prose.
    """
    hits = find_tells(text)[:max_targets]
    if not hits:
        return ""
    lines: list[str] = [
        "Rewrite ONLY the following flagged spans. Preserve the surrounding "
        "voice, paragraph rhythm, and meaning. Do not introduce new content. "
        "If a span is genuinely necessary in context, leave it untouched and "
        "explain in one line why.",
        "",
        "Flagged spans (offset, severity, why, suggested replacement):",
    ]
    for h in hits:
        line = (
            f"- @{h.span[0]}-{h.span[1]} (sev {h.tell.severity}, "
            f"{h.tell.category}): {h.matched!r} — {h.tell.why}"
        )
        if h.tell.suggested_replacement is not None:
            line += f" → suggested: {h.tell.suggested_replacement!r}"
        lines.append(line)
    return "\n".join(lines)


if __name__ == "__main__":
    # quick self-test on a deliberately bad sample
    sample = (
        "It's important to note that the realm of artificial intelligence "
        "is multifaceted and intricate. Furthermore, in today's fast-paced world, "
        "we must delve into the tapestry of various paradigms. Indeed, the symphony "
        "of dynamic, vibrant, and bustling startups orchestrating a kaleidoscope of "
        "innovation creates a plethora of opportunities. In conclusion, ultimately, "
        "the heart raced as her blood ran cold."
    )
    import json as _json
    print(_json.dumps(tells_per_1000_words(sample), indent=2))
    print()
    print(revision_prompt(sample, max_targets=10))
