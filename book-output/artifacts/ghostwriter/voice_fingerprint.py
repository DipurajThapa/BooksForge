"""
Voice fingerprinting from comp samples.

Ghostwriters need their drafts to sound like the client's voice — or like
the comp titles the client paid them to imitate.  This module measures the
fingerprint of a sample (3-5 short paragraphs is enough) and emits:

  1. A numeric profile (sentence-length distribution, word-tier mix,
     dialogue ratio, etc.)
  2. A `voice_constraints` block that can be injected into the drafter
     prompt, so the drafter actually writes within those numeric targets.

The profile is deliberately measurable, not vibes — that's the difference
between "make it sound literary" (LLMs ignore that) and "median sentence
length 11 words, 22% short sentences under 6 words, 18% dialogue lines"
(LLMs respect explicit numeric targets surprisingly well).

We also compute a `stylometric_distance` between two samples — useful for
"how close did this draft come to the comp" scoring.
"""
from __future__ import annotations

import re
from dataclasses import dataclass, asdict
from collections import Counter

# Common stop words for word-tier analysis.
COMMON_WORDS: set[str] = set(
    "the be to of and a in that have i it for not on with he as you do at this but his by from "
    "they we say her she or an will my one all would there their what so up out if about who get "
    "which go me when make can like time no just him know take people into year your good some "
    "could them see other than then now look only come its over think also back after use two how "
    "our work first well way even new want because any these give day most us is are was were been "
    "had has been being more very much such only own same so than too only into".split()
)


def _split_sentences(text: str) -> list[str]:
    # crude but adequate; preserves dialogue tags
    sents = re.split(r"(?<=[\.\!\?])\s+(?=[A-Z\"'“])", text.strip())
    return [s.strip() for s in sents if s.strip()]


def _is_dialogue(sent: str) -> bool:
    return bool(re.search(r'["“]', sent))


def _word_tokens(text: str) -> list[str]:
    return re.findall(r"[A-Za-z][A-Za-z'\-]+", text.lower())


def _quartile(xs: list[float], q: float) -> float:
    if not xs:
        return 0.0
    s = sorted(xs)
    pos = (len(s) - 1) * q
    lo = int(pos)
    hi = min(lo + 1, len(s) - 1)
    frac = pos - lo
    return s[lo] * (1 - frac) + s[hi] * frac


@dataclass
class VoiceProfile:
    word_count: int
    sentence_count: int
    paragraph_count: int
    median_sentence_length: float
    p25_sentence_length: float
    p75_sentence_length: float
    pct_short_sentences: float           # < 8 words
    pct_long_sentences: float            # > 25 words
    dialogue_ratio: float                # share of sentences containing quotation marks
    avg_paragraph_length_sentences: float
    type_token_ratio: float              # vocab richness 0-1
    rare_word_ratio: float               # share of words NOT in COMMON_WORDS
    em_dash_per_1000: float
    semicolon_per_1000: float
    parenthetical_per_1000: float
    avg_word_length: float
    pct_monosyllabic_words: float        # rough cadence proxy

    def constraints_block(self, label: str = "comp") -> str:
        """Render as a constraint block the drafter can read."""
        lines = [
            f"Voice constraints from {label}:",
            f"- Median sentence length: {round(self.median_sentence_length)} words "
            f"(IQR {round(self.p25_sentence_length)}–{round(self.p75_sentence_length)})",
            f"- Short-sentence share (<8 words): {round(self.pct_short_sentences*100)}%",
            f"- Long-sentence share (>25 words): {round(self.pct_long_sentences*100)}%",
            f"- Dialogue line share: {round(self.dialogue_ratio*100)}%",
            f"- Average paragraph length: {round(self.avg_paragraph_length_sentences, 1)} sentences",
            f"- Vocabulary richness (type-token ratio): {round(self.type_token_ratio, 2)}",
            f"- Rare-word share (non-stopword): {round(self.rare_word_ratio*100)}%",
            f"- Em-dashes per 1000 words: {round(self.em_dash_per_1000, 1)} (do NOT exceed)",
            f"- Semicolons per 1000 words: {round(self.semicolon_per_1000, 1)}",
            f"- Average word length: {round(self.avg_word_length, 2)} characters",
            f"- Monosyllabic-word share: {round(self.pct_monosyllabic_words*100)}% "
            f"(higher = punchier cadence)",
        ]
        return "\n".join(lines)


def fingerprint(text: str) -> VoiceProfile:
    sents = _split_sentences(text)
    paras = [p for p in re.split(r"\n\s*\n", text.strip()) if p.strip()]
    words = _word_tokens(text)
    sent_lens = [len(_word_tokens(s)) for s in sents]
    if not sent_lens or not words:
        return VoiceProfile(
            word_count=len(words), sentence_count=len(sents), paragraph_count=len(paras),
            median_sentence_length=0, p25_sentence_length=0, p75_sentence_length=0,
            pct_short_sentences=0, pct_long_sentences=0, dialogue_ratio=0,
            avg_paragraph_length_sentences=0, type_token_ratio=0, rare_word_ratio=0,
            em_dash_per_1000=0, semicolon_per_1000=0, parenthetical_per_1000=0,
            avg_word_length=0, pct_monosyllabic_words=0,
        )

    rare = [w for w in words if w not in COMMON_WORDS]
    syllable_estimate = lambda w: max(1, len(re.findall(r"[aeiouyAEIOUY]+", w)))  # noqa: E731
    mono = sum(1 for w in words if syllable_estimate(w) == 1)

    return VoiceProfile(
        word_count=len(words),
        sentence_count=len(sents),
        paragraph_count=len(paras),
        median_sentence_length=_quartile(sent_lens, 0.50),
        p25_sentence_length=_quartile(sent_lens, 0.25),
        p75_sentence_length=_quartile(sent_lens, 0.75),
        pct_short_sentences=sum(1 for l in sent_lens if l < 8) / len(sent_lens),
        pct_long_sentences=sum(1 for l in sent_lens if l > 25) / len(sent_lens),
        dialogue_ratio=sum(1 for s in sents if _is_dialogue(s)) / len(sents),
        avg_paragraph_length_sentences=len(sents) / max(1, len(paras)),
        type_token_ratio=len(set(words)) / len(words),
        rare_word_ratio=len(rare) / len(words),
        em_dash_per_1000=1000 * text.count("—") / len(words),
        semicolon_per_1000=1000 * text.count(";") / len(words),
        parenthetical_per_1000=1000 * text.count("(") / len(words),
        avg_word_length=sum(len(w) for w in words) / len(words),
        pct_monosyllabic_words=mono / len(words),
    )


def stylometric_distance(a: VoiceProfile, b: VoiceProfile) -> dict:
    """
    Numeric distance between two profiles. We weight the dimensions a human
    reader would notice first: median sentence length, dialogue ratio,
    short-sentence share, em-dash density, vocabulary richness.
    """
    def _diff(an: float, bn: float, scale: float) -> float:
        return abs(an - bn) / max(1e-6, scale)

    # weights sum to 1.0
    components = [
        ("median_sentence_length",      _diff(a.median_sentence_length, b.median_sentence_length, 10), 0.20),
        ("pct_short_sentences",         _diff(a.pct_short_sentences,    b.pct_short_sentences,    0.5), 0.12),
        ("pct_long_sentences",          _diff(a.pct_long_sentences,     b.pct_long_sentences,     0.3), 0.10),
        ("dialogue_ratio",              _diff(a.dialogue_ratio,         b.dialogue_ratio,         0.5), 0.12),
        ("avg_paragraph_length_sentences", _diff(a.avg_paragraph_length_sentences, b.avg_paragraph_length_sentences, 5), 0.08),
        ("type_token_ratio",            _diff(a.type_token_ratio,       b.type_token_ratio,       0.4), 0.10),
        ("rare_word_ratio",             _diff(a.rare_word_ratio,        b.rare_word_ratio,        0.4), 0.08),
        ("em_dash_per_1000",            _diff(a.em_dash_per_1000,       b.em_dash_per_1000,       8), 0.07),
        ("avg_word_length",             _diff(a.avg_word_length,        b.avg_word_length,        2), 0.07),
        ("pct_monosyllabic_words",      _diff(a.pct_monosyllabic_words, b.pct_monosyllabic_words, 0.4), 0.06),
    ]
    weighted = sum(d * w for _, d, w in components)
    # Score from 10 (identical) to 0 (very different).
    score = max(0.0, 10.0 - weighted * 10.0)
    return {
        "distance_score_out_of_10": round(score, 2),
        "components": [{"dim": k, "delta_norm": round(d, 3), "weight": w} for k, d, w in components],
    }


if __name__ == "__main__":
    import json as _json
    sample = (
        "She walked into the kitchen. The light was off. Eight years she'd lived here "
        "and she still flicked the switch on the wrong side every time.\n\n"
        "\"You up?\" she said.\n\n"
        "Nothing. Just the fridge clicking, and somewhere down the hall the cat doing the thing it did "
        "where it knocked a pen off the table for no reason at all."
    )
    p = fingerprint(sample)
    print(_json.dumps(asdict(p), indent=2))
    print()
    print(p.constraints_block(label="sample"))
