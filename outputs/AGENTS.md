# Agents — BooksForge

**Version:** 1.0.0  •  **Date:** 2026-05-06

This document defines the agent swarm. Read `ARCHITECTURE.md §6` first for the orchestrator. Read `PRODUCT_REQUIREMENTS.md §5` for which agents are MVP versus V1.0.

---

## 1. Design principles

The swarm is intentionally small, bounded, and explicit. Five rules govern every agent.

1. **Bounded purpose.** Each agent does one thing. If a task spans two purposes, it is two agents called in sequence by the orchestrator.
2. **No tools, no recursion.** In MVP, agents are pure prompt → text. They cannot call other agents, cannot invoke tools, cannot mutate the manuscript. The orchestrator is the only mutator. (Tools come post-MVP behind capability tokens.)
3. **Schema in, schema out.** Every agent declares an input schema and an output schema. Outputs that fail schema validation are **never silently retried more than twice**; after that, the orchestrator surfaces the raw output for the user to inspect.
4. **Local-first.** Every MVP agent must produce useful output on a 7B-Q4 model running locally via Ollama. Cloud or larger models may improve quality, but no agent is gated on them.
5. **User-gated mutation.** No agent's output is applied to the manuscript without an explicit user accept and a pre-edit snapshot.

## 2. Agent catalog

The agent catalog is hard-coded in `booksforge-agents/src/registry.rs` — agents cannot be added by configuration alone. The MVP runs **ten user-visible LLM agents** plus the always-present Orchestrator: the nine listed below (`intake` through `humanization`) plus the **Final Review Editor** (project addition introduced for world-class final-pass polish; see §4.11). One additional **internal** agent — the **Proposal Validator** (§4.12) — is registered in the same file but excluded from `mvp_agents()` because it runs as an orchestrator-grade verifier, never as a user-selectable workflow target. The remaining nine agents are V1.0+.

| Phase | ID | Name | Purpose |
|-------|----|------|---------|
| MVP | `intake` | Project Intake Agent | Turn a free-text idea into a structured project brief |
| MVP | `outline-architect` | Outline Architect Agent | Propose a chapter/scene outline from a brief |
| MVP | `memory-curator` | Memory Curator Agent | Maintain book/chapter/entity memory; refresh summaries on finalise |
| MVP | `vocab-dictionary` | Vocabulary Dictionary Agent | Maintain project-layer vocabulary dictionaries from accepted edits |
| MVP | `chapter-drafter` | Chapter Drafting Agent | Draft a scene from a synopsis (off by default) |
| MVP | `dev-editor` | Developmental Editor Agent | Produce structural notes per chapter |
| MVP | `continuity` | Continuity Agent | Flag name drift, POV violations, timeline issues |
| MVP | `copyeditor` | Copyeditor Agent | Mechanical fixes (punctuation, spacing, em-dashes) |
| MVP | `humanization` | Humanization Agent | Surface AI-tells; propose human-sounding alternatives using vocab + style memory |
| Always | `orchestrator` | Orchestrator | Controller (not an LLM agent — composes the others under hard caps and approval gates) |
| V1.0 | `book-strategy` | Book Strategy Agent | Audience/genre analysis and positioning notes |
| V1.0 | `research-organizer` | Research Organizer Agent | Tag and summarise imported research notes |
| V1.0 | `chapter-planner` | Chapter Planning Agent | Per-chapter scene plan from outline + research |
| V1.0 | `line-editor` | Line Editor Agent | Passage-level prose revisions |
| V1.0 | `style-guide` | Style Guide Agent | Detect voice/tone drift; enforce project style book |
| V1.0 | `fact-check` | Fact-Check Agent | Surface claims that warrant verification (no internet access; bibliography-grounded) |
| V1.0 | `formatting` | Formatting Agent | Decide template-vs-override conflicts; only when a deterministic rule is insufficient |
| V1.0 | `epub-export-qa` | ePUB Export QA Agent | Read EPUBCheck + visual regression results; propose user-friendly fixes |
| V1.0 | `final-review` | Final Review Agent | Pre-export readiness sweep; gathers issues from prior agents and validators |

The full list is **19 names** including the Orchestrator. The Orchestrator is the controller (not an LLM agent), so when speaking of "LLM agents" we count **9 in MVP / 18 total**.

Two roles **stay rule-based** in MVP because deterministic logic does the job better:

- **Formatting** — handled by the rule-based formatting engine plus templates. The V1.0 `formatting` agent is invoked only when a template-vs-override conflict cannot be resolved deterministically.
- **Export** — handled by the rule-based pipeline (`booksforge-export-epub` + Pandoc sidecar). The V1.0 `epub-export-qa` agent reads the QA results (EPUBCheck JSON, visual regression diffs) and proposes user-friendly remediations; it does not generate the EPUB.

We resist the urge to make every feature an agent. Hallucination risk and audit complexity make agents the wrong tool for deterministic problems.

## 3. Agent definition format

Every agent has the following fields. The fields live in code (`booksforge-agents/src/<agent_id>.rs`) — they are not loaded from disk, so they cannot be expanded at runtime.

```rust
pub struct AgentSpec {
    pub id: &'static str,                  // stable identifier
    pub name: &'static str,                // human-readable name
    pub purpose: &'static str,             // one sentence
    pub input_schema_id:  &'static str,    // name of the input type (lives in booksforge-domain)
    pub output_schema_id: &'static str,    // name of the output type
    pub prompt_template:  PromptTemplateId,  // versioned, hash-pinned
    pub model_preference: ModelPreference,   // family preference + size hint
    pub pinned_model: Option<&'static str>,  // optional exact-tag pin (e.g. "qwen3.6:latest" for FRE)
    pub context_budget: ContextBudget,       // token caps per slot
    pub validators: &'static [CrossCuttingValidator],  // bound at orchestrator-binding time
    pub failure_modes: &'static [FailureMode],
    pub when_to_run: WhenToRun,             // automatic | on_demand | scheduled
    pub user_gate: UserGate,                // required | not_required
}
```

The implementation carries **12 canonical fields plus `pinned_model`** (project addition for high-end agents like the Final Review Editor).

**Validation** runs in two stages:

1. **Structural.** `serde_json::from_str::<OutputType>(raw)` against the type named by `output_schema_id`. The Rust types live in `booksforge-domain::agent_io` and are the source of truth; JSON Schemas are derivable from them.
2. **Semantic.** Per-type `validate()` (e.g. `CopyeditProposals::validate(source_text)` enforces `before`-matches-source and the ≤10 % word-count rule) plus the cross-cutting validators declared on the agent's `validators` slice (`Schema`, `Redaction`, `Length`, optionally `EntitySanity` and `MemoryScope`).

`PromptTemplateId` resolves to a TOML template under `booksforge-prompt/templates/<id>/<version>.toml` and a hash that is recorded on every run.

### 3.1 The prompt template format

Same as `08-ai-integration §5`, with one addition for agents: a `[render.json_schema]` section that is sent to Ollama as a JSON-mode constraint when the model supports it. Example for `outline-architect`:

```toml
[template]
id = "outline-architect.v1"
schema_version = 1
description = "Propose a chapter/scene outline from a project brief."

[input.required]
brief = { kind = "json", schema = "ProjectBrief" }
target_chapter_count = { kind = "int", min = 6, max = 60 }

[input.optional]
genre_overlay = { kind = "string", default = "" }

[render.system]
text = """
You are an experienced developmental editor and book architect. Your job is
to propose a chapter and scene outline that delivers the brief. Be concrete:
each chapter has a one-sentence purpose and 2–4 scenes with one-sentence
synopses. Do not write prose. Do not invent facts. Use the language of the
brief. Output strictly valid JSON matching the provided schema.
"""

[render.user]
text = """
PROJECT BRIEF:
<<<USER_CONTENT>>>
{{ brief | to_json }}
<<<END_USER_CONTENT>>>

TARGET CHAPTER COUNT: {{ target_chapter_count }}
{% if genre_overlay %}GENRE OVERLAY: {{ genre_overlay }}{% endif %}

Return JSON of shape OutlineProposal:
{{ output_schema_as_pseudocode }}
"""

[render.json_schema]
ref = "OutlineProposal"
```

The orchestrator renders the template with MiniJinja, fences `<<<USER_CONTENT>>>` blocks, and instructs the model to ignore embedded instructions inside fences. Output is parsed against the JSON Schema.

## 4. Agent specifications (MVP)

The following specifications are the contract. Claude Code implements one agent file per ID.

### 4.1 Project Intake Agent (`intake`)

**Purpose.** Turn the user's free-text book idea into a structured `ProjectBrief`.

**Inputs.** A `RawIdea` object: `{ "idea_text": string, "preferred_mode": "fiction" | "non_fiction" | "memoir" | "academic" | null }`. Free-text idea ≤4,000 characters.

**Outputs.** `ProjectBrief`:

```json
{
  "title_suggestions": ["string", "..."],
  "mode": "fiction" | "non_fiction" | "memoir" | "academic",
  "genre": "string",
  "audience": "string",
  "tone": "string",
  "target_word_count": 0,
  "premise": "string (1–3 sentences)",
  "key_promises": ["string", "..."],
  "questions_for_user": ["string", "..."]
}
```

**Validation.** Schema valid; `mode` is one of the four; `target_word_count` between 5,000 and 250,000; `key_promises.length` between 1 and 6; `questions_for_user.length` ≤ 5.

**Model preference.** Any 7B+ instruct model. Llama 3.1 8B preferred for English; Qwen 2.5 7B for non-English idea text.

**Context budget.** Idea text + prompt ≤ 4,000 tokens.

**When to run.** On demand from "New Project" wizard or from "Refine brief" in project settings.

**User gate.** Required. The user reviews and edits the brief before any other agent runs.

**Failure modes.** Empty idea, off-topic idea (e.g., "write me a poem"), or non-book content. The agent declares a `not_a_book` flag in the brief if confidence is low; the orchestrator surfaces it.

**Prompt sketch.** "You are a senior acquisitions editor helping a writer turn a one-paragraph pitch into a structured brief. Do not write the book. Do not invent facts. Ask up to five questions if anything critical is missing. Output strictly valid JSON."

### 4.2 Outline Architect Agent (`outline-architect`)

**Purpose.** Propose a chapter and scene outline from a `ProjectBrief`.

**Inputs.** `{ "brief": ProjectBrief, "target_chapter_count": int, "genre_overlay": string? }`.

**Outputs.** `OutlineProposal`:

```json
{
  "parts": [
    {
      "title": "string",
      "purpose": "string",
      "chapters": [
        {
          "title": "string",
          "purpose": "string (one sentence)",
          "scenes": [
            {
              "synopsis": "string (one sentence)",
              "pov": "string?",
              "beat": "string?",
              "target_word_count": 0
            }
          ]
        }
      ]
    }
  ],
  "rationale": "string (≤300 words)",
  "notes_to_user": ["string", "..."]
}
```

**Validation.** Schema valid; total chapter count within ±20% of `target_chapter_count`; total target word count within ±20% of `brief.target_word_count`; every scene has a non-empty synopsis; no two scenes have identical synopses.

**Model preference.** Llama 3.1 8B (long-context-friendly), Qwen 2.5 7B as fallback.

**Context budget.** Brief + prompt ≤ 6,000 tokens; output ≤ 8,000 tokens; total ≤ 16,000 tokens.

**When to run.** From the new-project wizard after intake; from "Regenerate outline" in project settings.

**User gate.** Required. The user accepts/edits the outline; on accept the document tree is created from it.

**Failure modes.** Outline collapses to "Chapter 1: Beginning, Chapter 2: Middle, Chapter 3: End." A semantic validator rejects outlines with >40% identical synopsis tokens; the orchestrator retries once with a "be more specific" reminder.

### 4.3 Chapter Drafting Agent (`chapter-drafter`)

**Purpose.** Draft a scene from a synopsis. **This agent is opt-in per workflow — drafting is off by default**, because using it well requires careful tone matching.

**Inputs.** `{ "scene_synopsis": string, "preceding_scene_summary": string?, "character_cards": EntityCard[], "tone_preset": string, "target_word_count": int, "style_examples": string[]? }`.

**Outputs.** `SceneDraftProposal`:

```json
{
  "draft_text": "string (Markdown)",
  "approximate_word_count": 0,
  "warnings": ["string", "..."]
}
```

**Validation.** Word count within ±25% of target; no Markdown code fences leaking system instructions; no proper noun appearing in the draft that is not in `character_cards` plus a small allowlist (places, things — flagged as warnings, not errors).

**Model preference.** Largest available — Qwen 2.5 7B or Llama 3.1 8B minimum; 13B preferred if RAM allows.

**Context budget.** All inputs + prompt ≤ 16,000 tokens; output ≤ 4,000 tokens.

**When to run.** Only when explicitly invoked from a scene with the "Draft this scene" command. Never automatically.

**User gate.** Required. The user reads the draft and decides; on accept the orchestrator places it into a draft buffer the user can edit before merging.

**Failure modes.** Tone mismatch is common on small models; the agent emits a `tone_confidence_low` warning when any of the style examples differ markedly from the proposed draft (rough cosine similarity on simple word features). Long, repetitive output is rejected with a length-violation retry.

### 4.4 Developmental Editor Agent (`dev-editor`)

**Purpose.** Produce structural notes for a chapter — pacing, scene goals, character motivations, structural problems. Critique only; no rewriting.

**Inputs.** `{ "chapter_text": string, "outline_context": OutlineProposal, "character_cards": EntityCard[], "rubric": Rubric? }`.

**Outputs.** `DevelopmentalNotes`:

```json
{
  "summary": "string (≤120 words)",
  "issues": [
    {
      "severity": "high" | "medium" | "low",
      "category": "pacing" | "structure" | "characterization" | "stakes" | "scene_goal" | "other",
      "location_hint": "string (e.g., 'opening paragraphs', 'scene 2 climax')",
      "diagnosis": "string (1–3 sentences)",
      "suggestion": "string (1–3 sentences) | null"
    }
  ],
  "strengths": ["string", "..."]
}
```

**Validation.** Schema valid; issues count between 0 and 12; every issue has a non-empty diagnosis; severity distribution sane (not "all high"); suggestions are advisory, never imperative ("the chapter must…" → rejected and retried).

**Model preference.** Llama 3.1 8B for English; Qwen 2.5 7B for non-English; 13B preferred where RAM allows for more nuanced critique.

**Context budget.** Chapter text + outline excerpt + cards ≤ 24,000 tokens; output ≤ 4,000 tokens. If chapter > context, run per scene and aggregate (orchestrator handles the chunking).

**When to run.** On demand from "Developmental review" command on a chapter or the project; user gate required.

**User gate.** Required.

**Failure modes.** Hallucinated character names. The orchestrator runs an `EntitySanityCheck` over the output: any proper noun not in the character cards or a small place/thing allowlist is highlighted in the UI for the user. Generic-sounding notes ("show, don't tell") are flagged in a `dev_editor_quality_low` warning.

### 4.5 Continuity Agent (`continuity`)

**Purpose.** Flag name/place spelling drift, point-of-view violations, tense drift, and timeline contradictions across the project. Hybrid: deterministic linter first, LLM adjudicator on ambiguous matches.

**Inputs.** `{ "project_view": ProjectView, "deterministic_findings": ContinuityFinding[] }`.

The orchestrator runs the deterministic continuity linter first (in `booksforge-validator::continuity` — `lint_scene` plus per-kind detectors `detect_name_drift`, `detect_pov_drift`, `detect_tense_drift`, `detect_timeline`), then sends only **ambiguous** findings (`ContinuityFinding.ambiguous == true`) to this LLM adjudicator. High-confidence findings (e.g. a proper noun within Levenshtein-distance 2 of a known alias) skip the LLM entirely. This keeps token use small.

**Outputs.** `ContinuityReport`:

```json
{
  "findings": [
    {
      "kind": "name_drift" | "pov_drift" | "tense_drift" | "timeline" | "other",
      "severity": "error" | "warning" | "info",
      "evidence": [
        { "node_id": "ULID", "range_from": 0, "range_to": 0, "excerpt": "string" }
      ],
      "diagnosis": "string",
      "proposed_fix": {
        "kind": "rename" | "annotate" | "none",
        "from": "string?",
        "to": "string?",
        "scope": "scene" | "chapter" | "project"
      }
    }
  ]
}
```

**Validation.** Every `node_id` exists; every `range_from < range_to`; every excerpt ≤ 200 characters; severities and kinds in enum; proposed renames don't conflict with each other.

**Model preference.** 7B+ instruct, JSON-mode capable. Qwen 2.5 7B is a strong default for multilingual.

**Context budget.** Per-finding adjudication: ≤ 3,000 tokens. Run in batches.

**When to run.** On demand from "Continuity check" command; on save of a chapter (configurable, default off because of cost).

**User gate.** Required for any rename. Annotations can be auto-applied if the user opted in.

**Failure modes.** False positives on intentional aliases (e.g., a character has nicknames). The deterministic linter uses the entity-aliases table; the LLM is given the alias list explicitly.

### 4.6 Copyeditor Agent (`copyeditor`)

**Purpose.** Mechanical and stylistic micro-fixes: punctuation, capitalisation, double spaces, em-dash style, comma splices, quote-mark consistency. Never rewords prose.

**Inputs.** `{ "scene_text": string, "style_book": StyleBook }`. `StyleBook` carries the project's choices: en-dash vs em-dash, Oxford comma yes/no, smart quotes yes/no, curly apostrophe, etc.

**Outputs.** `CopyeditProposals`:

```json
{
  "edits": [
    {
      "range_from": 0,
      "range_to": 0,
      "before": "string",
      "after": "string",
      "category": "punctuation" | "spacing" | "casing" | "quotes" | "dashes" | "spelling" | "other",
      "rationale": "string (≤30 words)"
    }
  ],
  "summary": "string (≤80 words)"
}
```

**Validation.** Every range valid; every `before` matches the actual text at that range; no edit alters word count by >10%; `after` differs from `before`; category in enum.

**Model preference.** 7B+ instruct in JSON mode. Llama 3.1 8B preferred for English; Qwen 2.5 7B for non-English projects. Smaller models (3B) are acceptable but with a "low confidence" warning.

**Context budget.** Scene text ≤ 8,000 tokens; output ≤ 4,000 tokens.

**When to run.** On demand from "Copyedit this scene/chapter" command. Scope: scene by default; chapter or project on explicit request (with budget warnings).

**User gate.** Required. The UI shows each edit as an inline diff the user can accept/reject individually or bulk-accept by category.

**Failure modes.** Range mismatch when the model fabricates positions. Rejected at validation; retried once. Persistent mismatch surfaces as a `proposal_invalid` artifact for inspection.

### 4.7 Memory Curator Agent (`memory-curator`)

**Purpose.** Maintain `book_memory`, `chapter_memory`, and `entity_memory` (per `MEMORY_SYSTEM.md`). Refresh chapter summaries on finalise; suggest new entity cards.

**Inputs.** A `MemoryRefreshScope`: `{ "scope": "book" | "chapter" | "entity", "node_id": "ULID?" }` plus the chapter text(s) and current memory state for the scope.

**Outputs.** `MemoryRefreshProposals`:

```json
{
  "book_memory_proposals": { /* Partial book_memory fields with rationales */ },
  "chapter_memory_proposals": [
    {
      "node_id": "ULID",
      "fields": { "one_line_summary": "string", "paragraph_summary": "string", "...": "..." },
      "rationale": "string"
    }
  ],
  "entity_proposals": [
    {
      "op": "create" | "update",
      "entity_id": "ULID?",
      "kind": "string",
      "canonical_name": "string",
      "fields": { /* per-kind fields */ },
      "evidence_refs": ["string", "..."]
    }
  ]
}
```

**Validation.** Schema valid; every `node_id` references an existing chapter; every entity proposal includes evidence (the chapter passage where the entity was mentioned); summaries within length budgets per `MEMORY_SYSTEM.md §4`.

**Model preference.** Llama 3.1 8B (long-context-friendly) for English; Qwen 2.5 7B for non-English. Per-chapter scope ≤ 24,000 tokens; book scope chunks via batch.

**Context budget.** Chapter scope: chapter text + current chapter_memory + relevant entity cards ≤ 24,000 tokens. Book scope: high-level summaries only, batched.

**When to run.**

- Automatic on chapter finalise (user gate required).
- On demand from "Refresh memory for this chapter" or "Refresh book memory" commands.
- After accepted Continuity Agent rename proposals (to update entity_memory).

**User gate.** Required. The Memory tab shows each proposal as an inline diff against the current memory; user accepts/rejects per field or per entity.

**Failure modes.** Invented entity references. The `EntitySanityCheck` cross-cutting validator catches proper nouns not in the manuscript text; surfaced as warnings. Long chapters that exceed context — the orchestrator batches by scene.

**Memory writes.** `book_memory`, `chapter_memory`, `entity_memory` (within scope). Pre-edit snapshot mandatory before any accepted write.

### 4.8 Vocabulary Dictionary Agent (`vocab-dictionary`)

**Purpose.** Maintain the project-layer vocabulary dictionary (per `VOCABULARY_DICTIONARIES.md §6`) by observing accepted edits and proposals.

**Inputs.** A `VocabUpdateContext`:

```json
{
  "scope": "post-edit" | "post-copyedit-batch" | "post-humanization-batch" | "chapter-finalise" | "user-demand",
  "evidence": [
    {
      "kind": "accepted_edit" | "accepted_copyedit_proposal" | "accepted_humanization_proposal" | "user_explicit",
      "before_text": "string?",
      "after_text": "string?",
      "edit_id": "ULID?",
      "user_note": "string?"
    }
  ],
  "current_dict_summary": { /* compact summary of the current project-layer dict */ }
}
```

**Outputs.** `VocabUpdateProposals` (per `VOCABULARY_DICTIONARIES.md §6`).

**Validation.** Every proposal references at least one piece of evidence; replacements have non-empty `replacement`; rationales ≤ 200 chars; no duplicate entries (term + kind already present in the dict triggers an `edit` op rather than a `create`).

**Model preference.** 7B+ instruct in JSON mode. Llama 3.1 8B preferred for English; Qwen 2.5 7B for non-English.

**Context budget.** Evidence + current dict summary ≤ 8,000 tokens; output ≤ 2,000 tokens.

**When to run.**

- Automatic after every batch of 5 accepted Copyeditor / Humanization proposals.
- Automatic on chapter finalise.
- On demand from the Vocabulary tab.

**User gate.** Required. The Vocabulary tab shows pending proposals; user accepts/rejects each.

**Failure modes.** Surfacing too many proposals (noise). Mitigated by the cooldown: at most 10 proposals per run, deduplicated against the current dict.

**Memory writes.** `vocab_entries`, `vocab_updates`, and observed patterns into `style_memory.repeated_phrases_json` / `style_memory.overused_constructions_json` (proposal — also user-gated).

### 4.9 Humanization Agent (`humanization`)

**Purpose.** Surface passages that read as robotic / GenAI prose and propose human alternatives, using the merged vocab dictionaries plus style memory.

**Inputs.** A `HumanizationScope`:

```json
{
  "scope": "scene" | "chapter",
  "node_id": "ULID",
  "text": "string",
  "merged_vocab_decisions": [ /* the lookups for terms appearing in the text */ ],
  "style_memory": { /* relevant fields */ }
}
```

**Outputs.** `HumanizationProposals` (per `VOCABULARY_DICTIONARIES.md §7`).

**Validation.** Schema valid; ranges valid against `text`; no proposal alters word count by >25%; `category` in enum; if `vocab_entry_id` present, it points to a real entry; no proposal "rewrites" beyond the rationale category (e.g., a `category: ai-tell` proposal cannot also restructure unrelated sentences).

**Model preference.** 7B+ instruct. Llama 3.1 8B preferred for English; Qwen 2.5 7B for non-English. Smaller models acceptable with a "low confidence" warning.

**Context budget.** Scene text + merged vocab decisions + style memory ≤ 12,000 tokens; output ≤ 4,000 tokens.

**When to run.** On demand from "Humanize this scene/chapter" command. Off by default for project scope (large token cost).

**User gate.** Required. The UI shows each proposal as an inline diff with the rationale citing the vocab entry that triggered it. User accepts/rejects individually or by category.

**Failure modes.** Voice-flattening — when proposals strip distinctive author voice. Mitigated by: (a) the `style_memory.preferred_tone` is part of context; (b) the user can mark a proposal "this is my voice, leave it" which adds a `vocab_entries` entry of `kind = prefer` for the project-layer dict, so future runs don't re-surface it.

**Memory writes.** None directly. Proposals reinforce vocab entries (via the Vocabulary Dictionary Agent's confidence loop) once accepted.

### 4.10 Orchestrator (controller — not an LLM agent)

The Orchestrator is the runtime that runs workflows. It is fully specified in `ARCHITECTURE.md §6`. It does not call an LLM and does not have a prompt; it composes the other agents under hard caps and approval gates.

Listed in the catalog for completeness because the user's brief explicitly listed it.

### 4.11 Final Review Editor Agent (`final-review-editor`)

**Purpose.** Polish prose to a publishable, world-class standard while preserving the author's voice and established facts. Runs **last** in the publish pipeline — after copyedit / continuity / humanization / dev-edit are settled. Distinct from the Copyeditor (mechanical) and the Humanization agent (anti-AI-tells): this is the qualitative "is this prose good?" pass.

**Inputs.** `{ scene_text, style_book_json, vocab_json, memory_excerpt, genre, audience }`.

**Outputs.** `FinalReviewOutput`:

```json
{
  "revised_text": "string — the polished prose",
  "changes": [
    { "kind": "rewrite|tighten|reorder|word_swap|cut",
      "before": "string", "after": "string", "rationale": "string" }
  ],
  "summary": "string (≤120 words)",
  "confidence": "high|medium|low",
  "warnings": ["string", "..."]
}
```

**Validation.** No fact invention against `memory_excerpt`; cross-cutting `EntitySanityCheck` catches drifted proper nouns; `confidence` in enum.

**Model preference.** `qwen3.6:latest` pinned (36B MoE, Q4) — the highest-quality local option. Pin is advisory; if the model isn't pulled the orchestrator falls back to the largest official model that fits RAM and emits a `low_confidence` warning.

**Context budget.** Scene/chapter prose + style/vocab/memory ≤ 24,000 tokens; output ≤ 8,000 tokens.

**When to run.** On demand. Heavy — the writer opts in per session.

**User gate.** Required. The diff is shown change-by-change with rationale.

**Failure modes.** Voice drift, fact invention (terminal), pinned model not pulled (terminal). The first two trigger one retry with the offending evidence appended; the third surfaces a setup hint.

### 4.12 Proposal Validator Agent (`proposal-validator`) — *internal, not user-visible*

**Purpose.** 360° review of another agent's proposal before it surfaces to the user. Runs in two tiers; Tier 1 is deterministic (always on), Tier 2 is LLM-backed (opt-in per project).

**Why it's internal.** This agent is not selectable from a workflow UI. It is an orchestrator-grade verifier: the orchestrator dispatches it after every primary agent's parse succeeds, and routes its `verdict ∈ {pass, warn, block}` into the retry / surface decision. It lives in `booksforge-agents/src/proposal_validator.rs` and is **excluded from `mvp_agents()`** so the user-visible catalog stays at ten.

**Inputs.** `ProposalValidationInput`:

```json
{
  "primary_agent_id": "string",
  "primary_output":   "any (the proposal under review)",
  "context_excerpt":  "string (relevant slice of what the primary saw)",
  "tier_1_findings":  "ProposalValidation (the deterministic pass)"
}
```

**Outputs.** `ProposalValidation`:

```json
{
  "verdict": "pass|warn|block",
  "checks": [
    { "axis":        "schema|contract|range|redaction|length|entity_sanity|memory_scope|idempotent|faithfulness|style|coherence|self_consistency",
      "outcome":     "pass|warn|fail",
      "evidence":    "string",
      "remediation": "string (optional)" }
  ],
  "summary":   "string (≤140 words)",
  "tier_2_ran": "bool"
}
```

**Validation tiers.**

| Tier | Where it runs | Axes | Cost |
|---|---|---|---|
| **1 (deterministic)** | `booksforge-orchestrator::cross_cutting` + `proposal_validator::run_tier1` | `schema`, `redaction`, `length`, `entity_sanity`, `memory_scope`, `idempotent`, `range`, `contract` | Free; always on |
| **2 (LLM)** | This agent's prompt template | `faithfulness`, `style`, `coherence`, `self_consistency` | One model call; opt-in |

**Tier 1** runs every cross-cutting validator declared on the primary agent's `AgentSpec.validators` slice. **Tier 2** runs only after Tier 1 passes and only when the project has the `validators.tier_2_enabled` flag on. Verdict aggregation is conservative: any `Fail` → `Block`; any `Warn` (and no `Fail`) → `Warn`; otherwise `Pass`. A `Block` triggers exactly one retry of the *primary* agent with the validator's evidence appended as a reminder; a second `Block` aborts the run with `proposal_invalid`.

**Model preference.** Small/fast (3B+ acceptable). The validator runs frequently, so cheap-model bias is correct.

**Context budget.** ≤ 6,000 tokens input; ≤ 1,500 output. Larger primary outputs are excerpted.

**When to run.** Automatic — orchestrator-driven, not user-initiated.

**User gate.** Not required (it gates other agents' user gates).

**Failure modes.** `verdict-out-of-enum`, `axis-out-of-enum`, `no-evidence`, `loops-back-to-self` (terminal — the validator hallucinates that it itself is wrong). On terminal failure the orchestrator falls back to Tier-1-only and surfaces a warning.

**Memory writes.** None.

## 5. Context selection

Every agent run is preceded by a context-build step. The orchestrator's `ContextBuilder`:

1. Starts from the agent's required inputs.
2. Adds optional inputs in priority order (entity cards, prior scene, style book, plugin overlays — when plugins exist).
3. Estimates token count using the model's tokenizer or a conservative byte-based fallback (`bytes / 3.5`).
4. If over budget, drops lowest-priority items first and re-estimates.
5. If still over budget, falls back to per-scope chunking (e.g., the dev-editor runs per scene rather than per chapter).
6. Records the final context bundle hash on the `agent_tasks` row.

The user can preview the assembled context in the UI before running an on-demand agent. Sending equals previewing — there are no hidden additions. (This is the same invariant as `08-ai-integration §6`.)

## 5.1 Voice fingerprint — the project's prose voice

Every project carries a `VoiceFingerprint` (in `booksforge-domain::voice`) that captures how the author writes — independent of *what* they write. It is recomputed on each chapter finalise from the corpus of accepted prose. Six structural signals:

| Signal | What it captures | LLM bias |
|---|---|---|
| `sentence_words_mean` / `_stddev` | Sentence-length cadence | LLMs prefer uniform mid-length sentences |
| `em_dash_per_1000` | Em-dash density | LLMs over-use them (3–5× human rate) |
| `ly_adverb_per_1000` | `-ly` adverb density | LLMs lean on adverbs |
| `ai_tell_triad_per_1000` | `delve` / `tapestry` / `intricate` rate | Spike-on-LLM signal; should be near-zero in human prose |
| `discourse_marker_per_1000` | `indeed` / `moreover` / `furthermore` / `thus` rate | LLMs love these |
| `type_token_ratio` | Vocabulary richness | LLM prose clusters at 0.35–0.42; human voice 0.45+ |

The fingerprint is rendered into every prose-emitting agent's prompt via the orchestrator's **prompt-guard** layer (see §6.5) so the agent matches the project's voice rather than a model-default register.

## 6. Failure modes and validation

Every agent run goes through the same lifecycle:

1. **Schema validation** of the raw output against the agent's typed Rust output (deserialization-as-validation).
2. **Semantic validators** declared on the type itself (e.g. `CopyeditProposals::validate(source_text)` enforces `before`-text-matches-source-at-range, ≤10 % word-count change, no-overlap; `ContinuityReport::validate()` enforces ULID node_ids and rename-target completeness).
3. **Cross-cutting validators** applied via the agent's `AgentSpec.validators` slice (live in `booksforge-orchestrator::cross_cutting`, dispatched by `proposal_validator::run_tier1`):
   - `Schema`        — output parses as JSON object.
   - `Redaction`     — output does not contain anything that looks like a system prompt or chain-of-thought leak (suspicious-phrase scan).
   - `Length`        — output not absurdly long or empty (4 B ≤ size ≤ 64 KiB).
   - `EntitySanity`  — proper nouns in output prose fields are in the entity bible plus an allowlist (added for prose-emitting agents only).
   - `MemoryScope`   — proposed memory writes are within `allowed_write_scopes(agent_id)` (added for memory-touching agents).
4. **Proposal Validator (Tier 2)**, if enabled — see §4.12. LLM-backed review for `faithfulness`, `style`, `coherence`, `self_consistency`. Runs only after Tier 1 passes.

If any tier returns `Block`, the orchestrator retries up to 2 times with an appended reminder ("Output must strictly conform to the schema. Previous output was rejected because: …"). After two retries, the run is marked `proposal_invalid` and the raw output is preserved as an inspectable artifact under `agent_runs/<run_id>/<task_id>.json`. **Failed proposals are never silently retried beyond the cap and are never partially applied.**

## 6.5 Cross-verification council and prompt-guard layer

Two systems sit between the primary agent's parse and the user gate, both orchestrator-mediated (agents remain stateless prompt-in/schema-out units; they never call each other).

### 6.5.1 Cross-verification council

Per `booksforge-domain::council` and `booksforge-orchestrator::council`. After Tier-1 + (opt-in) Tier-2 ProposalValidator pass, the council can dispatch **peer reviewers** — other MVP agents reviewing the primary's proposal from their own perspective. Each pairing carries a `PeerReviewFocus` (`fact_fidelity` / `voice_preservation` / `ai_tell_residue` / `name_pov_preservation` / `structural_purpose` / `memory_consistency` / `emotional_clarity`) and a `default_on` flag.

| Primary agent | Default-on reviewers | Opt-in reviewers (high-confidence mode) |
|---|---|---|
| `intake` | — | — |
| `outline-architect` | — | `memory-curator` (fact_fidelity) |
| `chapter-drafter` | `memory-curator` (memory_consistency), `continuity` (name_pov_preservation), `humanization` (ai_tell_residue) | `dev-editor` (structural_purpose) |
| `dev-editor` | — | `memory-curator` (memory_consistency) |
| `continuity` | `memory-curator` (memory_consistency) | — |
| `copyeditor` | `continuity` (name_pov_preservation) | — |
| `humanization` | `memory-curator` (fact_fidelity) | `final-review-editor` (voice_preservation) |
| `final-review-editor` | `humanization` (ai_tell_residue), `memory-curator` (fact_fidelity) | `continuity` (name_pov_preservation) |
| `memory-curator`, `vocab-dictionary` | — (Tier-1 + MemoryScope check is enough) | — |

**Bounds.** Peer reviews count toward the workflow's ≤ 8-call cap. The council is non-recursive: a reviewer cannot trigger its own peer reviewers. The verdict aggregator is conservative: any `Block` from any source escalates the council verdict to `Block`; any `Warn` (no `Block`) → `Warn`; otherwise `Pass`. A `Block` triggers exactly one retry of the *primary* with the council's evidence appended. The full audit trail (`agent_runs` + `agent_tasks` rows tied via `parent_task_id` in `caps_json`) makes every cross-verification traceable.

The aggregated `VerificationReport` (Tier-1 + optional Tier-2 + peer reviews + final verdict) travels with the proposal to the user gate so the writer sees who validated what.

### 6.5.2 Prompt-guard layer

Per `booksforge-orchestrator::prompt_guard`. Every prose-emitting agent's rendered prompt is appended with an **always-observe constraints** block composed of three parts:

1. **Humanity & empathy** (static) — six rules covering specific-over-abstract language, sensory grounding, behavioural interiority, subtext over explanation, legible stakes, and "empathy is witnessing, not narrating."
2. **Voice fingerprint** (per-project) — concrete cadence / em-dash / discourse-marker / triad-avoidance targets derived from the project's `VoiceFingerprint` (§5.1). When the fingerprint isn't yet established (corpus < 2 000 tokens), generic anti-uniform-cadence advice is used instead.
3. **Avoid-rules** (per-project) — the active-layer vocabulary `avoid` and `replace` entries, formatted as a numbered watch-list with rationale.

This guard is *additive*: the agent's primary template owns the task; the guard adds project-wide humanity / voice / vocabulary constraints uniformly.

## 7. Workflows (MVP)

Workflows compose agents. They are hard-coded. The Orchestrator's caps (`≤8 agent calls per run`, `≤10 minutes`, `≤200k tokens`, `≤3 retries per step`) apply **per workflow run**. To stay within budget when scope spans many chapters or scenes, workflows that iterate over multiple nodes are executed as **batch jobs** — the Orchestrator dispatches **one run per node** rather than one giant run.

A batch job has its own cap (`≤32 sub-runs per batch by default`, configurable up to 200 with a confirmation prompt) and is itself a tracked entity (`agent_runs.workflow_id` records the batch id; each sub-run is its own `agent_runs` row with a `parent_batch_id` set in `caps_json`). Batch jobs are cancellable as a whole and report aggregate progress.

### 7.1 `IntakeAndOutline` (single-run workflow)

```
1. ProjectIntakeAgent (input: RawIdea) → ProjectBrief
2. user_gate
3. OutlineArchitectAgent (input: ProjectBrief) → OutlineProposal
4. user_gate
5. on_accept → orchestrator creates document tree and entity stubs
```

Total: 2 agent calls in the run; well under cap.

### 7.2 `DraftScene` (single-run workflow)

```
1. ChapterDraftingAgent (input: SceneContext) → SceneDraftProposal
2. user_gate
3. on_accept → orchestrator places draft in the scene's working buffer (not main content) — user must explicitly merge
```

Total: 1 agent call.

### 7.3 `DevelopmentalReview` (batch-of-runs)

Scope: chapter | project. When scope is `chapter`, this is a single run. When scope is `project`, the orchestrator dispatches **one run per chapter** as a batch:

```
batch:
  for each chapter in scope.chapters:
    sub_run:
      1. DevelopmentalEditorAgent (input: ChapterContext) → DevelopmentalNotes
      2. notes persisted as project review (not applied to manuscript)
on batch completion:
  user_gate (review aggregate)
  user can convert any note into a structured TODO that becomes a scene-level note
```

Each sub-run has its own ≤8-call cap (a chapter long enough to require chunked context can use up to 8 internal calls — see `AGENTS.md §5` on context fallback). The batch has its own cap. The user can cancel the batch at any time; completed sub-runs' outputs are preserved.

### 7.4 `ContinuityCheck` (single-run with internal chunking)

```
1. Deterministic linter runs (Rust, no LLM) → ContinuityFinding[]
2. ContinuityAgent adjudicates ambiguous findings → ContinuityReport
   (internally batched — adjudicates findings in groups of ≤10 to fit context)
3. user_gate
4. on_accept (per finding) → orchestrator applies rename/annotate; pre-edit snapshot taken
```

The internal batching of ambiguous findings counts toward the 8-call cap; if more than 80 ambiguous findings exist, the run fails fast with a `too_many_findings` error and the user is offered "split by part" or "review deterministic findings only."

### 7.5 `Copyedit` (batch-of-runs)

Scope: scene | chapter | project.

- `scene`: single run (1 call).
- `chapter`: batch — one sub-run per scene.
- `project`: batch — one sub-run per scene; default cap is 32 sub-runs and the user is prompted before exceeding it.

```
sub_run:
  1. CopyeditorAgent (input: SceneText + StyleBook) → CopyeditProposals
on batch completion:
  user_gate (per edit, with "accept all by category" affordance)
  on_accept → orchestrator applies edits; pre-edit snapshot taken
```

### 7.6 Cap summary

| Workflow | Shape | Calls per run | Calls per batch (project scope) |
|----------|-------|---------------|---------------------------------|
| `IntakeAndOutline` | single | 2 | n/a |
| `DraftScene` | single | 1 | n/a |
| `DevelopmentalReview` | batch | ≤8 | ≤32 sub-runs (default; ≤200 with confirmation) |
| `ContinuityCheck` | single (chunked) | ≤8 (≤80 findings) | n/a |
| `Copyedit` | batch | ≤8 | ≤32 sub-runs (default; ≤200 with confirmation) |

The hard ceiling on a single workflow **run** is invariant: 8 calls, 10 minutes, 200k tokens, 3 retries per step. Batches multiply the ceiling but only along the scope dimension and only with explicit user awareness.

## 8. Telemetry and audit

Every agent run records:

- `run_id`, `workflow_id`, `agent_id`, `prompt_template_id`, `prompt_template_hash`.
- Ollama version + model digest (from `/api/show`).
- Input bundle hash, output bundle hash.
- Token counts (context, output) and duration.
- Status: `running | completed | invalid | cancelled | error`.
- Error category if applicable.

Stored in `agent_runs`, `agent_tasks`, `agent_outputs`. See `DATA_MODEL.md §5`.

## 9. UX hooks

Each agent has a corresponding UI surface (`UI_UX_SPEC.md`):

- **Project Intake** — New-project wizard; "Refine brief" in project settings.
- **Outline Architect** — Wizard step; "Regenerate outline" command.
- **Memory Curator** — Memory tab; "Refresh memory" commands; runs automatically on chapter finalise.
- **Vocabulary Dictionary** — Vocabulary tab (in Bible tab in MVP); "Pending vocab proposals" inbox.
- **Chapter Drafting** — "Draft this scene" command; off by default.
- **Developmental Editor** — "Developmental review" command; results shown in the right panel.
- **Continuity** — "Continuity check" command; results in the validators panel sidebar.
- **Copyeditor** — "Copyedit this scene/chapter" command; inline diff in the editor.
- **Humanization** — "Humanize this scene/chapter" command; inline diff with rationale citing the vocab entry that triggered.

The Agent Activity panel shows the live run with progress, current step, current context summary, and a Cancel button. Run history is browsable per project.

## 10. V1.0 agents (specified in summary)

These are not built in MVP but the specs need to be aligned to avoid surprises later. Full specs land when their phase begins.

- **Book Strategy Agent (`book-strategy`).** Inputs: brief + outline + sample chapter. Output: audience analysis, comp titles (named explicitly to avoid hallucination), positioning notes, and risks. User-gated; informs marketing copy in V1.5.
- **Research Organizer Agent (`research-organizer`).** Inputs: imported notes/PDF text. Output: tagged summaries + suggested entity cards (concepts, claims, sources, definitions, frameworks). Use case: a non-fiction author imports research; the agent extracts claims and a topic map. Memory writes: entity_memory (concept/claim/source kinds), via user-gated proposals.
- **Chapter Planning Agent (`chapter-planner`).** Inputs: outline (one chapter), book_memory, relevant entity_memory, research notes (V1.0 only). Output: a per-chapter scene plan (3–6 scenes with goal, conflict, outcome for fiction; or argument structure for nonfiction). Sits between Outline Architect (high-level) and Chapter Drafter (per-scene).
- **Line Editor Agent (`line-editor`).** Inputs: passage, style memory, voice samples (a few accepted user passages). Output: passage-level rewrite proposals with rationales. Different from Copyeditor: it can rephrase. Different from the Sharpen preset: it considers the surrounding passages and the project tone.
- **Style Guide Agent (`style-guide`).** Inputs: passage, project style book + style memory, voice samples. Output: voice-drift findings. Read-only — never proposes rewrites; suggests questions to ask of the Line Editor or Humanization Agent.
- **Fact-Check Agent (`fact-check`).** Inputs: passage, project bibliography (CSL). Output: claims that warrant verification + which bibliography entries support or contradict them. **No internet access in V1.0** — only the project's own bibliography. Internet-augmented fact-checking is V2.0.
- **Formatting Agent (`formatting`).** Inputs: a template-vs-override conflict that the deterministic formatting engine could not resolve (e.g., user override conflicts with template constraint). Output: a recommendation with rationale. Rare — invoked only when the rule-based engine punts.
- **ePUB Export QA Agent (`epub-export-qa`).** Inputs: EPUBCheck JSON report + visual regression diff summary. Output: human-readable explanations of issues plus suggested user-friendly fixes ("Your ToC depth is 4; KDP requires ≤3. Restructure chapter 7 to have one fewer subsection."). Read-only — does not generate or modify the EPUB.
- **Final Review Agent (`final-review`).** Inputs: project state + recent agent runs + validator results + memory + style memory. Output: pre-export readiness report with go/no-go summary, top issues to fix, and a confidence rating per chapter.

## 11. What is *not* an agent

These are deterministic features, not agents — by design.

- Spell check, grammar checks, find/replace, word counts, readability scores.
- Validator engine and built-in rules.
- Export pipeline (template selection, Pandoc invocation, font embedding, store-profile rules).
- Template hot-swap.
- Entity extraction in MVP (simple regex + capitalisation heuristics; LLM-assisted extraction is V1.0 and uses the Continuity Agent for adjudication).

We will resist the urge to make every feature an agent. Hallucination risk and audit complexity make agents the wrong tool for deterministic problems.

## 12. Acceptance criteria for the agent layer (MVP)

The agent layer is acceptable when:

1. All **nine** MVP agents run successfully on a clean install with `qwen2.5:7b-instruct-q4_K_M` and produce schema-valid outputs on the test fixtures in `booksforge-test-fixtures/agent-fixtures/`.
2. The orchestrator's caps (8 calls, 10 min, 200k tokens, 3 retries) are demonstrably enforced — there is a property test that throws workflows at it and confirms it terminates within the budget.
3. No agent applies a manuscript or memory change without a recorded `pre_agent_edit` snapshot existing in the snapshots table within 10 ms before the edit.
4. The UI shows every agent's context bundle before send; the bundle the model receives equals the previewed bundle byte-for-byte.
5. A randomised "evil model" mock that produces malformed output is contained: the orchestrator surfaces a `proposal_invalid` artifact and never crashes, never applies a partial edit, and never loops.
6. With Ollama killed mid-run, the run is recorded as `external_error`, the partial outputs are preserved as inspectable artifacts, and the user is offered "retry when Ollama is back."
7. **Memory writes are scope-checked** — an agent attempting to write outside its declared memory scope is rejected by the orchestrator with a typed error. Tested with a deliberately mis-configured fixture agent.
8. **The Vocabulary Dictionary Agent's `vocab_updates` ledger is reversible** — a property test asserts any sequence of accepted vocab updates is restorable byte-for-byte through the ledger.
9. **The Humanization Agent's proposals respect voice** — when the user marks a proposal "this is my voice", the orchestrator records a `prefer` entry in the project-layer dict and the Humanization Agent's next run does not re-surface the same construction.
