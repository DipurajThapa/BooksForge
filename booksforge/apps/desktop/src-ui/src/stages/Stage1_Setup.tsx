/**
 * Stage 1 — Book Setup (Phase B Step 1 + Phase C concept scorer).
 *
 * Editable Brief form. Loads from `book:project_brief` memory (set by
 * the wizard's `projectBriefSave` call); lets the writer edit and
 * re-save. Every downstream agent reads the saved brief, so this
 * panel is the single source of truth for "the book's spine."
 *
 * Phase C addition: the "Refine with AI" button runs the `concept-scorer`
 * agent on the saved brief and shows a 5-axis score (Clarity,
 * Originality, Emotional pull, Market fit, Execution potential) plus
 * 0-5 targeted revisions the writer can apply with one click. Gate
 * passes when composite ≥ 8.5 AND every axis ≥ 7.0.
 */
import { useEffect, useState } from "react";
import type { OpenProjectResult } from "@booksforge/shared-types";
import { ipc } from "../lib/ipc";
import { errorMessage } from "../lib/errorMessage";
import { useToast } from "../components/ToastProvider";

// Mirror of `booksforge_domain::ConceptScoreProposal`. Hand-typed
// because the domain crate doesn't derive ts-rs — the IPC carries the
// score as a JSON string in `proposal_json` and the UI parses.
interface ConceptScoreAxis  { score: number; reason?: string }
interface ConceptEdit       { field: string; suggestion: string; replacement?: string }
interface ConceptScore {
  clarity:              ConceptScoreAxis;
  originality:          ConceptScoreAxis;
  emotional_pull:       ConceptScoreAxis;
  market_fit:           ConceptScoreAxis;
  execution_potential:  ConceptScoreAxis;
  overall_summary?:     string;
  edits?:               ConceptEdit[];
}

type ScoreState =
  | { kind: "idle" }
  | { kind: "running"; startedAt: number }
  | { kind: "ready";   score: ConceptScore }
  | { kind: "error";   message: string };

const AXIS_FLOOR          = 7.0;
const COMPOSITE_THRESHOLD = 8.5;

interface Props {
  project:    OpenProjectResult;
  /** Called after a successful save so the StageRail can refresh
   *  status. Optional — panels without save actions just ignore it. */
  onChanged?: () => void;
  /** Called by the "Save & continue" CTA after a successful save.
   *  Advances the rail to the next stage. Optional so the panel can
   *  also be rendered standalone (e.g. in tests). */
  onAdvance?: () => void;
}

// Local form shape — matches the wizard's brief shape but flattened
// for in-form editing.
interface BriefForm {
  premise:                string;
  background:             string;  // stored in creative_seed for now
  genre:                  string;  // may contain " / sub-genre"
  tone:                   string;  // may contain " — writing-style"
  audience:               string;
  target_word_count:      string;  // string for input, parsed on save
  key_promises:           string;  // textarea, one per line
  comp_titles_or_authors: string;  // textarea, one per line
}

const EMPTY: BriefForm = {
  premise:                "",
  background:             "",
  genre:                  "",
  tone:                   "",
  audience:               "",
  target_word_count:      "75000",
  key_promises:           "",
  comp_titles_or_authors: "",
};

export default function Stage1_Setup({ project, onChanged, onAdvance }: Props) {
  const [form,         setForm]         = useState<BriefForm>(EMPTY);
  const [loading,      setLoading]      = useState(true);
  const [saving,       setSaving]       = useState(false);
  const [loaded,       setLoaded]       = useState(false);
  const [error,        setError]        = useState<string | null>(null);
  const [savedHint,    setSavedHint]    = useState<string | null>(null);
  const [briefSource,  setBriefSource]  = useState<string>("");
  const [briefSavedAt, setBriefSavedAt] = useState<string>("");
  const [scoreState,   setScoreState]   = useState<ScoreState>({ kind: "idle" });
  const toast = useToast();

  // Load on mount.
  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const r = await ipc.projectBriefLoad();
        if (cancelled) return;
        setLoaded(r.loaded);
        setBriefSource((r as { source?: string }).source ?? "");
        setBriefSavedAt((r as { updated_at?: string }).updated_at ?? "");
        if (r.loaded) {
          const b = r.brief_json as Partial<{
            premise:                string;
            creative_seed:          string | null;
            genre:                  string;
            tone:                   string;
            audience:               string;
            target_word_count:      number;
            key_promises:           string[];
            comp_titles_or_authors: string[];
          }>;
          setForm({
            premise:                asStr(b.premise),
            background:             asStr(b.creative_seed),
            genre:                  asStr(b.genre),
            tone:                   asStr(b.tone),
            audience:               asStr(b.audience),
            target_word_count:      String(b.target_word_count ?? 75000),
            key_promises:           (b.key_promises ?? []).join("\n"),
            comp_titles_or_authors: (b.comp_titles_or_authors ?? []).join("\n"),
          });
        }
      } catch (e) {
        if (!cancelled) setError(errorMessage(e));
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();
    return () => { cancelled = true; };
  }, []);

  function update<K extends keyof BriefForm>(k: K, v: BriefForm[K]) {
    setForm((f) => ({ ...f, [k]: v }));
    setError(null);
    setSavedHint(null);
  }

  /**
   * Run the concept-scorer agent against the SAVED brief. Saves first
   * if there are unsaved changes so the agent reads the writer's
   * current intent, not a stale version. Light tier auto-resolves.
   */
  async function handleRefine() {
    // Save first so the agent reads what's on screen, not what's in
    // memory. handleSave returns false on validation or IPC failure
    // and already surfaces the error via toast + inline message —
    // we just abort the refine so the score state doesn't go into
    // "running" against a stale brief.
    if (loaded) {
      const ok = await handleSave();
      if (!ok) return;
    }
    setScoreState({ kind: "running", startedAt: Date.now() });
    try {
      const r = await ipc.agentRunConceptScorer({
        project_id: project.project_id,
        model:      null,  // auto-resolve to Light tier
      });
      if (r.status !== "completed" || !r.proposal_json) {
        setScoreState({
          kind: "error",
          message: r.error ?? `Agent returned status: ${r.status}`,
        });
        return;
      }
      const score = JSON.parse(r.proposal_json) as ConceptScore;
      setScoreState({ kind: "ready", score });
    } catch (e) {
      setScoreState({ kind: "error", message: errorMessage(e) });
    }
  }

  /**
   * Apply a single editor-suggested edit to the form. For string-typed
   * fields we just substitute. For array-typed fields (key_promises,
   * comp_titles_or_authors) we splice in the replacement at the start
   * so the writer can see what changed.
   */
  function applyEdit(edit: ConceptEdit) {
    const r = edit.replacement?.trim();
    if (!r) return;  // structural edit; writer applies manually
    switch (edit.field) {
      case "premise":      setForm((f) => ({ ...f, premise: r })); break;
      case "audience":     setForm((f) => ({ ...f, audience: r })); break;
      case "genre":        setForm((f) => ({ ...f, genre: r })); break;
      case "tone":         setForm((f) => ({ ...f, tone: r })); break;
      case "key_promises":
        setForm((f) => ({
          ...f,
          key_promises: r + (f.key_promises ? "\n" + f.key_promises : ""),
        }));
        break;
      case "comp_titles_or_authors":
        setForm((f) => ({
          ...f,
          comp_titles_or_authors:
            r + (f.comp_titles_or_authors ? "\n" + f.comp_titles_or_authors : ""),
        }));
        break;
    }
    setSavedHint(null);
    setError(null);
  }

  /**
   * Returns true on success, false on validation/IPC failure. The
   * "Save & continue" handler uses the return value to decide
   * whether to advance the rail. We surface failures via toast in
   * addition to the inline error so the writer notices even if
   * they've scrolled away from the form.
   */
  async function handleSave(): Promise<boolean> {
    // Client-side validation matching the domain validator so the
    // user gets a clear error here instead of a generic IPC failure.
    const targetWords = Number(form.target_word_count);
    if (!Number.isFinite(targetWords) || targetWords < 5000 || targetWords > 250_000) {
      const msg = "Target word count must be between 5 000 and 250 000.";
      setError(msg);
      toast.push({ severity: "warning", body: msg });
      return false;
    }
    const promises = form.key_promises
      .split("\n").map((l) => l.trim()).filter(Boolean);
    if (promises.length === 0 || promises.length > 6) {
      const msg = "Key promises must have 1–6 lines.";
      setError(msg);
      toast.push({ severity: "warning", body: msg });
      return false;
    }
    if (!form.premise.trim()) {
      const msg = "Premise is required.";
      setError(msg);
      toast.push({ severity: "warning", body: msg });
      return false;
    }
    setSaving(true); setError(null);
    try {
      const compTitles = form.comp_titles_or_authors
        .split("\n").map((l) => l.trim()).filter(Boolean);
      const payload = {
        title_suggestions:      [project.title],
        mode:                   "fiction",
        genre:                  form.genre || "literary fiction",
        audience:               form.audience || "adult literary readers",
        tone:                   form.tone || "spare",
        target_word_count:      targetWords,
        premise:                form.premise.trim(),
        key_promises:           promises,
        questions_for_user:     [],
        comp_titles_or_authors: compTitles,
        theme_keywords:         [],
        forbidden_tropes:       [],
        era_setting:            null,
        cultural_context:       null,
        creative_seed:          form.background.trim() || null,
      };
      const r = await ipc.projectBriefSave({ brief_json: payload });
      setLoaded(true);
      setBriefSource((r as { source?: string }).source ?? "user-edit");
      setBriefSavedAt((r as { updated_at?: string }).updated_at ?? new Date().toISOString());
      setSavedHint("Saved. Every agent run after this picks up the new brief.");
      onChanged?.();
      return true;
    } catch (e) {
      const msg = errorMessage(e);
      setError(msg);
      toast.push({
        severity: "error",
        title: "Brief save failed",
        body: msg,
      });
      return false;
    } finally {
      setSaving(false);
    }
  }

  /**
   * F5 — Save the brief and, on success, advance to Stage 2. If the
   * save fails the writer stays on this stage; toast + inline error
   * tell them why.
   */
  async function handleSaveAndContinue() {
    const ok = await handleSave();
    if (ok) {
      toast.push({
        severity: "success",
        body: "Brief saved. Next: Audience map.",
      });
      onAdvance?.();
    }
  }

  return (
    <div style={s.root}>
      <div style={s.col}>
        {/* Header */}
        <header style={s.header}>
          <p style={s.stageNum}>Stage 1 of 6</p>
          <h1 style={s.title}>Book Setup</h1>
          <p style={s.subtitle}>
            <b>{project.title}</b> · <span style={s.muted}>{project.author}</span>
          </p>
        </header>

        {/* Provenance / status banner */}
        {!loading && loaded && briefSource && (
          <div style={s.bannerOk}>
            ✓ Brief loaded from <b>{labelForSource(briefSource)}</b>
            {briefSavedAt && <> · last saved {formatLastSaved(briefSavedAt)}</>}.
            Edit any field below and click Save — every agent run after
            that picks up the new values.
          </div>
        )}
        {!loading && !loaded && (
          <div style={s.bannerWarn}>
            <b>No brief saved yet.</b> If you started this project from the
            wizard, the brief should land here automatically. Otherwise,
            fill in any fields and click Save to seed the project.
          </div>
        )}

        {loading && <p style={s.muted}>Loading brief…</p>}

        {!loading && (
          <>
            <Section title="Concept" hint="The book's spine. Every downstream agent reads these fields.">
              <Field label="Premise" required hint="1–3 sentences in your own register.">
                <textarea
                  style={{ ...s.input, minHeight: 90, fontFamily: "var(--font-prose, serif)" }}
                  value={form.premise}
                  onChange={(e) => update("premise", e.target.value)}
                  placeholder="A clockmaker's widow finds twenty-three sealed letters addressed to a woman she has never heard of."
                />
              </Field>
              <Field label="Background" hint="World, setting, era, or any context the AI should respect.">
                <textarea
                  style={{ ...s.input, minHeight: 70, fontFamily: "var(--font-prose, serif)" }}
                  value={form.background}
                  onChange={(e) => update("background", e.target.value)}
                  placeholder="1990s rural Pennsylvania. Small-town news travels by post office before phone."
                />
              </Field>
              <div style={s.gridTwo}>
                <Field label="Genre">
                  <input style={s.input} value={form.genre}
                    onChange={(e) => update("genre", e.target.value)}
                    placeholder="literary fiction" />
                </Field>
                <Field label="Tone / writing style">
                  <input style={s.input} value={form.tone}
                    onChange={(e) => update("tone", e.target.value)}
                    placeholder="spare — lyrical-precise" />
                </Field>
                <Field label="Audience">
                  <input style={s.input} value={form.audience}
                    onChange={(e) => update("audience", e.target.value)}
                    placeholder="adult literary readers" />
                </Field>
                <Field label="Target word count" hint="5 000 – 250 000.">
                  <input style={s.input} type="number" min={5000} max={250_000}
                    value={form.target_word_count}
                    onChange={(e) => update("target_word_count", e.target.value)} />
                </Field>
              </div>
            </Section>

            <Section title="Reader promises" hint="1–6 lines. What the reader will get from this book. Drives every chapter.">
              <Field label="Key promises" required>
                <textarea
                  style={{ ...s.input, minHeight: 100, fontFamily: "var(--font-prose, serif)" }}
                  value={form.key_promises}
                  onChange={(e) => update("key_promises", e.target.value)}
                  placeholder={
                    "Sustained dread anchored in everyday objects\n" +
                    "A village whose hierarchy is the real horror\n" +
                    "An ending that does not resolve cleanly"
                  }
                />
              </Field>
            </Section>

            <Section title="Positioning" hint="Anchors the AI's voice + market positioning. Not used for imitation.">
              <Field label="Comparable books or authors" hint="One per line.">
                <textarea
                  style={{ ...s.input, minHeight: 70, fontFamily: "var(--font-prose, serif)" }}
                  value={form.comp_titles_or_authors}
                  onChange={(e) => update("comp_titles_or_authors", e.target.value)}
                  placeholder={"Marilynne Robinson — Gilead\nCormac McCarthy — The Road"}
                />
              </Field>
            </Section>

            {error && <div style={s.error}>{error}</div>}
            {savedHint && <div style={s.savedHint}>{savedHint}</div>}

            <div style={s.footer}>
              <button
                style={s.ghostBtn}
                onClick={handleRefine}
                disabled={!loaded || scoreState.kind === "running" || saving}
                title={!loaded
                  ? "Save the brief first"
                  : "Run the concept-scorer agent (~30-60s on qwen3.5:9b)"}
              >
                {scoreState.kind === "running"
                  ? "Scoring…"
                  : "✨ Refine with AI"}
              </button>
              <button
                style={s.ghostBtn}
                onClick={() => { void handleSave(); }}
                disabled={saving}
                title="Save without leaving this stage"
              >
                {saving ? "Saving…" : "Save"}
              </button>
              <button
                style={{ ...s.primaryBtn, ...(saving ? s.primaryBtnBusy : {}) }}
                onClick={handleSaveAndContinue}
                disabled={saving}
                title="Save the brief and move to Stage 2 — Audience"
              >
                {saving ? "Saving…" : "Save & continue →"}
              </button>
            </div>

            <ScoreSection state={scoreState} onApplyEdit={applyEdit} onClear={() => setScoreState({ kind: "idle" })} />
          </>
        )}
      </div>
    </div>
  );
}

// ── Subcomponents ───────────────────────────────────────────────────────────

/**
 * Renders the concept-scorer's output. Idle is null (no UI noise);
 * Running shows an inline spinner; Ready shows a colour-coded panel
 * with per-axis bars + edit suggestions; Error explains what broke
 * and offers a Try-again.
 */
function ScoreSection({
  state, onApplyEdit, onClear,
}: {
  state:        ScoreState;
  onApplyEdit:  (edit: ConceptEdit) => void;
  onClear:      () => void;
}) {
  if (state.kind === "idle") {
    return (
      <Section
        title="Quality gate"
        hint={`Click ✨ Refine with AI above to score this concept. Pass threshold: composite ≥ ${COMPOSITE_THRESHOLD} AND every axis ≥ ${AXIS_FLOOR}.`}
      >
        <ul style={s.gateList}>
          {GATE_AXES.map((a) => (
            <li key={a.name} style={s.gateRow}>
              <span style={s.gateDot} aria-hidden="true" />
              <span><b>{a.name}</b> — {a.detail}</span>
            </li>
          ))}
        </ul>
      </Section>
    );
  }
  if (state.kind === "running") {
    return (
      <Section title="Scoring…" hint="The concept-scorer agent is reading your brief.">
        <div style={s.scoreRunning}>
          <span style={s.scoreSpinner} aria-hidden="true" />
          <span>Running on Light tier (qwen3.5:9b). Expected ~30-60 s.</span>
        </div>
      </Section>
    );
  }
  if (state.kind === "error") {
    return (
      <Section title="Score failed" hint="The agent returned an error or unparseable output.">
        <div style={s.scoreError}>{state.message}</div>
        <div style={{ display: "flex", justifyContent: "flex-end" }}>
          <button style={s.smallBtn} onClick={onClear}>Dismiss</button>
        </div>
      </Section>
    );
  }
  // Ready
  const score = state.score;
  const axes: Array<[string, ConceptScoreAxis]> = [
    ["Clarity",             score.clarity],
    ["Originality",         score.originality],
    ["Emotional pull",      score.emotional_pull],
    ["Market fit",          score.market_fit],
    ["Execution potential", score.execution_potential],
  ];
  const composite = (
    score.clarity.score + score.originality.score + score.emotional_pull.score +
    score.market_fit.score + score.execution_potential.score
  ) / 5;
  const allAxesPass = axes.every(([_, a]) => a.score >= AXIS_FLOOR);
  const passes = composite >= COMPOSITE_THRESHOLD && allAxesPass;
  const weakest = axes.reduce((min, cur) => cur[1].score < min[1].score ? cur : min, axes[0]);
  return (
    <Section
      title={passes ? "✓ Concept passes gate" : "Concept needs revision"}
      hint={passes
        ? `Composite ${composite.toFixed(1)}/10 with every axis ≥ ${AXIS_FLOOR}. Move on to Stage 2.`
        : `Weakest axis: ${weakest[0]} (${weakest[1].score.toFixed(1)}/10). Apply edits below or revise manually.`}
    >
      <div style={s.scoreSummary}>
        <div style={{
          ...s.scoreBig,
          color: passes ? "var(--color-green-700, #15803d)" : "var(--color-amber-700, #b45309)",
        }}>
          {composite.toFixed(1)}
          <span style={s.scoreBigDenom}>/10</span>
        </div>
        <div style={s.scoreBigLabel}>composite</div>
      </div>
      <div style={s.axisGrid}>
        {axes.map(([name, axis]) => (
          <AxisBar key={name} label={name} axis={axis} threshold={AXIS_FLOOR} />
        ))}
      </div>
      {score.overall_summary && (
        <div style={s.overallSummary}>{score.overall_summary}</div>
      )}
      {(score.edits ?? []).length > 0 && (
        <div style={s.editsBlock}>
          <h4 style={s.editsH}>Suggested edits ({(score.edits ?? []).length})</h4>
          <ul style={s.editsList}>
            {(score.edits ?? []).map((edit, i) => (
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
      <div style={{ display: "flex", justifyContent: "flex-end" }}>
        <button style={s.smallBtn} onClick={onClear}>Clear score</button>
      </div>
    </Section>
  );
}

function AxisBar({
  label, axis, threshold,
}: {
  label: string; axis: ConceptScoreAxis; threshold: number;
}) {
  const pct = Math.min(100, Math.max(0, axis.score * 10));
  const colour =
    axis.score >= threshold
      ? "var(--color-green-500, #22c55e)"
      : "var(--color-red-500, #ef4444)";
  return (
    <div style={s.axisRow} title={axis.reason ?? ""}>
      <div style={s.axisHeader}>
        <span style={s.axisLabel}>{label}</span>
        <span style={s.axisScore}>{axis.score.toFixed(1)}</span>
      </div>
      <div style={s.axisTrack}>
        <div style={{ ...s.axisFill, width: `${pct}%`, background: colour }} />
        <div style={{ ...s.axisFloor, left: `${threshold * 10}%` }} />
      </div>
      {axis.reason && <span style={s.axisReason}>{axis.reason}</span>}
    </div>
  );
}

function Section({ title, hint, children }: {
  title: string;
  hint?: string;
  children: React.ReactNode;
}) {
  return (
    <section style={s.section}>
      <header style={s.sectionHeader}>
        <h2 style={s.sectionTitle}>{title}</h2>
        {hint && <p style={s.sectionHint}>{hint}</p>}
      </header>
      <div style={s.sectionBody}>{children}</div>
    </section>
  );
}

function Field({ label, hint, required, children }: {
  label:    string;
  hint?:    string;
  required?: boolean;
  children: React.ReactNode;
}) {
  return (
    <label style={s.field}>
      <span style={s.fieldLabel}>
        {label}{required && <span style={s.required}> *</span>}
      </span>
      {children}
      {hint && <span style={s.fieldHint}>{hint}</span>}
    </label>
  );
}

// ── Helpers ─────────────────────────────────────────────────────────────────

function asStr(v: unknown): string {
  if (typeof v === "string") return v;
  if (v == null) return "";
  return String(v);
}

function labelForSource(source: string): string {
  switch (source) {
    case "wizard":    return "the New Project wizard";
    case "intake":    return "the intake agent";
    case "user-edit": return "your manual edit";
    default:          return source || "unknown";
  }
}

function formatLastSaved(iso: string): string {
  const t = Date.parse(iso);
  if (Number.isNaN(t)) return "recently";
  const elapsedMs = Date.now() - t;
  const minutes = Math.floor(elapsedMs / 60_000);
  if (minutes < 1)  return "just now";
  if (minutes < 60) return `${minutes} min ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours} h ago`;
  const days = Math.floor(hours / 24);
  if (days < 30) return `${days} d ago`;
  return new Date(t).toLocaleDateString();
}

const GATE_AXES = [
  { name: "Clarity",            detail: "Does the premise read in one breath?" },
  { name: "Originality",        detail: "vs. an embedded public-domain corpus + your named comps." },
  { name: "Emotional pull",     detail: "Is there a wound, a wonder, or a question?" },
  { name: "Market fit",         detail: "Book kind × genre × target word count are coherent." },
  { name: "Execution potential", detail: "Can this book actually deliver what the premise promises?" },
];

// ── Styles ──────────────────────────────────────────────────────────────────

const s: Record<string, React.CSSProperties> = {
  root: {
    height: "100%",
    overflow: "auto",
    padding: "32px 24px 48px",
    display: "flex", justifyContent: "center",
    fontFamily: "var(--font-ui)",
  },
  col: {
    width: "min(760px, 100%)",
    display: "flex", flexDirection: "column", gap: 16,
  },
  header: { display: "flex", flexDirection: "column", gap: 4, marginBottom: 8 },
  stageNum: {
    margin: 0,
    fontSize: 11, fontWeight: 600,
    textTransform: "uppercase", letterSpacing: "0.1em",
    color: "var(--color-amber-600)",
  },
  title: {
    margin: 0,
    fontFamily: "var(--font-prose, serif)",
    fontSize: 32, fontWeight: 700, lineHeight: 1.2,
    color: "var(--color-neutral-900)",
  },
  subtitle: { margin: 0, fontSize: 14, color: "var(--color-neutral-700)" },
  muted:    { color: "var(--color-neutral-500)" },
  bannerOk: {
    padding: "10px 14px",
    background: "rgba(34,197,94,0.06)",
    border: "1px solid rgba(34,197,94,0.25)",
    borderRadius: 6,
    fontSize: 13, color: "var(--color-neutral-800)", lineHeight: 1.5,
  },
  bannerWarn: {
    padding: "10px 14px",
    background: "rgba(245,158,11,0.08)",
    border: "1px solid rgba(245,158,11,0.3)",
    borderRadius: 6,
    fontSize: 13, color: "var(--color-neutral-800)", lineHeight: 1.5,
  },
  section: {
    background: "#fff",
    border: "1px solid var(--color-neutral-200)",
    borderRadius: 6,
    overflow: "hidden",
  },
  sectionHeader: {
    padding: "12px 16px",
    background: "var(--color-neutral-50)",
    borderBottom: "1px solid var(--color-neutral-200)",
  },
  sectionTitle: {
    margin: 0,
    fontSize: 11, fontWeight: 600,
    textTransform: "uppercase", letterSpacing: "0.08em",
    color: "var(--color-neutral-700)",
  },
  sectionHint: {
    margin: "4px 0 0",
    fontSize: 11, color: "var(--color-neutral-500)", lineHeight: 1.5,
  },
  sectionBody: {
    padding: 16,
    display: "flex", flexDirection: "column", gap: 12,
  },
  field: { display: "flex", flexDirection: "column", gap: 4 },
  fieldLabel: {
    fontSize: 11, fontWeight: 600,
    color: "var(--color-neutral-700)",
    textTransform: "uppercase", letterSpacing: "0.04em",
  },
  fieldHint: { fontSize: 11, color: "var(--color-neutral-500)" },
  required:  { color: "var(--color-amber-600)" },
  input: {
    width: "100%", boxSizing: "border-box",
    padding: "8px 12px",
    border: "1px solid var(--color-neutral-300)",
    borderRadius: 4,
    background: "#fff", color: "var(--color-neutral-900)",
    fontFamily: "var(--font-ui)", fontSize: 14, outline: "none",
  },
  gridTwo: {
    display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12,
  },
  error: {
    padding: "8px 12px",
    background: "rgba(220,38,38,0.06)",
    color: "var(--color-red-700, #b91c1c)",
    border: "1px solid rgba(220,38,38,0.25)",
    borderRadius: 4,
    fontFamily: "var(--font-mono)", fontSize: 12,
  },
  savedHint: {
    padding: "8px 12px",
    background: "rgba(34,197,94,0.08)",
    color: "var(--color-green-700, #15803d)",
    border: "1px solid rgba(34,197,94,0.3)",
    borderRadius: 4, fontSize: 12,
  },
  footer: {
    display: "flex", justifyContent: "flex-end", gap: 12,
    marginTop: 4,
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
  gateList: { listStyle: "none", margin: 0, padding: 0, display: "flex", flexDirection: "column", gap: 6 },
  gateRow: {
    display: "flex", alignItems: "flex-start", gap: 10,
    fontSize: 13, color: "var(--color-neutral-800)", lineHeight: 1.5,
  },
  gateDot: {
    width: 4, height: 4, borderRadius: "50%",
    background: "var(--color-amber-500)",
    flexShrink: 0, marginTop: 8,
  },
  gateThreshold: {
    margin: "10px 0 0",
    fontSize: 12, color: "var(--color-neutral-600)",
  },
  // Score-section styles ----------------------------------------------------
  scoreRunning: {
    display: "flex", alignItems: "center", gap: 10,
    padding: "12px 14px",
    background: "var(--color-neutral-50)",
    border: "1px solid var(--color-neutral-200)",
    borderRadius: 4,
    fontSize: 13, color: "var(--color-neutral-700)",
  },
  scoreSpinner: {
    width: 14, height: 14, flexShrink: 0,
    borderRadius: "50%",
    border: "2px solid var(--color-neutral-300)",
    borderTopColor: "var(--color-amber-600)",
    animation: "bf-stage1-spin 0.9s linear infinite",
  },
  scoreError: {
    padding: "8px 12px",
    background: "rgba(220,38,38,0.06)",
    color: "var(--color-red-700, #b91c1c)",
    border: "1px solid rgba(220,38,38,0.25)",
    borderRadius: 4, fontFamily: "var(--font-mono)", fontSize: 12,
  },
  scoreSummary: {
    display: "flex", alignItems: "baseline", gap: 12,
    padding: "8px 0 12px",
  },
  scoreBig: {
    fontFamily: "var(--font-prose, serif)",
    fontSize: 48, fontWeight: 700, lineHeight: 1,
    fontVariantNumeric: "tabular-nums",
  },
  scoreBigDenom: {
    fontSize: 18, fontWeight: 500,
    color: "var(--color-neutral-500)",
    marginLeft: 4,
  },
  scoreBigLabel: {
    fontSize: 11, fontWeight: 600,
    textTransform: "uppercase", letterSpacing: "0.08em",
    color: "var(--color-neutral-500)",
  },
  axisGrid: {
    display: "grid",
    gridTemplateColumns: "1fr 1fr",
    gap: "8px 16px",
  },
  axisRow: { display: "flex", flexDirection: "column", gap: 4 },
  axisHeader: {
    display: "flex", justifyContent: "space-between", alignItems: "baseline",
    gap: 8,
  },
  axisLabel: {
    fontSize: 11, fontWeight: 600,
    color: "var(--color-neutral-700)",
    textTransform: "uppercase", letterSpacing: "0.04em",
  },
  axisScore: {
    fontFamily: "var(--font-mono)", fontSize: 13, fontWeight: 600,
    color: "var(--color-neutral-900)",
    fontVariantNumeric: "tabular-nums",
  },
  axisTrack: {
    position: "relative",
    height: 6, background: "var(--color-neutral-200)",
    borderRadius: 3, overflow: "hidden",
  },
  axisFill: {
    height: "100%",
    transition: "width 200ms ease, background 200ms ease",
  },
  axisFloor: {
    position: "absolute", top: -2, bottom: -2, width: 1,
    background: "var(--color-neutral-500)",
    pointerEvents: "none",
  },
  axisReason: {
    fontSize: 11, color: "var(--color-neutral-600)",
    fontStyle: "italic", lineHeight: 1.4,
  },
  overallSummary: {
    padding: "10px 14px",
    background: "var(--color-neutral-50)",
    border: "1px solid var(--color-neutral-200)",
    borderRadius: 4,
    fontSize: 13, color: "var(--color-neutral-800)", lineHeight: 1.6,
    fontFamily: "var(--font-prose, serif)",
  },
  editsBlock: {
    display: "flex", flexDirection: "column", gap: 8,
  },
  editsH: {
    margin: 0,
    fontSize: 11, fontWeight: 600,
    textTransform: "uppercase", letterSpacing: "0.06em",
    color: "var(--color-neutral-500)",
  },
  editsList: { listStyle: "none", margin: 0, padding: 0, display: "flex", flexDirection: "column", gap: 4 },
  editRow: {
    display: "flex", justifyContent: "space-between", alignItems: "flex-start", gap: 12,
    padding: "8px 12px",
    background: "#fff",
    border: "1px solid var(--color-neutral-200)",
    borderRadius: 4,
  },
  editLeft: { display: "flex", flexDirection: "column", gap: 2, flex: 1, minWidth: 0 },
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
if (typeof document !== "undefined" && !document.getElementById("bf-stage1-anim")) {
  const styleEl = document.createElement("style");
  styleEl.id = "bf-stage1-anim";
  styleEl.textContent = `@keyframes bf-stage1-spin {
    from { transform: rotate(0deg); } to { transform: rotate(360deg); }
  }`;
  document.head.appendChild(styleEl);
}
