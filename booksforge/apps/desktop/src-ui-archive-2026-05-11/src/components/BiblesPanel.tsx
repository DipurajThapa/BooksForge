/**
 * BiblesPanel — writer-facing form for the character + world bibles.
 *
 * Why this exists: agent-generated bibles cost 2-5 minutes per call and
 * writers with strong existing bible material want to skip generation
 * entirely. The book pipeline auto-detects entries here and skips its
 * Stage 1 (character-bible) and Stage 2 (world-bible) when they're
 * present — so filling in this panel turns a 10-minute pipeline run
 * into a ~5-minute one (just the per-scene drafter).
 *
 * Two tabs: Characters and World. Each saves independently so partial
 * authoring is fine — the writer can fill in the world while still
 * letting the AI generate characters, or vice versa.
 */
import React, { useEffect, useState } from "react";
import { useDialogA11y } from "../lib/useDialogA11y";
import { ipc } from "../lib/ipc";
import { errorMessage } from "../lib/errorMessage";

interface Props {
  onClose: () => void;
}

// ── Local types matching the Rust schema ────────────────────────────────────

interface CharacterCard {
  name:                       string;
  role:                       string;
  external_objective:         string;
  internal_need:              string;
  fear_or_wound:              string;
  secret_or_contradiction:    string;
  voice_traits:               string[];
  relationships:              { to: string; nature: string }[];
  chapter_arc:                string[];
  emotional_turning_points:   string[];
}

interface WorldLocation {
  name:               string;
  purpose_in_story:   string;
  sensory_signature:  string;
  key_constraints:    string;
}

interface SensoryPalette {
  sight: string; sound: string; smell: string; touch: string; taste: string;
}

interface WorldBible {
  main_locations:          WorldLocation[];
  social_rules:            string[];
  history:                 string;
  sensory_palette:         SensoryPalette;
  conflict_sources:        string[];
  symbolic_motifs:         string[];
  continuity_constraints:  string[];
}

const EMPTY_CHARACTER: CharacterCard = {
  name: "", role: "supporting",
  external_objective: "", internal_need: "",
  fear_or_wound: "", secret_or_contradiction: "",
  voice_traits: [], relationships: [], chapter_arc: [], emotional_turning_points: [],
};

const EMPTY_WORLD: WorldBible = {
  main_locations: [], social_rules: [], history: "",
  sensory_palette: { sight: "", sound: "", smell: "", touch: "", taste: "" },
  conflict_sources: [], symbolic_motifs: [], continuity_constraints: [],
};

const ROLE_OPTIONS = ["protagonist", "antagonist", "mentor", "foil", "ally", "supporting"];

// ── Component ────────────────────────────────────────────────────────────────

export default function BiblesPanel({ onClose }: Props) {
  const { dialogProps, titleId } = useDialogA11y(onClose);
  const [tab,           setTab]           = useState<"characters" | "world">("characters");
  const [characters,    setCharacters]    = useState<CharacterCard[]>([]);
  const [world,         setWorld]         = useState<WorldBible>(EMPTY_WORLD);
  const [loading,       setLoading]       = useState(true);
  const [saving,        setSaving]        = useState(false);
  const [savedAt,       setSavedAt]       = useState<number | null>(null);
  const [error,         setError]         = useState<string | null>(null);

  useEffect(() => {
    ipc.biblesLoad().then((res) => {
      const cs = (res.characters as Partial<CharacterCard>[]).map((c) => ({
        ...EMPTY_CHARACTER, ...c,
        voice_traits:             c.voice_traits ?? [],
        relationships:            c.relationships ?? [],
        chapter_arc:              c.chapter_arc ?? [],
        emotional_turning_points: c.emotional_turning_points ?? [],
      }));
      setCharacters(cs);
      if (res.world && typeof res.world === "object") {
        const w = res.world as Partial<WorldBible>;
        setWorld({
          ...EMPTY_WORLD, ...w,
          main_locations:         w.main_locations ?? [],
          social_rules:           w.social_rules ?? [],
          sensory_palette:        { ...EMPTY_WORLD.sensory_palette, ...(w.sensory_palette ?? {}) },
          conflict_sources:       w.conflict_sources ?? [],
          symbolic_motifs:        w.symbolic_motifs ?? [],
          continuity_constraints: w.continuity_constraints ?? [],
        });
      }
    }).catch((e) => setError(errorMessage(e)))
      .finally(() => setLoading(false));
  }, []);

  async function saveCharacters() {
    setSaving(true); setError(null);
    try {
      const res = await ipc.biblesSave({ characters });
      setSavedAt(Date.now());
      // Surface the diff so the writer sees what landed.
      console.info("[bibles] characters saved:", res);
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setSaving(false);
    }
  }

  async function saveWorld() {
    setSaving(true); setError(null);
    try {
      const res = await ipc.biblesSave({ world });
      setSavedAt(Date.now());
      console.info("[bibles] world saved:", res);
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setSaving(false);
    }
  }

  // ── Character helpers ────────────────────────────────────────────
  function updateChar(idx: number, patch: Partial<CharacterCard>) {
    setCharacters((prev) => prev.map((c, i) => (i === idx ? { ...c, ...patch } : c)));
  }
  function addChar() {
    setCharacters((prev) => [...prev, { ...EMPTY_CHARACTER, role: prev.length === 0 ? "protagonist" : prev.length === 1 ? "antagonist" : "supporting" }]);
  }
  function removeChar(idx: number) {
    if (!window.confirm("Remove this character? You can re-add them, but their notes will be lost.")) return;
    setCharacters((prev) => prev.filter((_, i) => i !== idx));
  }

  // ── World helpers ────────────────────────────────────────────────
  function addLocation() {
    setWorld((w) => ({ ...w, main_locations: [...w.main_locations, { name: "", purpose_in_story: "", sensory_signature: "", key_constraints: "" }] }));
  }
  function updateLocation(i: number, patch: Partial<WorldLocation>) {
    setWorld((w) => ({ ...w, main_locations: w.main_locations.map((l, idx) => idx === i ? { ...l, ...patch } : l) }));
  }
  function removeLocation(i: number) {
    setWorld((w) => ({ ...w, main_locations: w.main_locations.filter((_, idx) => idx !== i) }));
  }

  // ── Render ───────────────────────────────────────────────────────
  return (
    <div style={s.overlay} role="presentation">
      <div {...dialogProps} style={s.dialog}>
        <header style={s.header}>
          <strong id={titleId}>Bibles — character + world</strong>
          <button style={s.close} onClick={onClose} aria-label="Close panel">✕</button>
        </header>

        <div style={s.tabBar}>
          <TabBtn active={tab === "characters"} onClick={() => setTab("characters")}>
            Characters{characters.length > 0 && ` · ${characters.length}`}
          </TabBtn>
          <TabBtn active={tab === "world"} onClick={() => setTab("world")}>
            World{world.main_locations.length > 0 && ` · ${world.main_locations.length} loc`}
          </TabBtn>
          <span style={s.tabBarSpacer} />
          {savedAt && (
            <span style={s.savedBadge}>
              Saved {Math.max(1, Math.round((Date.now() - savedAt) / 1000))}s ago
            </span>
          )}
        </div>

        {loading ? (
          <div style={s.body}><p style={s.muted}>Loading…</p></div>
        ) : tab === "characters" ? (
          <div style={s.body}>
            <p style={s.intro}>
              Add the characters that drive the book. The AI scene drafter
              will use voice traits, internal needs, and chapter arcs from
              every entry here to write consistent prose.
            </p>
            {characters.length === 0 && (
              <p style={s.empty}>
                No characters yet. Add at least one (the protagonist) — the
                AI scene drafter falls back to thin defaults without a
                bible, which costs you voice consistency.
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
            {error && <p style={s.error}>{error}</p>}
            <div style={s.footer}>
              <button style={s.ghostBtn} onClick={onClose}>Close</button>
              <button style={s.primaryBtn} onClick={saveCharacters} disabled={saving}>
                {saving ? "Saving…" : "Save characters"}
              </button>
            </div>
          </div>
        ) : (
          <div style={s.body}>
            <p style={s.intro}>
              Locations, social rules, history, sensory palette, motifs.
              Every prose stage uses these — the more grounded these are,
              the more grounded the AI's prose will be.
            </p>

            <h3 style={s.sectionH}>Locations</h3>
            {world.main_locations.length === 0 && (
              <p style={s.empty}>
                Add at least one location with a specific sensory signature
                ("pine resin and wet stone", not "outdoor smells").
              </p>
            )}
            <ul style={s.cardList}>
              {world.main_locations.map((l, i) => (
                <li key={i} style={s.card}>
                  <div style={s.cardHeader}>
                    <input
                      style={{ ...s.input, fontWeight: 600 }}
                      value={l.name}
                      placeholder="Location name (e.g. The Workshop)"
                      onChange={(e) => updateLocation(i, { name: e.target.value })}
                    />
                    <button style={s.removeBtn} onClick={() => removeLocation(i)}>Remove</button>
                  </div>
                  <Field label="Purpose in story">
                    <input style={s.input} value={l.purpose_in_story}
                      onChange={(e) => updateLocation(i, { purpose_in_story: e.target.value })}
                      placeholder="What plot work this location does." />
                  </Field>
                  <Field label="Sensory signature" hint="Specific details only — wet wool, oil, the click of the door hinge.">
                    <input style={s.input} value={l.sensory_signature}
                      onChange={(e) => updateLocation(i, { sensory_signature: e.target.value })}
                      placeholder="pine resin and wet stone" />
                  </Field>
                  <Field label="Key constraints" hint="Optional — rules that govern action there.">
                    <input style={s.input} value={l.key_constraints}
                      onChange={(e) => updateLocation(i, { key_constraints: e.target.value })}
                      placeholder="Empty if none." />
                  </Field>
                </li>
              ))}
            </ul>
            <div style={s.addRow}>
              <button style={s.addBtn} onClick={addLocation}>+ Add location</button>
            </div>

            <h3 style={s.sectionH}>History</h3>
            <textarea
              style={{ ...s.input, minHeight: 80, fontFamily: "var(--font-prose)" }}
              value={world.history}
              onChange={(e) => setWorld({ ...world, history: e.target.value })}
              placeholder="Backstory the writer needs to know but never explicitly says (≥30 words)."
            />

            <h3 style={s.sectionH}>Sensory palette</h3>
            <div style={s.gridTwo}>
              {(["sight","sound","smell","touch","taste"] as const).map((sense) => (
                <Field key={sense} label={sense}>
                  <input style={s.input}
                    value={world.sensory_palette[sense]}
                    onChange={(e) => setWorld({
                      ...world,
                      sensory_palette: { ...world.sensory_palette, [sense]: e.target.value },
                    })}
                    placeholder={
                      sense === "sight" ? "low gray light, dust on the bench"
                      : sense === "sound" ? "the click of the wrong-side switch"
                      : sense === "smell" ? "wet wool and old oil"
                      : sense === "touch" ? "cold brass"
                      : "tea gone cold"
                    } />
                </Field>
              ))}
            </div>

            <h3 style={s.sectionH}>Lists</h3>
            <Field label="Social rules" hint="One per line.">
              <ListTextarea
                value={world.social_rules}
                onChange={(v) => setWorld({ ...world, social_rules: v })}
                placeholder={"small-town news travels by post office before phone\nwidows are visited unannounced for the first six weeks"}
              />
            </Field>
            <Field label="Conflict sources" hint="One per line.">
              <ListTextarea
                value={world.conflict_sources}
                onChange={(v) => setWorld({ ...world, conflict_sources: v })}
                placeholder="a hidden life she did not know about"
              />
            </Field>
            <Field label="Symbolic motifs" hint="One per line.">
              <ListTextarea
                value={world.symbolic_motifs}
                onChange={(v) => setWorld({ ...world, symbolic_motifs: v })}
                placeholder={"the wound clock\nthe wrong-side light switch"}
              />
            </Field>
            <Field label="Continuity constraints" hint="Things the writer MUST NOT contradict. One per line.">
              <ListTextarea
                value={world.continuity_constraints}
                onChange={(v) => setWorld({ ...world, continuity_constraints: v })}
                placeholder={"Ada's husband died exactly six weeks before chapter 1\nThe clock was wound when she found it; she did not wind it"}
              />
            </Field>

            {error && <p style={s.error}>{error}</p>}
            <div style={s.footer}>
              <button style={s.ghostBtn} onClick={onClose}>Close</button>
              <button style={s.primaryBtn} onClick={saveWorld} disabled={saving}>
                {saving ? "Saving…" : "Save world bible"}
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

// ── CharacterEditor ─────────────────────────────────────────────────────────

function CharacterEditor({
  index, card, onChange, onRemove,
}: {
  index:    number;
  card:     CharacterCard;
  onChange: (patch: Partial<CharacterCard>) => void;
  onRemove: () => void;
}) {
  return (
    <li style={s.card}>
      <div style={s.cardHeader}>
        <input
          style={{ ...s.input, fontWeight: 600, flex: 1 }}
          value={card.name}
          placeholder={`Character ${index + 1} name (e.g. Elara)`}
          onChange={(e) => onChange({ name: e.target.value })}
        />
        <select
          style={s.input}
          value={card.role}
          onChange={(e) => onChange({ role: e.target.value })}
        >
          {ROLE_OPTIONS.map((r) => <option key={r} value={r}>{r}</option>)}
        </select>
        <button style={s.removeBtn} onClick={onRemove}>Remove</button>
      </div>

      <Field label="External objective" hint="What they're after in the world.">
        <input style={s.input} value={card.external_objective}
          onChange={(e) => onChange({ external_objective: e.target.value })}
          placeholder="Find Maeve Kowalski and ask the question." />
      </Field>
      <Field label="Internal need" hint="What they need to learn/face/release.">
        <input style={s.input} value={card.internal_need}
          onChange={(e) => onChange({ internal_need: e.target.value })}
          placeholder="To stop arranging the silence between them." />
      </Field>
      <Field label="Fear or wound">
        <input style={s.input} value={card.fear_or_wound}
          onChange={(e) => onChange({ fear_or_wound: e.target.value })}
          placeholder="Forty years of marriage missing a person inside it." />
      </Field>
      <Field label="Secret or contradiction">
        <input style={s.input} value={card.secret_or_contradiction}
          onChange={(e) => onChange({ secret_or_contradiction: e.target.value })}
          placeholder="Knew about the letters years ago and chose not to look." />
      </Field>
      <Field label="Voice traits" hint="3-6 specific markers — vocabulary, sentence rhythm, evasion patterns. Avoid 'kind' / 'bright' (vague).">
        <ListTextarea
          value={card.voice_traits}
          onChange={(v) => onChange({ voice_traits: v })}
          placeholder={"sentences truncated when cornered\nuses tool-shop vocabulary by reflex\nrarely uses adverbs"}
        />
      </Field>
      <Field label="Chapter arc" hint="What changes for this character per chapter, one entry per chapter.">
        <ListTextarea
          value={card.chapter_arc}
          onChange={(v) => onChange({ chapter_arc: v })}
          placeholder={"Ch1: opens the drawer, finds the letters\nCh2: drives to Maeve's house"}
        />
      </Field>
      <Field label="Emotional turning points" hint="Beats where the character's understanding shifts.">
        <ListTextarea
          value={card.emotional_turning_points}
          onChange={(v) => onChange({ emotional_turning_points: v })}
          placeholder={"reading the name Maeve on the envelope\nseeing the woman on the porch already waiting"}
        />
      </Field>
    </li>
  );
}

// ── Subcomponents ───────────────────────────────────────────────────────────

function TabBtn({ active, children, onClick }: {
  active: boolean; children: React.ReactNode; onClick: () => void;
}) {
  return (
    <button
      style={{
        ...s.tabBtn,
        borderBottom: active ? "2px solid var(--color-amber-600)" : "2px solid transparent",
        color: active ? "var(--color-text-primary)" : "var(--color-text-tertiary)",
        fontWeight: active ? 600 : 500,
      }}
      onClick={onClick}
    >
      {children}
    </button>
  );
}

function Field({ label, hint, children }: {
  label:    string;
  hint?:    string;
  children: React.ReactNode;
}) {
  return (
    <label style={s.field}>
      <span style={s.fieldLabel}>{label}</span>
      {children}
      {hint && <span style={s.fieldHint}>{hint}</span>}
    </label>
  );
}

function ListTextarea({ value, onChange, placeholder }: {
  value:       string[];
  onChange:    (v: string[]) => void;
  placeholder: string;
}) {
  return (
    <textarea
      style={{ ...s.input, minHeight: 60, fontFamily: "var(--font-prose)" }}
      value={value.join("\n")}
      placeholder={placeholder}
      onChange={(e) => onChange(
        e.target.value.split("\n").map((l) => l.trim()).filter((l) => l.length > 0),
      )}
    />
  );
}

const s: Record<string, React.CSSProperties> = {
  overlay: {
    position: "fixed", inset: 0,
    background: "rgba(0,0,0,0.45)",
    display: "flex", alignItems: "center", justifyContent: "center",
    zIndex: 1000,
  },
  dialog: {
    width: 760, maxHeight: "90vh",
    display: "flex", flexDirection: "column",
    background: "var(--color-surface, #fff)",
    border: "1px solid var(--color-border)", borderRadius: 8,
    boxShadow: "0 20px 60px rgba(0,0,0,0.3)",
    fontFamily: "var(--font-ui)",
  },
  header: {
    display: "flex", justifyContent: "space-between", alignItems: "center",
    padding: "var(--space-3) var(--space-4)",
    borderBottom: "1px solid var(--color-border)",
  },
  close: { background: "none", border: "none", cursor: "pointer", fontSize: 16, color: "var(--color-text-tertiary)" },
  tabBar: {
    display: "flex", alignItems: "center",
    padding: "0 var(--space-4)",
    borderBottom: "1px solid var(--color-border)",
  },
  tabBtn: {
    padding: "10px 12px",
    background: "none", border: "none", borderBottom: "2px solid transparent",
    cursor: "pointer", fontSize: 13,
    fontFamily: "inherit",
  },
  tabBarSpacer: { flex: 1 },
  savedBadge: {
    fontSize: 11, color: "var(--color-success, #22c55e)",
    marginRight: "var(--space-2)",
  },
  body: {
    padding: "var(--space-4)", overflowY: "auto", flex: 1,
    display: "flex", flexDirection: "column", gap: "var(--space-3)",
  },
  intro: { margin: 0, fontSize: 13, color: "var(--color-text-secondary)", lineHeight: 1.6 },
  empty: { margin: 0, fontSize: 12, color: "var(--color-text-tertiary)", lineHeight: 1.6 },
  muted: { color: "var(--color-text-tertiary)", fontSize: 13 },
  sectionH: {
    fontSize: 12, fontWeight: 600, textTransform: "uppercase", letterSpacing: "0.06em",
    color: "var(--color-text-tertiary)",
    margin: "var(--space-3) 0 var(--space-1)",
  },
  cardList: { listStyle: "none", margin: 0, padding: 0, display: "flex", flexDirection: "column", gap: "var(--space-2)" },
  card: {
    border: "1px solid var(--color-border)", borderRadius: 6,
    padding: "var(--space-3)",
    display: "flex", flexDirection: "column", gap: "var(--space-2)",
  },
  cardHeader: { display: "flex", gap: "var(--space-2)", alignItems: "center" },
  field: { display: "flex", flexDirection: "column", gap: 2 },
  fieldLabel: { fontSize: 11, fontWeight: 600, color: "var(--color-text-secondary)" },
  fieldHint: { fontSize: 11, color: "var(--color-text-tertiary)", marginTop: 2 },
  input: {
    width: "100%", boxSizing: "border-box",
    padding: "6px 10px",
    border: "1px solid var(--color-border)", borderRadius: 4,
    background: "var(--color-surface)",
    color: "var(--color-text-primary)",
    fontFamily: "var(--font-ui)", fontSize: 13, outline: "none",
  },
  gridTwo: { display: "grid", gridTemplateColumns: "1fr 1fr", gap: "var(--space-2)" },
  addRow: { display: "flex" },
  addBtn: {
    background: "transparent", border: "1px dashed var(--color-border)",
    borderRadius: 4, padding: "6px 10px", cursor: "pointer",
    color: "var(--color-text-secondary)", fontSize: 12, fontFamily: "var(--font-ui)",
  },
  removeBtn: {
    background: "transparent", border: "1px solid var(--color-border)",
    borderRadius: 4, padding: "4px 8px", cursor: "pointer",
    color: "var(--color-text-tertiary)", fontSize: 11, fontFamily: "var(--font-ui)",
  },
  error: { color: "var(--color-error)", fontSize: 12, fontFamily: "var(--font-mono)", margin: 0 },
  footer: {
    display: "flex", justifyContent: "flex-end", gap: "var(--space-2)",
    paddingTop: "var(--space-3)",
    borderTop: "1px solid var(--color-border)",
  },
  primaryBtn: {
    padding: "var(--space-2) var(--space-5)", background: "var(--color-amber-600)",
    color: "#fff", border: "none", borderRadius: 5,
    fontSize: 14, fontWeight: 600, cursor: "pointer", fontFamily: "var(--font-ui)",
  },
  ghostBtn: {
    padding: "var(--space-2) var(--space-4)", background: "transparent",
    color: "var(--color-text-secondary)", border: "1px solid var(--color-border)",
    borderRadius: 5, fontSize: 14, cursor: "pointer", fontFamily: "var(--font-ui)",
  },
};
