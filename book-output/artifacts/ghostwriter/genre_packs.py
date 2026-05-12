"""
Genre packs — three distinct workflows for literary, genre, and non-fiction.

Each pack supplies:

  - SYSTEM       : the system prompt used at every stage for that vertical
  - DRAFT_LENS   : per-scene drafter system addition that emphasises the
                   craft skill the genre cares about most
  - CRITIC_AXES  : the 4-6 axes the per-scene critic scores (different by
                   genre — literary cares about subtext, genre cares about
                   pacing, non-fiction cares about argument)
  - POLISH_STACK : ordered list of specialist polish passes for this genre
  - RUBRIC       : 12-dimension rubric with PER-GENRE weights so the same
                   raw score means a different weighted total
  - HARD_RULES   : non-negotiables (e.g. no fake stats for non-fiction)

This is the architecture answer to "the same pipeline can't serve all three
verticals". Same orchestrator; three different prompt + weight sets.
"""
from __future__ import annotations

from dataclasses import dataclass, field


@dataclass(frozen=True)
class PolishStage:
    name: str
    purpose: str
    system: str
    user_template: str   # use {chapter} placeholder


@dataclass(frozen=True)
class RubricAxis:
    key: str
    description: str
    weight_literary: float
    weight_genre: float
    weight_nonfiction: float


@dataclass(frozen=True)
class GenrePack:
    name: str
    system: str
    draft_lens: str
    critic_axes: list[str]
    polish_stack: list[PolishStage]
    hard_rules: list[str]


# ── Specialist polish stages ──────────────────────────────────────────────

DIALOGUE_POLISH = PolishStage(
    name="dialogue",
    purpose="Sharpen dialogue. Cut exposition-as-dialogue. Add subtext. Vary cadence between speakers.",
    system=(
        "You are a senior line editor with 15 years of experience editing dialogue for "
        "literary and commercial fiction. Your job is to make EVERY line of dialogue do "
        "at least one of: reveal character, advance plot, or carry subtext. Dialogue that "
        "exists only to convey information to the reader (exposition-as-dialogue) is the "
        "single biggest mark of amateur prose — cut or rewrite it. Each speaker must sound "
        "DIFFERENT — different vocabulary tier, different sentence rhythm, different "
        "evasion patterns. Preserve plot, character names, and word count within ±10%. "
        "Return ONLY the revised chapter — no commentary."
    ),
    user_template=(
        "Polish the dialogue in this chapter. Hard rules:\n"
        "- No 'as you know' exposition.\n"
        "- No two characters sounding identical.\n"
        "- At least 30% of dialogue lines should carry subtext (saying X to mean Y).\n"
        "- Reduce dialogue tags to 'said' / 'asked' wherever possible. Cut adverbs "
        "from tags ('she said angrily' → cut, replace with action beat).\n"
        "- Preserve narrative passages exactly as written; touch only dialogue + "
        "the tags / beats that bracket it.\n\n"
        "Chapter:\n---\n{chapter}\n---"
    ),
)

METAPHOR_POLISH = PolishStage(
    name="metaphor",
    purpose="Replace clichéd images. Tune metaphor density. Make every image specific to this character / world.",
    system=(
        "You are a craft editor specialising in figurative language. Replace clichéd "
        "metaphors and dead similes with fresh, character-specific images. Target "
        "density: roughly 1-2 fresh images per 500 words for literary, 0.5-1 for "
        "genre. EVERY metaphor must come from the POV character's lived "
        "experience — a baker thinks in terms of dough, a soldier in terms of weight "
        "of armour, a grandmother in terms of waiting rooms. Generic-AI metaphors "
        "(tapestry, symphony, kaleidoscope, dance, journey) are forbidden. Preserve "
        "plot, dialogue, and word count. Return ONLY the revised chapter."
    ),
    user_template=(
        "Tune metaphor and imagery in this chapter. Hard rules:\n"
        "- Cut every cliché image (cold as ice, heart of stone, etc).\n"
        "- Replace generic-AI metaphors (tapestry, symphony, kaleidoscope, dance) with "
        "an image from this POV character's specific world.\n"
        "- One genuinely fresh image per 400-600 words is the target — not more.\n"
        "- Preserve dialogue, plot, character actions exactly.\n\n"
        "Chapter:\n---\n{chapter}\n---"
    ),
)

SCENE_TENSION_POLISH = PolishStage(
    name="scene_tension",
    purpose="Tighten the rising line. Cut slack. Make each scene end on a hook.",
    system=(
        "You are a developmental editor. Your job is to ensure every scene has a "
        "rising tension line: a clear scene goal, escalating obstacles, a turn or "
        "reveal in the back third, and a hook ending that makes the reader turn the "
        "page. Cut slack — passages where nothing changes for the protagonist. "
        "Compress repetitive description. Strengthen scene endings. Preserve all "
        "events, characters, dialogue beats, and subplots. Return ONLY the revised "
        "chapter."
    ),
    user_template=(
        "Tighten scene tension in this chapter. Hard rules:\n"
        "- Cut any paragraph in which the protagonist's situation does not change.\n"
        "- The last sentence of the chapter must compel the next chapter — a "
        "question, a reversal, an arrival, a refusal.\n"
        "- Any flashback or backstory aside should be cut to the minimum that "
        "supplies essential information.\n"
        "- Preserve dialogue and plot points exactly.\n\n"
        "Chapter:\n---\n{chapter}\n---"
    ),
)

VOICE_POLISH = PolishStage(
    name="voice",
    purpose="Preserve and amplify author voice. Do NOT flatten.",
    system=(
        "You are a voice-preservation editor. Your ONLY job is to amplify what is "
        "distinctive about this prose — the cadence, the lexicon, the asides, the "
        "specific sentence shapes. You will encounter voice features that look like "
        "'errors' to a generic copyeditor — leave them. Comma splices that work, "
        "fragments that work, repeated words that work — leave them. Your edits "
        "should make the prose sound MORE like itself, never less. Preserve plot, "
        "dialogue, word count. Return ONLY the revised chapter."
    ),
    user_template=(
        "Preserve voice in this chapter. Hard rules:\n"
        "- DO NOT smooth out distinctive sentence shapes.\n"
        "- DO NOT replace specific vocabulary with generic synonyms.\n"
        "- DO NOT add hedge openers, transitional adverbs, or marketing adjectives.\n"
        "- The reader should be able to identify this as the same author across "
        "chapters by sentence cadence alone.\n\n"
        "Chapter:\n---\n{chapter}\n---"
    ),
)

ARGUMENT_POLISH = PolishStage(   # non-fiction-only
    name="argument",
    purpose="Sharpen the argument. Each chapter has a thesis; each section advances it.",
    system=(
        "You are a developmental editor for serious non-fiction (think long-form "
        "essay collections, business strategy, popular science). Your job: ensure "
        "each chapter has a clear thesis, that every section advances the thesis, "
        "that examples are concrete and earn their place, and that no claim is left "
        "unsupported. CRITICAL: do NOT invent statistics, dates, names, studies, or "
        "quotes. If the original prose makes a quantitative claim that needs a "
        "source, mark it [SOURCE NEEDED] inline rather than fabricating one. "
        "Preserve voice and length. Return ONLY the revised chapter."
    ),
    user_template=(
        "Sharpen the argument in this chapter. Hard rules:\n"
        "- Identify and surface the chapter thesis in the first 200 words.\n"
        "- Every section must visibly advance the thesis or the chapter wastes the reader's time.\n"
        "- NEVER fabricate statistics, dates, names, or studies. If an unsupported "
        "quantitative claim appears, replace with [SOURCE NEEDED] or order-of-magnitude language.\n"
        "- Concrete examples beat abstract claims.\n"
        "- Preserve voice and approximate word count.\n\n"
        "Chapter:\n---\n{chapter}\n---"
    ),
)

EVIDENCE_POLISH = PolishStage(   # non-fiction-only
    name="evidence",
    purpose="Tag every quantitative claim. No fake stats. Make uncertainty visible.",
    system=(
        "You are a fact-handling editor. Walk through the chapter and: (1) tag every "
        "quantitative claim and proper-noun reference with one of [VERIFIED], "
        "[SOURCE NEEDED], [APPROXIMATE], [FOR ILLUSTRATION], (2) replace anything "
        "that smells fabricated with order-of-magnitude phrasing or [SOURCE NEEDED], "
        "(3) keep the prose readable. Return the chapter with inline tags only."
    ),
    user_template=(
        "Audit evidence handling in this chapter. Tag every quantitative claim or "
        "proper-noun reference inline. NEVER invent a number to fill in [SOURCE "
        "NEEDED]. Order-of-magnitude phrasing is acceptable. Preserve narrative "
        "voice.\n\nChapter:\n---\n{chapter}\n---"
    ),
)


# ── Rubric axes ───────────────────────────────────────────────────────────

RUBRIC: list[RubricAxis] = [
    RubricAxis("voice",                "Distinctive, sustained, amplified",            weight_literary=3.0, weight_genre=1.5, weight_nonfiction=2.0),
    RubricAxis("prose_quality",        "Sentence-craft, rhythm, image, fresh language", weight_literary=3.0, weight_genre=1.5, weight_nonfiction=1.5),
    RubricAxis("originality",          "Premise + execution NOT derivative",            weight_literary=2.0, weight_genre=1.5, weight_nonfiction=2.0),
    RubricAxis("character_depth",      "Interiority + motivation + arc",                weight_literary=2.0, weight_genre=1.5, weight_nonfiction=0.5),
    RubricAxis("emotional_impact",     "Reader feels, not just understands",            weight_literary=2.0, weight_genre=2.0, weight_nonfiction=1.0),
    RubricAxis("pacing",               "Page-turning, no dead spots",                   weight_literary=1.0, weight_genre=3.0, weight_nonfiction=1.5),
    RubricAxis("hook_strength",        "First page, chapter ends, scene exits",         weight_literary=1.0, weight_genre=2.5, weight_nonfiction=1.5),
    RubricAxis("dialogue",             "Sharp, character-specific, subtext-bearing",   weight_literary=2.0, weight_genre=2.0, weight_nonfiction=0.5),
    RubricAxis("structure",            "Acts, arcs, payoff of setups",                  weight_literary=1.5, weight_genre=2.5, weight_nonfiction=2.5),
    RubricAxis("commercial_readiness", "Saleable on KDP / Apple / Google",              weight_literary=1.0, weight_genre=2.5, weight_nonfiction=2.0),
    RubricAxis("argument_strength",    "Thesis + supporting structure",                 weight_literary=0.5, weight_genre=0.5, weight_nonfiction=3.0),
    RubricAxis("evidence_handling",    "No fake stats; order-of-magnitude when uncertain", weight_literary=0.5, weight_genre=0.5, weight_nonfiction=3.0),
    RubricAxis("authority_voice",      "Reader trusts the narrator's expertise",        weight_literary=0.5, weight_genre=1.0, weight_nonfiction=2.5),
    RubricAxis("continuity",           "No contradictions, names + facts hold",         weight_literary=1.5, weight_genre=2.0, weight_nonfiction=1.5),
    RubricAxis("formatting_readiness", "Headings + structure ready for export",         weight_literary=0.5, weight_genre=0.5, weight_nonfiction=1.0),
]


# ── Genre packs ───────────────────────────────────────────────────────────

LITERARY = GenrePack(
    name="literary_fiction",
    system=(
        "You are working on a LITERARY fiction manuscript. Priorities, in order: "
        "voice, prose at the sentence level, interiority, subtext, originality of "
        "perception. Plot-velocity is secondary. Generic 'AI competent' prose is "
        "rejected — every sentence should feel placed, not generated. The narrator's "
        "specific perception of the world is the product."
    ),
    draft_lens=(
        "Drafting lens: literary. Prefer specificity over abstraction. The "
        "protagonist sees the world through a particular lens — every observation "
        "should reveal what THIS character notices that another wouldn't. Avoid "
        "generic mood-setting; pick one concrete sensory detail and let it carry. "
        "Sentences should vary — short, then long, then short. Subtext over "
        "subtext: dialogue says X, scene says Y, narrator's silence says Z."
    ),
    critic_axes=[
        "scene_goal_clear", "specificity_of_perception", "voice_distinct",
        "subtext_present", "image_freshness", "interiority_earned",
    ],
    polish_stack=[VOICE_POLISH, METAPHOR_POLISH, DIALOGUE_POLISH, SCENE_TENSION_POLISH],
    hard_rules=[
        "Prefer specificity over abstraction.",
        "Every metaphor must come from the POV character's lived world.",
        "Do not flatten distinctive sentence shapes.",
    ],
)

GENRE = GenrePack(
    name="genre_fiction",
    system=(
        "You are working on a GENRE fiction manuscript (cozy fantasy / thriller / "
        "romance / mystery / YA). Priorities, in order: pacing, hooks, character "
        "agency, genre-conventional beats hit reliably. Each scene must turn the "
        "page. Predictable comfort is a feature, not a bug — but execution must "
        "feel fresh within the convention."
    ),
    draft_lens=(
        "Drafting lens: genre fiction. Each scene must (a) set a clear goal in the "
        "first 100 words, (b) escalate in the middle, (c) end on a hook. "
        "Protagonist agency: the protagonist makes a choice that matters in every "
        "scene, never a pure observer. Dialogue carries the story — keep narrative "
        "passages tight. Genre conventions are friends — hit them with style, don't "
        "subvert for the sake of it."
    ),
    critic_axes=[
        "scene_goal_clear", "stakes_visible", "rising_tension",
        "hook_ending", "agency_of_protagonist", "convention_landed_with_flair",
    ],
    polish_stack=[SCENE_TENSION_POLISH, DIALOGUE_POLISH, METAPHOR_POLISH, VOICE_POLISH],
    hard_rules=[
        "Every scene ends on a hook.",
        "Protagonist agency in every scene.",
        "No info-dumps over 100 words.",
    ],
)

NONFICTION = GenrePack(
    name="non_fiction",
    system=(
        "You are working on a NON-FICTION manuscript (strategy / popular science / "
        "long-form essay / business). Priorities, in order: argument structure, "
        "evidence handling, authority voice, reader take-away clarity. NEVER invent "
        "statistics, studies, dates, or quotes — order-of-magnitude phrasing is "
        "acceptable; [SOURCE NEEDED] is acceptable; fabrication is not."
    ),
    draft_lens=(
        "Drafting lens: non-fiction. Every chapter has a thesis stated in the first "
        "200 words. Every section advances the thesis. Examples are concrete and "
        "earn their place — abstract claims without examples are rejected. Voice is "
        "authoritative but not lecturing — the reader is a peer, not a student. "
        "Quantitative claims are tagged [VERIFIED] / [SOURCE NEEDED] / "
        "[APPROXIMATE] / [FOR ILLUSTRATION]."
    ),
    critic_axes=[
        "thesis_clear", "argument_advances", "evidence_handled_honestly",
        "examples_concrete", "authority_voice", "no_fabricated_specifics",
    ],
    polish_stack=[ARGUMENT_POLISH, EVIDENCE_POLISH, VOICE_POLISH, SCENE_TENSION_POLISH],
    hard_rules=[
        "NEVER fabricate stats, dates, names, studies, or quotes.",
        "Every chapter has an explicit thesis.",
        "Tag uncertain quantitative claims inline.",
    ],
)


PACKS: dict[str, GenrePack] = {
    "literary": LITERARY,
    "genre":    GENRE,
    "non-fiction": NONFICTION,
    "nonfiction":  NONFICTION,
}


def weights_for(pack: GenrePack) -> dict[str, float]:
    """Return the per-axis weights for the chosen pack."""
    out: dict[str, float] = {}
    for ax in RUBRIC:
        out[ax.key] = (
            ax.weight_literary if pack.name == "literary_fiction" else
            ax.weight_genre if pack.name == "genre_fiction" else
            ax.weight_nonfiction
        )
    return out


if __name__ == "__main__":
    import json as _json
    for k, p in PACKS.items():
        print(f"=== {k} → {p.name} ===")
        w = weights_for(p)
        print(_json.dumps({"weights": w, "polish_stack": [s.name for s in p.polish_stack]}, indent=2))
