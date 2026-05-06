# Vocabulary Dictionaries — BooksForge

**Version:** 1.0.0  •  **Date:** 2026-05-06  •  **Authoritative for the vocabulary subsystem.** Companion to `MEMORY_SYSTEM.md` (style memory cross-references vocab) and `AGENTS.md` (Vocabulary Dictionary Agent + Humanization Agent).

The vocabulary subsystem is what keeps BooksForge's prose from sounding like generic GenAI output. It is a **layered, continuously evolving** set of dictionaries — not a one-time setup, not a single "banned words" list — that adapts to the book's genre, sub-genre, domain, audience, character voices, and chapter type.

This is a deliberate departure from the simplest "banned words" approach because real authors do not write to a global list — a regency romance and a kubernetes textbook need different rules.

---

## 1. Goals

1. **Reduce robotic GenAI prose** by surfacing genre/audience-appropriate alternatives to overused AI tics.
2. **Preserve voice** — never forcibly rewrite where the user's voice is intentional.
3. **Layer correctly** — a sub-genre dictionary overrides a genre dictionary, which overrides a book-level dictionary.
4. **Continuously evolve** — every accepted edit, every Copyeditor proposal, every user revision teaches the dictionaries.
5. **Audit every change** — every dictionary update has a `source` (user, agent id, importer) and a `reason`.

## 2. Layers and lookup order

Dictionaries are layered. A lookup for "should I avoid the word *tapestry* here?" merges layers in this order, with later layers winning on conflict:

1. **Project (book) layer** — user's explicit choices for this book.
2. **Genre layer** — e.g., `fiction.romance`, `fiction.mystery`, `nonfiction.business`, `nonfiction.memoir`.
3. **Sub-genre layer** — e.g., `fiction.romance.regency`, `fiction.mystery.cosy`, `nonfiction.business.startup`.
4. **Domain layer** — e.g., `domain.software`, `domain.medicine`, `domain.law`, `domain.history`. Used in nonfiction; ignored in fiction unless the user explicitly enables it.
5. **Audience layer** — e.g., `audience.adult-trade`, `audience.YA`, `audience.children-mg`, `audience.academic`.
6. **Character-voice layer** (fiction/memoir only) — per-character preferred/avoid lists.
7. **Chapter-type layer** — e.g., `chapter-type.action`, `chapter-type.exposition`, `chapter-type.reflection`, `chapter-type.how-to`.

A dictionary entry can have a **kind**: `prefer`, `avoid`, `replace` (with a target), `caution` (use sparingly with rationale).

## 3. Schema

Three SQL tables in `booksforge-vocab`:

```sql
-- ---------------------------------------------------------------
-- vocab_dictionaries — one row per (layer, key) tuple
-- The "key" is the layer instance, e.g. 'fiction.romance.regency'
-- The 'project' layer has key = the project's manifest id.
-- ---------------------------------------------------------------
CREATE TABLE vocab_dictionaries (
  id              TEXT PRIMARY KEY,                 -- ULID
  layer           TEXT NOT NULL,                    -- 'project'|'genre'|'sub-genre'|'domain'|'audience'|'character-voice'|'chapter-type'
  layer_key       TEXT NOT NULL,                    -- e.g. 'fiction.romance.regency' or '<project_id>' or '<entity_id>'
  display_name    TEXT NOT NULL,
  description     TEXT,
  is_builtin      INTEGER NOT NULL DEFAULT 0,       -- 1 if shipped with the app; 0 if user/agent created
  created_at      TEXT NOT NULL,
  updated_at      TEXT NOT NULL,
  UNIQUE (layer, layer_key)
);

-- ---------------------------------------------------------------
-- vocab_entries — entries belong to a dictionary
-- ---------------------------------------------------------------
CREATE TABLE vocab_entries (
  id              TEXT PRIMARY KEY,                 -- ULID
  dict_id         TEXT NOT NULL REFERENCES vocab_dictionaries(id) ON DELETE CASCADE,
  term            TEXT NOT NULL,                    -- the word/phrase/idiom
  kind            TEXT NOT NULL,                    -- prefer | avoid | replace | caution
  replacement     TEXT,                             -- when kind = replace
  rationale       TEXT,                             -- why
  example_text    TEXT,                             -- usage example
  source          TEXT NOT NULL,                    -- 'user' | 'builtin' | <agent_id>
  source_evidence_json BLOB,                        -- e.g. {"accepted_edit_id": "..."} for traceability
  created_at      TEXT NOT NULL,
  updated_at      TEXT NOT NULL,
  retired_at      TEXT,                             -- soft-delete; entries can be retired but not deleted
  UNIQUE (dict_id, term, kind)
);

-- ---------------------------------------------------------------
-- vocab_updates — append-only audit ledger
-- ---------------------------------------------------------------
CREATE TABLE vocab_updates (
  id              TEXT PRIMARY KEY,                 -- ULID
  entry_id        TEXT REFERENCES vocab_entries(id),
  dict_id         TEXT NOT NULL,
  op              TEXT NOT NULL,                    -- 'create' | 'edit' | 'retire' | 'restore'
  prev_json       BLOB,                             -- snapshot of prev state
  next_json       BLOB,                             -- snapshot of new state
  writer          TEXT NOT NULL,                    -- 'user' | <agent_id>
  reason          TEXT,
  created_at      TEXT NOT NULL
);

CREATE INDEX idx_vocab_entries_dict ON vocab_entries(dict_id);
CREATE INDEX idx_vocab_updates_entry ON vocab_updates(entry_id, created_at);
```

Markdown mirror: `manuscript/.vocab/<layer>/<layer_key>.md` for inspectability.

## 4. Built-in starter dictionaries

BooksForge ships with starter dictionaries for the layers that have broad consensus. They are seeded at template installation time, never auto-modified by the user-facing agents (any agent change creates a new project-layer override entry rather than editing the builtin entry).

### 4.1 The "AI tells" baseline (audience-agnostic, contextual)

These are the words/phrases most associated with GenAI prose. They are **`avoid` by default** in fiction and trade nonfiction; `caution` in academic and technical writing where they may be legitimate.

| Term | Default kind | Notes |
|------|--------------|-------|
| in today's world | avoid | Hollow opener. |
| delve | avoid | Hallmark AI verb. |
| tapestry | avoid | Especially "rich tapestry of …". |
| journey (metaphorical) | caution | Fine literally; suspicious metaphorically in nonfiction. |
| unlock (metaphorical) | avoid | "Unlock potential" / "unlock value". |
| transformative | avoid | Vague hype. |
| seamless | avoid | Vague positive. |
| robust | avoid in fiction; caution in trade nonfiction; allowed in technical | Context-sensitive. |
| cutting-edge | avoid | Cliché. |
| leverage (verb) | caution | Fine in finance/M&A; otherwise replace with "use" / "draw on". |
| it's important to note | avoid | Throat-clearing. |
| whether you're | avoid | Lazy direct-address opener. |
| not only ... but also | avoid | Repetitive corporate construction. |
| at the end of the day | avoid | Cliché. |
| in conclusion | caution | Fine if the chapter actually concludes. |
| moreover | caution | Overused transition. |
| furthermore | caution | Overused transition. |
| in essence | avoid | Often filler. |
| navigating the complexities | avoid | AI cliché. |
| ever-evolving | avoid | Cliché. |
| pioneering / paradigm-shifting | avoid | Vague hype. |
| ensuring | caution | Often verbose; replace with "to make sure" or restructure. |

These are seeded into a builtin dictionary `audience.builtin.ai-tells`. The full starter list lives in `templates/vocab/builtin-ai-tells.toml` and ships with every project; the user can override per-project.

### 4.2 Genre starter dictionaries

Each MVP template (Generic Novel, Romance Novel, General Non-Fiction) ships with a genre starter:

- **`fiction.generic-novel`** — emphasises sensory grounding, restrained adverbs, varied dialogue tags. Bans: "she felt that", "he realised that" (over-narrating), "bleeding eyes" (overused gore cliché). Prefers: showing over telling, scene-anchored verbs.
- **`fiction.romance`** — bans certain melodrama clichés ("his eyes bored into hers"), prefers tactile sensory verbs in dialogue beats. Allows higher emotional intensity.
- **`nonfiction.general`** — bans "in today's world", "unlock", "transformative" (stronger than baseline). Prefers concrete examples and per-claim sourcing. Allows "framework", "principle" as legitimate terms.
- **`audience.adult-trade`** — middle reading-level target; bans academic register ("herein", "thereto") and the most aggressive AI tells.
- **`audience.YA`** — additional caution on "kids these days" framing; prefers contemporary speech patterns; no condescension.
- **`audience.academic`** — re-allows "robust", "framework", formal register; bans "delve" (overused even in academia); prefers passive voice in some contexts.
- **`audience.children-mg`** — caution on multi-syllable words; banned: profanity, mature themes, condescending "as you grow up" framing.

V1.0 expands to: Mystery, Sci-Fi/Fantasy, Memoir, Business/Self-help, Academic Monograph, Cookbook, Textbook, Children-PB.

### 4.3 Domain starter dictionaries (nonfiction only)

- **`domain.software`** — preferred: precise nouns ("function", "service", "endpoint"); avoids "magic", "intelligent" applied to non-AI code.
- **`domain.medicine`** — preferred: clinical specificity; avoids dramatic language; cautions colloquial "cure".
- **`domain.law`** — preferred: precise legal terms with citations; avoids journalistic flourishes.
- **`domain.history`** — preferred: dated terminology with explanations on first use; avoids anachronistic verbs.
- **`domain.finance`** — preferred: numbers with sources; avoids unbacked superlatives.

Each ships with 30–80 entries. Plenty of headroom for community-contributed expansions post-MVP.

### 4.4 Chapter-type starter dictionaries

- **`chapter-type.action`** (fiction) — short sentences preferred; avoid em-dash overuse; avoid "suddenly".
- **`chapter-type.exposition`** — vary sentence length; avoid stacked subordinate clauses; bans "as you may know".
- **`chapter-type.reflection`** — first-person interiority allowed; bans "I thought to myself".
- **`chapter-type.how-to`** (nonfiction) — imperative verbs preferred; second-person addressing audience allowed; avoids passive voice.

## 5. Lookups and merging

The lookup function `vocab.lookup(term, context)` returns a merged decision.

```rust
pub struct VocabContext {
    pub project_id: ProjectId,
    pub genre: Option<String>,
    pub sub_genre: Option<String>,
    pub domain: Option<String>,
    pub audience: Option<String>,
    pub character_voice: Option<EntityId>,    // None unless inside a character's POV scene
    pub chapter_type: Option<String>,
}

pub fn lookup(term: &str, ctx: &VocabContext) -> VocabDecision {
    // Walk layers in priority order:
    // chapter-type → character-voice → audience → domain → sub-genre → genre → project
    // Later layers (further down) win on conflict.
    // Return the highest-priority decision found, or None.
}
```

`VocabDecision` is one of: `Allowed`, `Preferred`, `Avoid { rationale }`, `Replace { with, rationale }`, `Caution { rationale }`.

## 6. The Vocabulary Dictionary Agent

See `AGENTS.md` for the full spec. Summary:

**Purpose.** Maintain the project-layer dictionary by observing accepted edits, accepted Copyeditor / Humanization proposals, and explicit user additions.

**Inputs.** New evidence of language use:

- An accepted user edit (the diff between before/after).
- An accepted Copyeditor proposal (one of its rules added a `replace` to the project dictionary if confirmed).
- An accepted Humanization proposal.
- An explicit user-added entry.

**Outputs.** Proposed updates to the project-layer `vocab_dictionaries` dictionary as `VocabUpdateProposals`:

```json
{
  "proposals": [
    {
      "op": "create" | "edit" | "retire" | "restore",
      "term": "string",
      "kind": "prefer" | "avoid" | "replace" | "caution",
      "replacement": "string?",
      "rationale": "string",
      "evidence_refs": ["string", "..."]
    }
  ]
}
```

**Validation.** Every proposal references at least one piece of evidence (an `accepted_edit_id` or a user note). Replacements have a non-empty `replacement` string. Rationales ≤ 200 chars.

**When to run.** Every chapter finalisation; every batch of 5 accepted Copyeditor edits; on user demand from the Vocabulary tab.

**User gate.** Required. The Vocabulary tab UI lets the user accept/reject each proposal individually. Accepted proposals trigger a `vocab_updates` entry.

## 7. The Humanization Agent

See `AGENTS.md` for the full spec. Summary:

**Purpose.** Surface passages that read as robotic GenAI prose and propose human alternatives, using the merged vocab dictionaries plus style memory.

**Inputs.** A scope of text (paragraph, scene, or chapter), the merged vocab decisions for the scope's context, and the project's `style_memory` (preferred tone, sentence rhythm, banned phrases, repeated phrases).

**Outputs.** `HumanizationProposals`:

```json
{
  "proposals": [
    {
      "range_from": 0,
      "range_to": 0,
      "before": "string",
      "after": "string",
      "category": "ai-tell" | "rhythm" | "register" | "repetition" | "filler",
      "rationale": "string",
      "vocab_entry_id": "string?"           
    }
  ]
}
```

**Validation.** Schema valid; ranges valid; no edit alters word count by >25% per proposal; `category` in enum; if `vocab_entry_id` present, it points to a real entry.

**When to run.** On demand from "Humanize this scene/chapter" command. Scope: scene by default; chapter or project on explicit request with budget warnings.

**User gate.** Required. The UI shows each proposal as an inline diff with the rationale; the user accepts/rejects individually or by category.

The Humanization Agent **observes** patterns — when the user rejects a proposal three times for a particular vocab entry, the entry's confidence is downgraded; when the user accepts proposals consistently, the entry is reinforced. This learning loop is the continuous evolution.

## 8. Continuous evolution — the update loops

The dictionaries grow and shrink over the life of a project through these loops:

1. **Accepted user edit → vocab observation.** When a user replaces "delve into" with "look at", the Vocabulary Dictionary Agent proposes a project-layer entry: `term="delve into", kind=replace, replacement="look at", source=user_edit, evidence={accepted_edit_id}`. The user accepts; entry is added.
2. **Accepted Copyeditor proposal → vocab reinforcement.** Many accepted "smart-quote" or "em-dash" proposals reinforce the project's mechanical style book; the Vocabulary Dictionary Agent does not duplicate those (the Style Book is its own subsystem) but it does pick up word/phrase reinforcements ("preferred phrases").
3. **Accepted Humanization proposal → vocab confidence.** When a Humanization proposal is accepted with rationale "AI-tell", the underlying vocab entry's confidence increases; future proposals using the same entry are surfaced more aggressively.
4. **User-added entries.** The user can add entries directly from the Vocabulary tab. These are project-layer by default; the user can promote a project-layer entry to a custom genre/sub-genre layer that ships with the next plugin pack (post-MVP).
5. **Chapter finalisation pass.** When a chapter is finalised, the Vocabulary Dictionary Agent reviews the chapter for new repeated phrases, new banned-phrase candidates, and new domain terminology. Findings are proposed to the user.
6. **Final review pass (V1.0).** The Final Review Agent (V1.0) does a whole-book vocab consistency check before export.

## 9. UX surface

In the Right Panel of the workspace, a **Vocabulary** tab (V1.0; in MVP this lives inside the Bible tab as a sub-section) shows:

- The active dictionaries by layer with their entry counts.
- A search/filter UI over entries.
- A "Pending proposals" inbox from the Vocabulary Dictionary Agent.
- Edit/add/retire actions on entries, with the audit trail visible.
- Per-scene "Vocab decisions" preview (which entries applied here and why).

## 10. Privacy and locality

Vocabulary lives in the project bundle (`project.db` + `manuscript/.vocab/` mirror). Built-in dictionaries are bundled with the app. Nothing leaves the device.

Note: **the vocabulary is not the manuscript content.** It is metadata about word choices. Even so, it is treated with the same privacy posture: no telemetry, no cloud sync in MVP.

## 11. Reversibility

Every `vocab_updates` row is reversible. The Vocabulary tab exposes a "Revert this update" affordance that restores `prev_json`. A property test asserts that any sequence of updates followed by a sequence of reverts restores the prior state byte-for-byte.

## 12. Acceptance criteria for the vocabulary subsystem

The subsystem is acceptable when:

1. A new project from the Romance template lands with the merged starter dictionaries (`audience.builtin.ai-tells` + `fiction.romance` + `audience.adult-trade`) populated.
2. The Humanization Agent on a fixture passage that contains "in today's world" surfaces a proposal with rationale citing `audience.builtin.ai-tells.in-todays-world`.
3. Accepting a user edit that replaces "tapestry" with "fabric" causes the Vocabulary Dictionary Agent to propose a project-layer `replace` entry, which the user accepts.
4. The Markdown mirror under `manuscript/.vocab/` reflects every dictionary state.
5. A property test: any sequence of vocab updates is reversible byte-for-byte through `vocab_updates`.
6. A 100k-word fixture book runs the Vocabulary Dictionary Agent end-to-end in <30s on the reference hardware.

## 13. Out of scope (V1.0+)

- Cross-project dictionary sharing (a "Romance Author Pack" plugin) — V1.5.
- Vector-embedding-based semantic vocab lookups — V2.0.
- Style-transfer fine-tuning on the user's prior books — V2.0 and only on-device.
- Internet-augmented research that updates domain dictionaries — V2.0.

## 14. Anti-goals (do not build)

- A global "banned words" list applied to all books regardless of context.
- A blunt regex find-and-replace that doesn't know which layer is active.
- An auto-apply Humanization that bypasses the user gate.
- A vocabulary system that requires the user to set up dictionaries before they can write — the starter dictionaries must work out of the box.
