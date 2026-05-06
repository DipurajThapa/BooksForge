# Memory System — BooksForge

**Version:** 1.0.0  •  **Date:** 2026-05-06  •  **Authoritative for the memory subsystem.** Companion to `DATA_MODEL.md` (schema), `AGENTS.md` (agents that touch memory), `VOCABULARY_DICTIONARIES.md` (separate but related).

The memory system is what keeps a 100k-word book internally consistent. Without it, agents drift, characters change spelling, claims contradict between chapters, and the prose rhythm slowly degrades. With it, BooksForge can edit chapter 24 with chapter 1 still in mind.

Memory is **continuous, typed, scoped, and audited**. It is not an afterthought.

---

## 1. Goals

1. **Continuity.** Names, dates, places, claims, terminology, and tone remain consistent across the whole book.
2. **Auditability.** Every memory write records who wrote it (user, agent, importer) and when.
3. **Reversibility.** Any memory write can be reverted; a snapshot precedes every accepted agent edit that produces a memory mutation.
4. **Token-efficiency.** Memory is queryable in chunks small enough to fit in a 7B-Q4 model's context window without summarising the whole book.
5. **Mode-aware.** Fiction memory looks different from nonfiction memory; the schema accommodates both via typed entity kinds.

## 2. Layers

Four layers, each its own SQL table family. Together they form the memory.

| Layer | Scope | Updated by | Read by |
|-------|-------|------------|---------|
| **Book memory** | Project-wide | Memory Curator Agent, user | Every agent |
| **Chapter memory** | Per chapter | Memory Curator Agent, user | Most agents |
| **Entity memory** | Per character/concept/term | Continuity Agent, user, importer | Most agents |
| **Style memory** | Per project (with sub-scopes) | Style Guide Agent (V1.0), Humanization Agent, user | Drafting + editing agents |

A separate **vocabulary** subsystem (see `VOCABULARY_DICTIONARIES.md`) is closely related but kept apart because it has different update cadence and lookup semantics.

## 3. Book-level memory

One row per project (singleton enforced by primary key).

```sql
CREATE TABLE book_memory (
  id                INTEGER PRIMARY KEY CHECK (id = 1),
  -- Identity
  title             TEXT NOT NULL DEFAULT '',
  subtitle          TEXT NOT NULL DEFAULT '',
  series            TEXT,
  series_position   INTEGER,
  -- Classification
  mode              TEXT NOT NULL,                 -- fiction | non_fiction | memoir | academic
  genre             TEXT,
  sub_genre         TEXT,
  domain            TEXT,                          -- e.g. 'software' | 'medicine' | 'history' | NULL for fiction
  audience          TEXT,                          -- e.g. 'adult-trade' | 'YA' | 'academic' | 'beginner-business'
  reading_level     TEXT,                          -- 'middle-grade' | 'YA' | 'adult' | 'academic' | etc.
  -- Voice & structure
  tone              TEXT,                          -- e.g. 'literary' | 'journalistic' | 'warm-instructional'
  voice             TEXT,                          -- e.g. 'first-person' | 'omniscient' | 'authorial-we'
  pov               TEXT,                          -- e.g. 'first' | 'third-limited' | 'third-omniscient' | 'second'
  tense             TEXT,                          -- 'past' | 'present'
  structure         TEXT,                          -- e.g. 'three-act' | 'hero-journey' | 'problem-solution'
  -- Promise
  themes_json       BLOB,                          -- JSON array
  core_promise      TEXT,                          -- one sentence
  constraints_json  BLOB,                          -- JSON: e.g. ['no profanity', 'PG-13 violence']
  -- Style anchor
  canonical_style_rules_json BLOB,                  -- JSON of opinionated style choices, e.g. Oxford comma, em-dash style
  -- Audit
  created_at        TEXT NOT NULL,
  updated_at        TEXT NOT NULL,
  last_writer       TEXT NOT NULL DEFAULT 'user'   -- user | <agent_id> | importer
);
```

Markdown mirror of book memory: `manuscript/.memory/book.md`.

## 4. Chapter memory

One row per chapter node. Updated by the Memory Curator Agent when a chapter is saved or finalised, and by the user via inline editing.

```sql
CREATE TABLE chapter_memory (
  node_id            TEXT PRIMARY KEY REFERENCES nodes(id) ON DELETE CASCADE,
  -- Synopsis layers
  one_line_summary   TEXT,                          -- ≤ 140 chars
  paragraph_summary  TEXT,                          -- ≤ 600 chars
  full_summary       TEXT,                          -- ≤ 2,500 chars
  purpose            TEXT,                          -- "Sets up X; pays off Y"
  -- Specifics
  key_events_json    BLOB,                          -- fiction: events; nonfiction: claims/arguments
  introduces_json    BLOB,                          -- entity_ids first introduced here
  reintroduces_json  BLOB,                          -- entity_ids re-mentioned with new info
  setting            TEXT,                          -- where/when this chapter takes place
  in_world_date      TEXT,                          -- ISO-like for ordered timelines, free text otherwise
  -- Loops
  open_loops_json    BLOB,                          -- questions/threads opened here
  resolved_loops_json BLOB,                         -- threads resolved here
  -- Terminology
  terms_used_json    BLOB,                          -- vocabulary items used (cross-references vocab subsystem)
  -- Continuity
  continuity_notes   TEXT,                          -- free-text notes from Continuity Agent + user
  -- Status
  draft_status       TEXT NOT NULL DEFAULT 'planned', -- planned | drafting | revised | final
  finalised_at       TEXT,
  -- Audit
  updated_at         TEXT NOT NULL,
  last_writer        TEXT NOT NULL DEFAULT 'user'
);
```

Markdown mirror: `manuscript/.memory/chapters/<part>/<chapter>.md` for human inspection.

### 4.1 Update lifecycle

The Memory Curator Agent runs in three triggers:

1. **On scene save** — refreshes the parent chapter's `terms_used_json` (extracted) and the `paragraph_summary` if the user explicitly opts in (default off — too noisy otherwise).
2. **On chapter finalise** — runs a full pass: regenerates `one_line_summary`, `paragraph_summary`, `full_summary`, `key_events_json`, `introduces_json`, `reintroduces_json`, `open_loops_json`, `resolved_loops_json`. User-gated.
3. **On user demand** — "Refresh memory for this chapter" command from the Bible / Memory tab.

## 5. Entity memory

One row per entity in the series bible. The schema accommodates fiction and nonfiction with a typed `kind` and a structured `fields_json`.

```sql
CREATE TABLE entity_memory (
  entity_id          TEXT PRIMARY KEY REFERENCES entities(id) ON DELETE CASCADE,
  -- Identity
  canonical_name     TEXT NOT NULL,
  kind               TEXT NOT NULL,                 -- character | location | item | organisation | theme
                                                    -- (nonfiction): concept | term | claim | source |
                                                    -- definition | framework | case_study | acronym
  -- Fiction-relevant
  appearance         TEXT,                          -- physical description summary (character)
  motivations        TEXT,                          -- character motivations
  voice_patterns     TEXT,                          -- speech tics, vocabulary, register
  emotional_arc      TEXT,                          -- arc summary
  relationships_json BLOB,                          -- {related_entity_id: relationship_kind}
  backstory          TEXT,
  timeline_json      BLOB,                          -- ordered events involving this entity
  -- Nonfiction-relevant
  definition         TEXT,                          -- for concept | term | definition
  authority          TEXT,                          -- "according to X (year)" pointer
  source_id          TEXT,                          -- reference to bibliography
  framework_steps_json BLOB,                        -- for framework
  case_study_summary TEXT,                          -- for case_study
  acronym_expansion  TEXT,                          -- for acronym
  -- Cross-cutting
  aliases_json       BLOB,                          -- short list, also reflected in entity_aliases table
  contradictions_json BLOB,                          -- automatic + user-flagged contradictions across the book
  -- Audit
  created_at         TEXT NOT NULL,
  updated_at         TEXT NOT NULL,
  last_writer        TEXT NOT NULL DEFAULT 'user'
);
```

Markdown mirror: `manuscript/.memory/entities/<kind>/<id>.md`.

### 5.1 Contradiction detection

The Continuity Agent's deterministic linter runs on chapter save: it checks `entity_memory` against every appearance's mention to detect:

- Spelling drift (`entity_aliases` table is the authority on canonical vs. alias).
- Contradictory facts (e.g., "blue eyes" in chapter 3 vs. "brown eyes" in chapter 17 — flagged because both are simple field deltas).
- Timeline inversions (entity_timeline_json events out of order).
- POV/tense violations within a scene (per `chapter_memory`).

Findings flow through the Continuity Agent's adjudication step (see `AGENTS.md §4.5`).

## 6. Style memory

One row per project plus a history table for reversion.

```sql
CREATE TABLE style_memory (
  id                       INTEGER PRIMARY KEY CHECK (id = 1),
  -- Tone & rhythm
  preferred_tone           TEXT,                    -- 'literary' | 'plain' | 'punchy' | 'warm' | 'academic'
  sentence_rhythm          TEXT,                    -- 'varied' | 'short' | 'flowing' | 'mixed'
  reading_level_target     INTEGER,                 -- Flesch-Kincaid target
  narrative_distance       TEXT,                    -- 'intimate' | 'mid' | 'distant' (fiction)
  formality                TEXT,                    -- 'informal' | 'mid' | 'formal'
  humor_level              TEXT,                    -- 'none' | 'light' | 'wry' | 'comedic'
  emotional_intensity      TEXT,                    -- 'restrained' | 'mid' | 'intense'
  -- Patterns
  repeated_phrases_json    BLOB,                    -- phrases frequently observed; agents avoid over-using
  banned_phrases_json      BLOB,                    -- phrases the user has banned for this project
  overused_constructions_json BLOB,                 -- e.g. "she felt that" patterns flagged
  -- Humanization
  humanization_rules_json  BLOB,                    -- per-project anti-AI prose rules; layered with vocab
  -- Audit
  updated_at               TEXT NOT NULL,
  last_writer              TEXT NOT NULL DEFAULT 'user'
);

CREATE TABLE style_memory_history (
  id                INTEGER PRIMARY KEY AUTOINCREMENT,
  changed_at        TEXT NOT NULL,
  field             TEXT NOT NULL,
  prev_value_json   BLOB,
  new_value_json    BLOB,
  writer            TEXT NOT NULL,                  -- user | <agent_id>
  reason            TEXT
);
```

`style_memory.banned_phrases_json` and `humanization_rules_json` interact with the **vocabulary subsystem** (see `VOCABULARY_DICTIONARIES.md`). Banned phrases here are project-wide; vocabulary entries are layered (genre, audience, etc.).

## 7. Memory access by agent

Each agent declares the memory tables it may **read** and **write**. The Orchestrator enforces the scopes by rejecting out-of-scope writes.

| Agent | Reads | Writes |
|-------|-------|--------|
| Project Intake | — (no project yet) | `book_memory` (initial draft) |
| Outline Architect | `book_memory` | None |
| Memory Curator | `book_memory`, `chapter_memory`, `entity_memory` | `chapter_memory`, `entity_memory` (new entries) |
| Vocabulary Dictionary | `book_memory`, `style_memory`, vocab tables | vocab tables, `style_memory.repeated_phrases_json`, `style_memory.overused_constructions_json` |
| Chapter Drafter (opt-in) | `book_memory`, `chapter_memory` (this chapter only), `entity_memory` (cards present in scene), `style_memory`, vocab tables | None |
| Developmental Editor | `book_memory`, `chapter_memory` (this chapter), `entity_memory` (referenced) | None |
| Continuity | `book_memory`, all `chapter_memory`, all `entity_memory` | `entity_memory.contradictions_json` (proposals), `entity_memory.aliases_json` (proposals) |
| Copyeditor | `style_memory`, `style_book` (in `DATA_MODEL.md`), vocab tables | None |
| Humanization | `style_memory`, vocab tables | `style_memory.repeated_phrases_json` (observed), `style_memory.overused_constructions_json` (observed) |

V1.0 agents (Style Guide, Line Editor, Fact-Check, Research Organizer, etc.) have their own scopes documented in `AGENTS.md`.

## 8. Memory writes are user-gated by default

The same orchestrator rule that protects manuscript edits applies to memory: every agent-proposed memory write is a **proposal** that requires user accept. On accept:

1. A pre-edit snapshot is taken (using the same `pre_agent_edit` snapshot mechanism — memory is part of the snapshot scope).
2. The write is applied.
3. The history table records the change with `writer = '<agent_id>'`.

The user can revert any memory write from the Memory tab.

## 9. Memory in the agent context bundle

Before any agent run, the Orchestrator's `ContextBuilder` (see `AGENTS.md §5`) assembles memory into the prompt context. The rules:

- The agent's `memory_reads` declaration drives selection.
- Items are ranked by relevance (currently: explicit references > scene-overlap > recency > whole-book defaults).
- The token budget is the agent's declared `context_budget`. Items dropped first are: whole-book defaults, then recency-only items, then scene-overlap items.
- The user sees the assembled context in the Context Preview UI (per `UI_UX_SPEC.md §6.2`). The send equals the preview.

A worked example for the Continuity Agent on chapter 17:

1. Always-on: `book_memory` core fields (mode, tone, voice, POV, tense, themes, canonical_style_rules) — ~300 tokens.
2. Per-scope: `chapter_memory` for chapter 17 — ~500 tokens.
3. Per-scope: every `entity_memory` row whose canonical_name or alias appears in chapter 17 — variable, capped at 4,000 tokens; oldest-mentioned first dropped if over budget.
4. Per-scope: deterministic continuity findings from the linter — variable.
5. Optional: `chapter_memory` for chapters 1–16 (one-line summaries) — used when the user enables "whole-book context" — ~1,500 tokens.

## 10. Memory storage and recovery

- **Source of truth**: SQLite tables above.
- **Markdown mirror**: under `manuscript/.memory/`. Regenerated on every memory commit (best-effort, after the SQLite write).
- **Recovery**: if `project.db` is corrupted, the Markdown mirror is enough to reconstruct ≥95% of memory via re-import (similar to the manuscript-mirror recovery path).
- **Encryption**: post-MVP. MVP relies on filesystem permissions.

## 11. Memory QA

The `memory-system-review` Claude Code skill (see `CLAUDE_CODE_SKILLS_SPEC.md`) is invoked before merging any change touching memory. It checks:

- Schema invariants (each `chapter_memory.node_id` references an existing chapter; `entity_memory.entity_id` references the entity).
- Markdown mirror exists and matches.
- Cross-chapter contradiction detection: every chapter's `key_events_json` is consistent with its `entity_memory` references.
- Audit completeness: every memory write has a `last_writer` and an updated `updated_at`.

## 12. Acceptance criteria for the memory subsystem

The subsystem is acceptable when:

1. A 100k-word fixture book has a fully populated memory after the Memory Curator completes a one-time backfill run.
2. Editing chapter 1 to rename a character and accepting the Continuity Agent's rename proposal updates `entity_memory.aliases_json` and `entity_memory.canonical_name` atomically with a pre-edit snapshot.
3. The Markdown mirror under `manuscript/.memory/` exists for every chapter and entity.
4. Killing the app mid-memory-write leaves no corrupt state on relaunch (the recovery flow handles it).
5. The agent context preview UI shows exactly the memory rows that will be sent — no hidden additions.
6. A property test asserts: any sequence of accepted memory writes followed by a snapshot restore restores the prior memory state byte-for-byte (modulo timestamps).

## 13. Out of scope (V1.0+)

- Cross-project memory sharing (e.g., a series bible used across books). V1.5.
- Memory-vs-manuscript contradictions surfaced as live banners while typing. V1.0.
- Auto-extraction of entity backstory from manuscript text using LLM-assisted extraction. V1.0 (until then, the Memory Curator updates entities only when triggered, and the user can edit cards directly).
- Vector-embedding-backed memory retrieval for very long books. V1.5.

These are tracked in `_deep/12-risk-register.md` as deferred items.
