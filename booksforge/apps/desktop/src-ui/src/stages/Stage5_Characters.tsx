/**
 * Stage 5 — Bibles (Phase B Step 3).
 *
 * Two tabs:
 *   - Characters: array of CharacterCard, each editable inline.
 *   - World: locations + history + sensory palette + lists.
 *
 * Wires the existing `bibles_load` / `bibles_save` IPC. The book
 * pipeline (Stage 8) auto-skips its bible stages when entries exist
 * here — that's the whole point of letting writers hand-author them.
 *
 * Save semantics: each tab saves independently. Empty arrays leave
 * that half of memory untouched (`null` in BiblesSaveInput); supplied
 * arrays are treated as the authoritative list (anything not in the
 * new list is removed).
 */
import { useEffect, useState } from "react";
import type { OpenProjectResult } from "@booksforge/shared-types";
import { ipc } from "../lib/ipc";
import { errorMessage } from "../lib/errorMessage";
import {
  AxisBar,
  AXIS_FLOOR,
  COMPOSITE_THRESHOLD,
  type AxisLike,
} from "../components/AxisBar";
import { ScoreSummary, FindingsList } from "../components/ScorePanel";

interface Props {
  project:    OpenProjectResult;
  onChanged?: () => void;
}

// ── Character-critic types (Phase C — Stage 3 quality gate) ────────────────
// Mirrors `booksforge_domain::character_score::*`. Hand-typed because
// the domain crate doesn't derive ts-rs; the agent payload arrives as
// JSON inside `AgentRunResultDto.proposal_json` and the UI parses it.
interface CharacterScoreDto {
  character:             string;
  depth:                 AxisLike;
  consistency:           AxisLike;
  uniqueness:            AxisLike;
  narrative_usefulness:  AxisLike;
  emotional_impact:      AxisLike;
  overall_note?:         string;
}
interface CrossCardFindingDto {
  kind:     string;   // "duplicate_name" | "dangling_relationship" | …
  message:  string;
  severity: "error" | "warning" | string;
}
interface CharacterEditDto {
  character:    string;
  field:        string;    // CharacterCard field name
  suggestion:   string;
  replacement?: string;
}
interface CharacterCriticProposal {
  scores:              CharacterScoreDto[];
  cross_card_findings: CrossCardFindingDto[];
  edits:               CharacterEditDto[];
  overall_summary?:    string;
}

type CritState =
  | { kind: "idle" }
  | { kind: "running"; startedAt: number }
  | { kind: "ready";   proposal: CharacterCriticProposal }
  | { kind: "error";   message: string };

function cardComposite(c: CharacterScoreDto): number {
  return (
    c.depth.score
    + c.consistency.score
    + c.uniqueness.score
    + c.narrative_usefulness.score
    + c.emotional_impact.score
  ) / 5;
}
function cardPasses(c: CharacterScoreDto): boolean {
  return cardComposite(c) >= COMPOSITE_THRESHOLD
    && c.depth.score                >= AXIS_FLOOR
    && c.consistency.score          >= AXIS_FLOOR
    && c.uniqueness.score           >= AXIS_FLOOR
    && c.narrative_usefulness.score >= AXIS_FLOOR
    && c.emotional_impact.score     >= AXIS_FLOOR;
}
function bibleComposite(p: CharacterCriticProposal): number {
  if (p.scores.length === 0) return 0;
  return p.scores.reduce((acc, c) => acc + cardComposite(c), 0) / p.scores.length;
}
function biblePasses(p: CharacterCriticProposal): boolean {
  if (p.scores.length === 0) return false;
  const allCards = p.scores.every(cardPasses);
  const noErrors = p.cross_card_findings.every((f) => f.severity !== "error");
  return allCards && noErrors;
}

// ── Local types matching the Rust schema ────────────────────────────────────

interface CharacterCard {
  name:                      string;
  role:                      string;
  external_objective:        string;
  internal_need:             string;
  fear_or_wound:             string;
  secret_or_contradiction:   string;
  voice_traits:              string[];
  relationships:             { to: string; nature: string }[];
  chapter_arc:               string[];
  emotional_turning_points:  string[];
}

interface WorldLocation {
  name:              string;
  purpose_in_story:  string;
  sensory_signature: string;
  key_constraints:   string;
}

interface SensoryPalette {
  sight: string; sound: string; smell: string; touch: string; taste: string;
}

interface WorldBible {
  main_locations:         WorldLocation[];
  social_rules:           string[];
  history:                string;
  sensory_palette:        SensoryPalette;
  conflict_sources:       string[];
  symbolic_motifs:        string[];
  continuity_constraints: string[];
}

const EMPTY_CHARACTER: CharacterCard = {
  name: "", role: "supporting",
  external_objective: "", internal_need: "",
  fear_or_wound: "", secret_or_contradiction: "",
  voice_traits: [], relationships: [],
  chapter_arc: [], emotional_turning_points: [],
};

const EMPTY_WORLD: WorldBible = {
  main_locations: [], social_rules: [], history: "",
  sensory_palette: { sight: "", sound: "", smell: "", touch: "", taste: "" },
  conflict_sources: [], symbolic_motifs: [], continuity_constraints: [],
};

const ROLES = ["protagonist", "antagonist", "mentor", "foil", "ally", "supporting"];

// ── Component ──────────────────────────────────────────────────────────────

export default function Stage5_Bibles({ project, onChanged }: Props) {
  const [tab,           setTab]           = useState<"characters" | "world">("characters");
  const [characters,    setCharacters]    = useState<CharacterCard[]>([]);
  const [world,         setWorld]         = useState<WorldBible>(EMPTY_WORLD);
  const [loading,       setLoading]       = useState(true);
  const [saving,        setSaving]        = useState(false);
  const [error,         setError]         = useState<string | null>(null);
  const [savedHint,     setSavedHint]     = useState<string | null>(null);
  const [critState,     setCritState]     = useState<CritState>({ kind: "idle" });

  useEffect(() => {
    ipc.biblesLoad()
      .then((r) => {
        const cs = (r.characters as Partial<CharacterCard>[]).map((c) => ({
          ...EMPTY_CHARACTER, ...c,
          voice_traits:             c.voice_traits             ?? [],
          relationships:            c.relationships            ?? [],
          chapter_arc:              c.chapter_arc              ?? [],
          emotional_turning_points: c.emotional_turning_points ?? [],
        }));
        setCharacters(cs);
        if (r.world && typeof r.world === "object") {
          const w = r.world as Partial<WorldBible>;
          setWorld({
            ...EMPTY_WORLD, ...w,
            main_locations:         w.main_locations  ?? [],
            social_rules:           w.social_rules    ?? [],
            sensory_palette:        { ...EMPTY_WORLD.sensory_palette, ...(w.sensory_palette ?? {}) },
            conflict_sources:       w.conflict_sources       ?? [],
            symbolic_motifs:        w.symbolic_motifs        ?? [],
            continuity_constraints: w.continuity_constraints ?? [],
          });
        }
      })
      .catch((e) => setError(errorMessage(e)))
      .finally(() => setLoading(false));
  }, []);

  async function saveCharacters() {
    setSaving(true); setError(null); setSavedHint(null);
    try {
      await ipc.biblesSave({ characters });
      setSavedHint(`Saved ${characters.length} character${characters.length === 1 ? "" : "s"}. Stage 8 will skip its bible stage on the next run.`);
      onChanged?.();
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setSaving(false);
    }
  }

  /**
   * Phase C — Stage 3 quality gate. Saves the current characters,
   * then runs the `character-critic` agent against the persisted bible.
   * The agent scores each card on five axes + emits cross-card
   * findings + per-card edits. Auto-resolves to Medium tier.
   */
  async function handleScoreCharacters() {
    if (characters.length === 0) {
      setError("Add at least one character before scoring.");
      return;
    }
    // Persist so the agent reads what's on screen.
    try {
      await ipc.biblesSave({ characters });
    } catch (e) {
      setError(errorMessage(e));
      return;
    }
    setError(null); setSavedHint(null);
    setCritState({ kind: "running", startedAt: Date.now() });
    try {
      const r = await ipc.agentRunCharacterCritic({
        project_id: project.project_id,
        model:      null,  // auto-resolve to Medium
      });
      if (r.status !== "completed" || !r.proposal_json) {
        setCritState({
          kind: "error",
          message: r.error ?? `Agent returned status: ${r.status}`,
        });
        return;
      }
      const parsed = JSON.parse(r.proposal_json) as Partial<CharacterCriticProposal>;
      const proposal: CharacterCriticProposal = {
        scores:              parsed.scores ?? [],
        cross_card_findings: parsed.cross_card_findings ?? [],
        edits:               parsed.edits ?? [],
        overall_summary:     parsed.overall_summary,
      };
      setCritState({ kind: "ready", proposal });
    } catch (e) {
      setCritState({ kind: "error", message: errorMessage(e) });
    }
  }

  /**
   * Apply a single character-critic edit to the local form. We match
   * by `character` (the CharacterCard name) and patch the named field.
   * List fields (voice_traits / chapter_arc / relationships /
   * emotional_turning_points) prepend the replacement so the writer
   * can see what changed; scalar fields overwrite.
   */
  function applyCharacterEdit(edit: CharacterEditDto) {
    const replacement = (edit.replacement ?? "").trim();
    if (!replacement) return;  // structural edit; writer applies manually
    setCharacters((prev) => prev.map((c) => {
      if (c.name !== edit.character) return c;
      switch (edit.field) {
        case "name":                    return { ...c, name:                    replacement };
        case "role":                    return { ...c, role:                    replacement };
        case "external_objective":      return { ...c, external_objective:      replacement };
        case "internal_need":           return { ...c, internal_need:           replacement };
        case "fear_or_wound":           return { ...c, fear_or_wound:           replacement };
        case "secret_or_contradiction": return { ...c, secret_or_contradiction: replacement };
        case "voice_traits":            return { ...c, voice_traits:            [replacement, ...c.voice_traits] };
        case "chapter_arc":             return { ...c, chapter_arc:             [replacement, ...c.chapter_arc] };
        case "emotional_turning_points": return { ...c, emotional_turning_points: [replacement, ...c.emotional_turning_points] };
        case "relationships": {
          // Expect "Name: nature" or just a free string; best-effort split.
          const idx = replacement.indexOf(":");
          const to     = idx >= 0 ? replacement.slice(0, idx).trim() : "";
          const nature = idx >= 0 ? replacement.slice(idx + 1).trim() : replacement;
          return { ...c, relationships: [{ to, nature }, ...c.relationships] };
        }
        default:
          return c;
      }
    }));
    setSavedHint(null);
    setError(null);
  }

  async function saveWorld() {
    setSaving(true); setError(null); setSavedHint(null);
    try {
      await ipc.biblesSave({ world });
      setSavedHint("World bible saved. Stage 8 will skip its world-bible stage on the next run.");
      onChanged?.();
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setSaving(false);
    }
  }

  // Character helpers
  function updateChar(i: number, patch: Partial<CharacterCard>) {
    setCharacters((prev) => prev.map((c, idx) => (idx === i ? { ...c, ...patch } : c)));
    setSavedHint(null);
  }
  function addChar() {
    setCharacters((prev) => [
      ...prev,
      { ...EMPTY_CHARACTER, role: prev.length === 0 ? "protagonist" : prev.length === 1 ? "antagonist" : "supporting" },
    ]);
  }
  function removeChar(i: number) {
    if (!window.confirm("Remove this character? Their notes will be lost.")) return;
    setCharacters((prev) => prev.filter((_, idx) => idx !== i));
  }

  // World helpers
  function addLocation() {
    setWorld((w) => ({ ...w, main_locations: [...w.main_locations, { name: "", purpose_in_story: "", sensory_signature: "", key_constraints: "" }] }));
    setSavedHint(null);
  }
  function updateLocation(i: number, patch: Partial<WorldLocation>) {
    setWorld((w) => ({ ...w, main_locations: w.main_locations.map((l, idx) => (idx === i ? { ...l, ...patch } : l)) }));
    setSavedHint(null);
  }
  function removeLocation(i: number) {
    setWorld((w) => ({ ...w, main_locations: w.main_locations.filter((_, idx) => idx !== i) }));
  }

  // ── Render ──────────────────────────────────────────────────────────────

  return (
    <div style={s.root}>
      <div style={s.col}>
        <header style={s.header}>
          <p style={s.stageNum}>Stage 3 of 6</p>
          <h1 style={s.title}>Bibles</h1>
          <p style={s.lede}>
            Characters and world / setting. <b>This stage is optional</b> —
            Stage 8's pipeline auto-generates bibles when these are empty.
            Fill them in if you have existing material or want full control;
            the pipeline detects what's here and skips its bible stages,
            saving 5–10 min per drafting run.
          </p>
        </header>

        <div style={s.tabBar}>
          <Tab active={tab === "characters"} onClick={() => setTab("characters")}>
            Characters{characters.length > 0 && ` · ${characters.length}`}
          </Tab>
          <Tab active={tab === "world"} onClick={() => setTab("world")}>
            World{world.main_locations.length > 0 && ` · ${world.main_locations.length} loc`}
          </Tab>
        </div>

        {loading && <p style={s.muted}>Loading bibles…</p>}

        {!loading && tab === "characters" && (
          <section style={s.section}>
            <header style={s.sectionHeader}>
              <h2 style={s.sectionTitle}>Character bible</h2>
              <p style={s.sectionHint}>
                Each card carries voice traits, internal need, chapter arc.
                The scene drafter reads every field per scene to keep
                voice and motivation consistent across the book.
              </p>
            </header>
            <div style={s.sectionBody}>
              {characters.length === 0 && (
                <p style={s.empty}>
                  No characters yet. Stage 8 will auto-generate four
                  (protagonist, antagonist, two supporting) using
                  <code style={s.code}>character-bible-chunked</code> on the
                  Light tier. To override with your own material, click
                  <b> + Add character</b>.
                </p>
              )}
              <ul style={s.cardList}>
                {characters.map((c, i) => (
                  <CharacterEditor
                    key={i}
                    index={i}
                    card={c}
                    onChange={(patch) => updateChar(i, patch)}
                    onRemove={() => removeChar(i)}
                  />
                ))}
              </ul>
              <div style={s.addRow}>
                <button style={s.addBtn} onClick={addChar}>+ Add character</button>
              </div>

              {error && <div style={s.error}>{error}</div>}
              {savedHint && <div style={s.savedHint}>{savedHint}</div>}

              <div style={s.actionsRow}>
                <button
                  style={s.ghostBtn}
                  onClick={handleScoreCharacters}
                  disabled={
                    characters.length === 0
                    || critState.kind === "running"
                    || saving
                  }
                  title={characters.length === 0
                    ? "Add at least one character first"
                    : "Run the character-critic agent (~60-120 s on Medium tier)"}
                >
                  {critState.kind === "running" ? "Scoring…" : "✨ Score with AI"}
                </button>
                <button
                  style={{ ...s.primaryBtn, ...(saving ? s.primaryBtnBusy : {}) }}
                  onClick={saveCharacters}
                  disabled={saving}
                >
                  {saving ? "Saving…" : "Save characters"}
                </button>
              </div>

              <CriticSection
                state={critState}
                onApplyEdit={applyCharacterEdit}
                onClear={() => setCritState({ kind: "idle" })}
              />
            </div>
          </section>
        )}

        {!loading && tab === "world" && (
          <section style={s.section}>
            <header style={s.sectionHeader}>
              <h2 style={s.sectionTitle}>World bible</h2>
              <p style={s.sectionHint}>
                Locations, social rules, history, sensory palette, motifs.
                Every prose stage reads these — the more grounded these are,
                the more grounded the AI's prose will be.
              </p>
            </header>
            <div style={s.sectionBody}>
              <h3 style={s.subH}>Locations</h3>
              {world.main_locations.length === 0 && (
                <p style={s.empty}>
                  No locations yet. Add at least one with a specific
                  sensory signature ("pine resin and wet stone", not
                  "outdoor smells") for the drafter to anchor scenes.
                </p>
              )}
              <ul style={s.cardList}>
                {world.main_locations.map((l, i) => (
                  <li key={i} style={s.card}>
                    <div style={s.cardHeader}>
                      <input
                        style={{ ...s.input, fontWeight: 600 }}
                        value={l.name}
                        placeholder="Location name"
                        onChange={(e) => updateLocation(i, { name: e.target.value })}
                      />
                      <button style={s.removeBtn} onClick={() => removeLocation(i)}>Remove</button>
                    </div>
                    <Field label="Purpose in story">
                      <input style={s.input} value={l.purpose_in_story}
                        onChange={(e) => updateLocation(i, { purpose_in_story: e.target.value })}
                        placeholder="What plot work this location does." />
                    </Field>
                    <Field label="Sensory signature" hint="Specific details only.">
                      <input style={s.input} value={l.sensory_signature}
                        onChange={(e) => updateLocation(i, { sensory_signature: e.target.value })}
                        placeholder="pine resin and wet stone" />
                    </Field>
                    <Field label="Key constraints" hint="Optional rules that govern action there.">
                      <input style={s.input} value={l.key_constraints}
                        onChange={(e) => updateLocation(i, { key_constraints: e.target.value })} />
                    </Field>
                  </li>
                ))}
              </ul>
              <div style={s.addRow}>
                <button style={s.addBtn} onClick={addLocation}>+ Add location</button>
              </div>

              <h3 style={s.subH}>History</h3>
              <textarea
                style={{ ...s.input, minHeight: 80, fontFamily: "var(--font-prose, serif)" }}
                value={world.history}
                onChange={(e) => { setWorld({ ...world, history: e.target.value }); setSavedHint(null); }}
                placeholder="Backstory the writer needs to know but never explicitly says (≥ 30 words for the drafter to use it)."
              />

              <h3 style={s.subH}>Sensory palette</h3>
              <div style={s.gridTwo}>
                {(["sight","sound","smell","touch","taste"] as const).map((sense) => (
                  <Field key={sense} label={sense}>
                    <input style={s.input}
                      value={world.sensory_palette[sense]}
                      onChange={(e) => {
                        setWorld({
                          ...world,
                          sensory_palette: { ...world.sensory_palette, [sense]: e.target.value },
                        });
                        setSavedHint(null);
                      }}
                      placeholder={SENSE_PLACEHOLDERS[sense]} />
                  </Field>
                ))}
              </div>

              <h3 style={s.subH}>Lists</h3>
              <Field label="Social rules" hint="One per line.">
                <ListInput
                  value={world.social_rules}
                  onChange={(v) => { setWorld({ ...world, social_rules: v }); setSavedHint(null); }}
                  placeholder={"small-town news travels by post office before phone"}
                />
              </Field>
              <Field label="Conflict sources" hint="One per line.">
                <ListInput
                  value={world.conflict_sources}
                  onChange={(v) => { setWorld({ ...world, conflict_sources: v }); setSavedHint(null); }}
                  placeholder="a hidden life she did not know about"
                />
              </Field>
              <Field label="Symbolic motifs" hint="One per line.">
                <ListInput
                  value={world.symbolic_motifs}
                  onChange={(v) => { setWorld({ ...world, symbolic_motifs: v }); setSavedHint(null); }}
                  placeholder={"the wound clock\nthe wrong-side light switch"}
                />
              </Field>
              <Field label="Continuity constraints" hint="Things the book must NOT contradict. One per line.">
                <ListInput
                  value={world.continuity_constraints}
                  onChange={(v) => { setWorld({ ...world, continuity_constraints: v }); setSavedHint(null); }}
                  placeholder={"the husband died exactly six weeks before chapter 1"}
                />
              </Field>

              {error && <div style={s.error}>{error}</div>}
              {savedHint && <div style={s.savedHint}>{savedHint}</div>}

              <div style={s.actionsRow}>
                <button
                  style={{ ...s.primaryBtn, ...(saving ? s.primaryBtnBusy : {}) }}
                  onClick={saveWorld}
                  disabled={saving}
                >
                  {saving ? "Saving…" : "Save world bible"}
                </button>
              </div>
            </div>
          </section>
        )}
      </div>
    </div>
  );
}

// ── CharacterEditor ─────────────────────────────────────────────────────────

function CharacterEditor({
  index, card, onChange, onRemove,
}: {
  index: number;
  card: CharacterCard;
  onChange: (patch: Partial<CharacterCard>) => void;
  onRemove: () => void;
}) {
  return (
    <li style={s.card}>
      <div style={s.cardHeader}>
        <input
          style={{ ...s.input, fontWeight: 600, flex: 1 }}
          value={card.name}
          placeholder={`Character ${index + 1} name`}
          onChange={(e) => onChange({ name: e.target.value })}
        />
        <select
          style={s.input}
          value={card.role}
          onChange={(e) => onChange({ role: e.target.value })}
        >
          {ROLES.map((r) => <option key={r} value={r}>{r}</option>)}
        </select>
        <button style={s.removeBtn} onClick={onRemove}>Remove</button>
      </div>
      <Field label="External objective" hint="What they're after in the world.">
        <input style={s.input} value={card.external_objective}
          onChange={(e) => onChange({ external_objective: e.target.value })}
          placeholder="Find Maeve Kowalski and ask the question." />
      </Field>
      <Field label="Internal need" hint="What they need to learn / face / release.">
        <input style={s.input} value={card.internal_need}
          onChange={(e) => onChange({ internal_need: e.target.value })}
          placeholder="To stop arranging the silence between them." />
      </Field>
      <Field label="Fear or wound">
        <input style={s.input} value={card.fear_or_wound}
          onChange={(e) => onChange({ fear_or_wound: e.target.value })} />
      </Field>
      <Field label="Secret or contradiction">
        <input style={s.input} value={card.secret_or_contradiction}
          onChange={(e) => onChange({ secret_or_contradiction: e.target.value })} />
      </Field>
      <Field label="Voice traits" hint="3–6 specific markers. Avoid 'kind' or 'bright'.">
        <ListInput
          value={card.voice_traits}
          onChange={(v) => onChange({ voice_traits: v })}
          placeholder={"sentences truncated when cornered\nuses tool-shop vocabulary by reflex"}
        />
      </Field>
      <Field label="Chapter arc" hint="What changes for this character per chapter. One entry per chapter.">
        <ListInput
          value={card.chapter_arc}
          onChange={(v) => onChange({ chapter_arc: v })}
          placeholder={"Ch1: opens the drawer, finds the letters"}
        />
      </Field>
      <Field label="Emotional turning points">
        <ListInput
          value={card.emotional_turning_points}
          onChange={(v) => onChange({ emotional_turning_points: v })}
        />
      </Field>
    </li>
  );
}

// ── Subcomponents ───────────────────────────────────────────────────────────

function Tab({ active, children, onClick }: {
  active: boolean; children: React.ReactNode; onClick: () => void;
}) {
  return (
    <button
      style={{
        ...s.tabBtn,
        borderBottom: active ? "2px solid var(--color-amber-600)" : "2px solid transparent",
        color: active ? "var(--color-neutral-900)" : "var(--color-neutral-500)",
        fontWeight: active ? 600 : 500,
      }}
      onClick={onClick}
    >
      {children}
    </button>
  );
}

function Field({ label, hint, children }: {
  label: string; hint?: string; children: React.ReactNode;
}) {
  return (
    <label style={s.field}>
      <span style={s.fieldLabel}>{label}</span>
      {children}
      {hint && <span style={s.fieldHint}>{hint}</span>}
    </label>
  );
}

function ListInput({ value, onChange, placeholder }: {
  value: string[]; onChange: (v: string[]) => void; placeholder?: string;
}) {
  return (
    <textarea
      style={{ ...s.input, minHeight: 60, fontFamily: "var(--font-prose, serif)" }}
      value={value.join("\n")}
      placeholder={placeholder}
      onChange={(e) => onChange(
        e.target.value.split("\n").map((l) => l.trim()).filter((l) => l.length > 0),
      )}
    />
  );
}

// ── CriticSection (Phase C — Stage 3 quality gate) ────────────────────────

function CriticSection({
  state, onApplyEdit, onClear,
}: {
  state:       CritState;
  onApplyEdit: (edit: CharacterEditDto) => void;
  onClear:     () => void;
}) {
  if (state.kind === "idle") {
    return (
      <section style={s.critIdle}>
        <header style={s.critIdleHeader}>
          <span style={s.critIdleTitle}>Quality gate</span>
        </header>
        <p style={s.critIdleHint}>
          Click <b>✨ Score with AI</b> above. The character-critic agent
          reads each card and scores five axes: <b>depth</b>, <b>consistency</b>,{" "}
          <b>uniqueness</b>, <b>narrative usefulness</b>, <b>emotional impact</b>.
          Gate passes when every card composite ≥ {COMPOSITE_THRESHOLD},
          every axis ≥ {AXIS_FLOOR}, and there are zero error-severity
          cross-card findings (duplicate names, dangling relationships, …).
        </p>
      </section>
    );
  }
  if (state.kind === "running") {
    return (
      <section style={s.critIdle}>
        <header style={s.critIdleHeader}>
          <span style={s.critIdleTitle}>Scoring…</span>
        </header>
        <div style={s.critRunning}>
          <span style={s.critSpinner} aria-hidden="true" />
          <span>
            character-critic is reading your bible on the Medium tier. ~60-120 s.
          </span>
        </div>
      </section>
    );
  }
  if (state.kind === "error") {
    return (
      <section style={s.critIdle}>
        <header style={s.critIdleHeader}>
          <span style={s.critIdleTitle}>Score failed</span>
        </header>
        <div style={s.critErr}>{state.message}</div>
        <div style={{ display: "flex", justifyContent: "flex-end", marginTop: 8 }}>
          <button style={s.smallBtn} onClick={onClear}>Dismiss</button>
        </div>
      </section>
    );
  }
  // Ready
  const p = state.proposal;
  const composite = bibleComposite(p);
  const passes = biblePasses(p);
  const errors = p.cross_card_findings.filter((f) => f.severity === "error");
  return (
    <section style={s.critIdle}>
      <header style={s.critIdleHeader}>
        <span style={s.critIdleTitle}>
          {passes ? "✓ Character bible passes gate" : "Character bible needs revision"}
        </span>
        <button style={s.smallBtn} onClick={onClear}>Clear</button>
      </header>
      <ScoreSummary
        composite={composite}
        passing={passes}
        stats={(
          <>
            <div><b>{p.scores.length}</b> cards scored</div>
            <div>
              <b>{p.scores.filter(cardPasses).length}</b> /{" "}
              {p.scores.length} pass per-card gate
            </div>
            <div>
              <b>{errors.length}</b> blocking finding{errors.length === 1 ? "" : "s"}
            </div>
          </>
        )}
      />

      {p.overall_summary && (
        <div style={s.overallSummary}>{p.overall_summary}</div>
      )}

      <FindingsList
        title="Cross-card findings"
        findings={p.cross_card_findings.map((f) => ({
          kind:     f.kind,
          message:  f.message,
          severity: f.severity,
        }))}
      />


      <ul style={s.cardScoreList}>
        {p.scores.map((c, i) => (
          <CardScoreBlock
            key={`${c.character}-${i}`}
            score={c}
            edits={p.edits.filter((e) => e.character === c.character)}
            onApplyEdit={onApplyEdit}
          />
        ))}
      </ul>
    </section>
  );
}

function CardScoreBlock({
  score, edits, onApplyEdit,
}: {
  score: CharacterScoreDto;
  edits: CharacterEditDto[];
  onApplyEdit: (edit: CharacterEditDto) => void;
}) {
  const composite = cardComposite(score);
  const passes = cardPasses(score);
  return (
    <li style={s.cardScore}>
      <header style={s.cardScoreHeader}>
        <div style={s.cardScoreNameWrap}>
          <span style={s.cardScoreName}>{score.character || "(unnamed)"}</span>
          <span style={{
            ...s.cardScoreBadge,
            background: passes
              ? "rgba(34,197,94,0.12)"
              : "rgba(245,158,11,0.16)",
            color: passes
              ? "var(--color-green-700, #15803d)"
              : "var(--color-amber-700, #b45309)",
            borderColor: passes
              ? "rgba(34,197,94,0.4)"
              : "rgba(245,158,11,0.5)",
          }}>
            {passes ? "PASS" : "NEEDS WORK"} · {composite.toFixed(1)}
          </span>
        </div>
      </header>
      <div style={s.cardAxisGrid}>
        {([
          ["Depth",                 score.depth],
          ["Consistency",           score.consistency],
          ["Uniqueness",            score.uniqueness],
          ["Narrative usefulness",  score.narrative_usefulness],
          ["Emotional impact",      score.emotional_impact],
        ] as Array<[string, AxisLike]>).map(([label, axis]) => (
          <AxisBar key={label} label={label} axis={axis} />
        ))}
      </div>
      {score.overall_note && (
        <div style={s.cardScoreNote}>{score.overall_note}</div>
      )}
      {edits.length > 0 && (
        <div style={s.editsBlock}>
          <h4 style={s.editsH}>Suggested edits ({edits.length})</h4>
          <ul style={s.editsList}>
            {edits.map((edit, i) => (
              <li key={i} style={s.editRow}>
                <div style={s.editLeft}>
                  <span style={s.editField}>{edit.field}</span>
                  <span style={s.editSuggestion}>{edit.suggestion}</span>
                  {edit.replacement && (
                    <span style={s.editReplacement}>
                      ↳ <em>{edit.replacement}</em>
                    </span>
                  )}
                </div>
                {edit.replacement && (
                  <button style={s.smallBtn} onClick={() => onApplyEdit(edit)}>
                    Apply
                  </button>
                )}
              </li>
            ))}
          </ul>
        </div>
      )}
    </li>
  );
}

const SENSE_PLACEHOLDERS = {
  sight: "low gray light, dust on the bench",
  sound: "the click of the wrong-side switch",
  smell: "wet wool and old oil",
  touch: "cold brass",
  taste: "tea gone cold",
};

// ── Styles ──────────────────────────────────────────────────────────────────

const s: Record<string, React.CSSProperties> = {
  root: {
    height: "100%", overflow: "auto",
    padding: "32px 24px 48px",
    display: "flex", justifyContent: "center",
    fontFamily: "var(--font-ui)",
  },
  col: { width: "min(820px, 100%)", display: "flex", flexDirection: "column", gap: 16 },
  header: { display: "flex", flexDirection: "column", gap: 4, marginBottom: 8 },
  stageNum: {
    margin: 0, fontSize: 11, fontWeight: 600,
    textTransform: "uppercase", letterSpacing: "0.1em",
    color: "var(--color-amber-600)",
  },
  title: {
    margin: 0, fontFamily: "var(--font-prose, serif)",
    fontSize: 32, fontWeight: 700, lineHeight: 1.2,
    color: "var(--color-neutral-900)",
  },
  lede: { margin: "4px 0 0", fontSize: 14, color: "var(--color-neutral-700)", lineHeight: 1.6 },
  muted: { color: "var(--color-neutral-500)", fontSize: 13, margin: 0 },
  tabBar: {
    display: "flex", gap: 4,
    borderBottom: "1px solid var(--color-neutral-200)",
  },
  tabBtn: {
    padding: "10px 16px",
    background: "none", border: "none", borderBottom: "2px solid transparent",
    cursor: "pointer", fontSize: 13, fontFamily: "inherit",
  },
  section: {
    background: "#fff",
    border: "1px solid var(--color-neutral-200)",
    borderRadius: 6,
    overflow: "hidden",
    // flex-shrink:0 — parent `col` is a flex column; without this
    // sections compress and the overflow:hidden clips the body.
    flexShrink: 0,
  },
  sectionHeader: {
    padding: "12px 16px",
    background: "var(--color-neutral-50)",
    borderBottom: "1px solid var(--color-neutral-200)",
  },
  sectionTitle: {
    margin: 0, fontSize: 15, fontWeight: 600,
    color: "var(--color-neutral-900)",
  },
  sectionHint: {
    margin: "4px 0 0", fontSize: 12,
    color: "var(--color-neutral-600)", lineHeight: 1.5,
  },
  sectionBody: { padding: 16, display: "flex", flexDirection: "column", gap: 12 },
  subH: {
    margin: "var(--space-2, 8px) 0 0",
    fontSize: 11, fontWeight: 600,
    color: "var(--color-neutral-600)",
    textTransform: "uppercase", letterSpacing: "0.06em",
  },
  empty: { fontSize: 12, color: "var(--color-neutral-500)", lineHeight: 1.6, margin: 0 },
  cardList: { listStyle: "none", margin: 0, padding: 0, display: "flex", flexDirection: "column", gap: 12 },
  card: {
    border: "1px solid var(--color-neutral-200)", borderRadius: 6,
    padding: 12, display: "flex", flexDirection: "column", gap: 8,
    background: "var(--color-neutral-50)",
  },
  cardHeader: { display: "flex", gap: 8, alignItems: "center" },
  field: {
    // Explicit width so the field doesn't shrink to min-content
    // inside the nested flex columns (cardList → card → field).
    display: "flex", flexDirection: "column", gap: 4,
    width: "100%",
  },
  fieldLabel: {
    fontSize: 11, fontWeight: 600,
    color: "var(--color-neutral-700)",
    textTransform: "uppercase", letterSpacing: "0.04em",
  },
  fieldHint: { fontSize: 11, color: "var(--color-neutral-500)" },
  input: {
    // Visible-by-default form control — display:block + min-height
    // prevents inline <input>/<textarea> from collapsing to zero
    // height inside a flex column. Border one step darker for clear
    // contrast against the card's white background.
    display: "block",
    width: "100%", boxSizing: "border-box",
    padding: "6px 10px",
    border: "1px solid var(--color-neutral-400, #9ca3af)",
    borderRadius: 4,
    background: "#fff", color: "var(--color-neutral-900)",
    fontFamily: "var(--font-ui)", fontSize: 13,
    lineHeight: 1.4,
    minHeight: 36,
    outline: "none",
  },
  gridTwo: { display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8 },
  addRow: { display: "flex" },
  addBtn: {
    background: "transparent",
    border: "1px dashed var(--color-neutral-300)",
    borderRadius: 4, padding: "6px 10px", cursor: "pointer",
    color: "var(--color-neutral-600)", fontSize: 12, fontFamily: "var(--font-ui)",
  },
  removeBtn: {
    background: "transparent", color: "var(--color-neutral-500)",
    border: "1px solid var(--color-neutral-300)", borderRadius: 4,
    fontSize: 11, padding: "4px 8px", cursor: "pointer",
    fontFamily: "var(--font-ui)",
  },
  error: {
    padding: "8px 12px",
    background: "rgba(220,38,38,0.06)",
    color: "var(--color-red-700, #b91c1c)",
    border: "1px solid rgba(220,38,38,0.25)",
    borderRadius: 4, fontFamily: "var(--font-mono)", fontSize: 12,
  },
  savedHint: {
    padding: "8px 12px",
    background: "rgba(34,197,94,0.08)",
    color: "var(--color-green-700, #15803d)",
    border: "1px solid rgba(34,197,94,0.3)",
    borderRadius: 4, fontSize: 12,
  },
  actionsRow: {
    display: "flex", justifyContent: "flex-end", gap: 12, marginTop: 4,
  },
  primaryBtn: {
    padding: "10px 20px",
    background: "var(--color-amber-600)", color: "#fff",
    border: "none", borderRadius: 5,
    fontSize: 14, fontWeight: 600, cursor: "pointer",
    fontFamily: "var(--font-ui)",
  },
  primaryBtnBusy: { opacity: 0.7, cursor: "wait" },
  ghostBtn: {
    padding: "10px 16px",
    background: "transparent", color: "var(--color-neutral-700)",
    border: "1px solid var(--color-neutral-300)", borderRadius: 5,
    fontSize: 13, fontWeight: 500, cursor: "pointer",
    fontFamily: "var(--font-ui)",
  },
  code: {
    fontFamily: "var(--font-mono)", fontSize: 11,
    padding: "1px 4px",
    background: "var(--color-neutral-100)", borderRadius: 3,
  },
  // ── Character-critic Score Panel ────────────────────────────────────────
  critIdle: {
    marginTop: 12,
    padding: 12,
    background: "var(--color-neutral-50)",
    border: "1px solid var(--color-neutral-200)",
    borderRadius: 6,
    display: "flex", flexDirection: "column", gap: 10,
  },
  critIdleHeader: {
    display: "flex", justifyContent: "space-between", alignItems: "center",
  },
  critIdleTitle: {
    fontSize: 11, fontWeight: 600,
    textTransform: "uppercase", letterSpacing: "0.08em",
    color: "var(--color-neutral-700)",
  },
  critIdleHint: {
    margin: 0,
    fontSize: 12, color: "var(--color-neutral-600)", lineHeight: 1.6,
  },
  critRunning: {
    display: "flex", alignItems: "center", gap: 10,
    padding: "10px 12px",
    background: "#fff",
    border: "1px solid var(--color-neutral-200)",
    borderRadius: 4,
    fontSize: 13, color: "var(--color-neutral-700)",
  },
  critSpinner: {
    width: 14, height: 14, flexShrink: 0,
    borderRadius: "50%",
    border: "2px solid var(--color-neutral-300)",
    borderTopColor: "var(--color-amber-600)",
    animation: "bf-stage5-spin 0.9s linear infinite",
  },
  critErr: {
    padding: "8px 12px",
    background: "rgba(220,38,38,0.06)",
    color: "var(--color-red-700, #b91c1c)",
    border: "1px solid rgba(220,38,38,0.25)",
    borderRadius: 4, fontFamily: "var(--font-mono)", fontSize: 12,
  },
  critSummaryRow: {
    display: "flex", gap: 24, alignItems: "center",
    flexWrap: "wrap",
  },
  scoreSummary: {
    display: "flex", alignItems: "baseline", gap: 8,
  },
  scoreBig: {
    fontFamily: "var(--font-prose, serif)",
    fontSize: 40, fontWeight: 700, lineHeight: 1,
    fontVariantNumeric: "tabular-nums",
  },
  scoreBigDenom: {
    fontSize: 16, fontWeight: 500,
    color: "var(--color-neutral-500)",
    marginLeft: 4,
  },
  scoreBigLabel: {
    fontSize: 11, fontWeight: 600,
    textTransform: "uppercase", letterSpacing: "0.08em",
    color: "var(--color-neutral-500)",
  },
  critSummaryStats: {
    display: "flex", flexDirection: "column", gap: 2,
    fontSize: 12, color: "var(--color-neutral-700)",
  },
  overallSummary: {
    padding: "10px 14px",
    background: "#fff",
    border: "1px solid var(--color-neutral-200)",
    borderRadius: 4,
    fontSize: 13, color: "var(--color-neutral-800)", lineHeight: 1.6,
    fontFamily: "var(--font-prose, serif)",
  },
  findingsBlock: {
    display: "flex", flexDirection: "column", gap: 6,
  },
  findingsH: {
    margin: 0,
    fontSize: 11, fontWeight: 600,
    textTransform: "uppercase", letterSpacing: "0.06em",
    color: "var(--color-neutral-500)",
  },
  findingsList: {
    listStyle: "none", margin: 0, padding: 0,
    display: "flex", flexDirection: "column", gap: 4,
  },
  findingRow: {
    display: "flex", gap: 10, alignItems: "flex-start",
    padding: "6px 10px",
    borderRadius: 4,
    fontSize: 12, lineHeight: 1.5,
  },
  findingErr: {
    background: "rgba(220,38,38,0.06)",
    border: "1px solid rgba(220,38,38,0.25)",
    color: "var(--color-red-700, #b91c1c)",
  },
  findingWarn: {
    background: "rgba(245,158,11,0.08)",
    border: "1px solid rgba(245,158,11,0.3)",
    color: "var(--color-amber-700, #b45309)",
  },
  findingKind: {
    fontFamily: "var(--font-mono)", fontSize: 10,
    fontWeight: 600, textTransform: "uppercase", letterSpacing: "0.04em",
    flexShrink: 0,
  },
  cardScoreList: {
    listStyle: "none", margin: 0, padding: 0,
    display: "flex", flexDirection: "column", gap: 10,
  },
  cardScore: {
    background: "#fff",
    border: "1px solid var(--color-neutral-200)",
    borderRadius: 4,
    padding: 12,
    display: "flex", flexDirection: "column", gap: 10,
  },
  cardScoreHeader: {
    display: "flex", justifyContent: "space-between", alignItems: "center",
  },
  cardScoreNameWrap: {
    display: "flex", alignItems: "center", gap: 10,
  },
  cardScoreName: {
    fontFamily: "var(--font-prose, serif)",
    fontSize: 15, fontWeight: 600,
    color: "var(--color-neutral-900)",
  },
  cardScoreBadge: {
    fontFamily: "var(--font-mono)", fontSize: 10,
    fontWeight: 700, letterSpacing: "0.06em",
    padding: "2px 8px", borderRadius: 999,
    border: "1px solid transparent",
  },
  cardAxisGrid: {
    display: "grid",
    gridTemplateColumns: "1fr 1fr",
    gap: "8px 16px",
  },
  cardScoreNote: {
    padding: "8px 12px",
    background: "var(--color-neutral-50)",
    border: "1px solid var(--color-neutral-200)",
    borderRadius: 4,
    fontSize: 12, color: "var(--color-neutral-800)", lineHeight: 1.6,
    fontFamily: "var(--font-prose, serif)",
  },
  editsBlock: {
    display: "flex", flexDirection: "column", gap: 6,
  },
  editsH: {
    margin: 0,
    fontSize: 11, fontWeight: 600,
    textTransform: "uppercase", letterSpacing: "0.06em",
    color: "var(--color-neutral-500)",
  },
  editsList: {
    listStyle: "none", margin: 0, padding: 0,
    display: "flex", flexDirection: "column", gap: 4,
  },
  editRow: {
    display: "flex", justifyContent: "space-between", alignItems: "flex-start",
    gap: 12,
    padding: "8px 12px",
    background: "var(--color-neutral-50)",
    border: "1px solid var(--color-neutral-200)",
    borderRadius: 4,
  },
  editLeft: {
    display: "flex", flexDirection: "column", gap: 2,
    flex: 1, minWidth: 0,
  },
  editField: {
    fontSize: 10, fontWeight: 700, letterSpacing: "0.06em",
    textTransform: "uppercase",
    color: "var(--color-amber-600)",
  },
  editSuggestion: {
    fontSize: 13, color: "var(--color-neutral-800)", lineHeight: 1.5,
  },
  editReplacement: {
    fontSize: 12, color: "var(--color-neutral-600)",
    fontFamily: "var(--font-prose, serif)",
    lineHeight: 1.5,
  },
  smallBtn: {
    padding: "4px 10px",
    background: "var(--color-amber-50, #fffbeb)",
    color: "var(--color-amber-700, #b45309)",
    border: "1px solid var(--color-amber-300, #fcd34d)",
    borderRadius: 4,
    fontSize: 12, fontWeight: 600, cursor: "pointer",
    fontFamily: "var(--font-ui)",
    flexShrink: 0,
  },
};

// Inject the score-spinner keyframes once on module load (HMR-safe).
if (typeof document !== "undefined" && !document.getElementById("bf-stage5-anim")) {
  const styleEl = document.createElement("style");
  styleEl.id = "bf-stage5-anim";
  styleEl.textContent = `@keyframes bf-stage5-spin {
    from { transform: rotate(0deg); } to { transform: rotate(360deg); }
  }`;
  document.head.appendChild(styleEl);
}
