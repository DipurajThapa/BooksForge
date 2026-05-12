/**
 * Stage 2 — Audience Map (Phase B Step 6 + Phase C audience mapper).
 *
 * Two parts:
 *   1. Audience subset of the brief — `audience`, `theme_keywords`,
 *      `forbidden_tropes`, `comp_titles_or_authors`. Saves to
 *      `book:project_brief` via `projectBriefSave`.
 *   2. AI-generated Reader Expectation Map — runs the `audience-mapper`
 *      agent which reads the brief and writes a structured map to
 *      `book:audience_map`. Surfaces genre_expectations,
 *      emotional_promises, recommended_themes, tropes_to_avoid,
 *      pacing_expectation, and an editor's note. Downstream agents
 *      (scene drafter, polish stack) read the map via
 *      `creative_profile`.
 */
import { useEffect, useState } from "react";
import type { OpenProjectResult } from "@booksforge/shared-types";
import { ipc } from "../lib/ipc";
import { errorMessage } from "../lib/errorMessage";
import { useToast } from "../components/ToastProvider";
import type { StageId } from "../components/StageRail";

// Mirror of `booksforge_domain::ReaderExpectationMap`. Hand-typed
// because we carry it as a JSON string in `proposal_json`.
type PacingExpectation = "slow_build" | "page_turner" | "episodic" | "lyrical";
interface ReaderExpectationMap {
  genre_expectations:   string[];
  genre_anti_patterns?: string[];
  emotional_promises:   string[];
  recommended_themes:   string[];
  recommended_tropes?:  string[];
  tropes_to_avoid:      string[];
  pacing_expectation?:  PacingExpectation;
  overall_note?:        string;
}

type MapState =
  | { kind: "idle";    map: ReaderExpectationMap | null }   // null = never generated
  | { kind: "running"; startedAt: number }
  | { kind: "ready";   map: ReaderExpectationMap }
  | { kind: "error";   message: string };

interface Props {
  project:    OpenProjectResult;
  onChanged?: () => void;
  /** F5 — Called by "Save & continue" after a successful save so the
   *  StageRail advances to the next stage. Optional. */
  onAdvance?: () => void;
  /** F12 — Called by the "Go to Stage N" CTA in the missing-prereq
   *  banner. EditorShell wires it to its stage-jump handler so the
   *  writer doesn't have to navigate the rail by hand. Optional so
   *  the panel can still render in isolation (tests). */
  onJumpToStage?: (id: StageId) => void;
}

interface AudienceForm {
  audience:               string;
  comp_titles_or_authors: string;  // textarea, one per line
  theme_keywords:         string;  // textarea — "what readers expect / want"
  forbidden_tropes:       string;  // textarea — "what to avoid"
  // We keep the rest of the brief intact when saving by stashing the
  // full loaded brief here and merging on save.
  _fullBrief?:            Record<string, unknown> | null;
}

const EMPTY: AudienceForm = {
  audience:               "",
  comp_titles_or_authors: "",
  theme_keywords:         "",
  forbidden_tropes:       "",
};

export default function Stage2_Audience({ project, onChanged, onAdvance, onJumpToStage }: Props) {
  void project;
  const [form,         setForm]         = useState<AudienceForm>(EMPTY);
  const [loading,      setLoading]      = useState(true);
  const [saving,       setSaving]       = useState(false);
  const [loaded,       setLoaded]       = useState(false);
  const [error,        setError]        = useState<string | null>(null);
  const [savedHint,    setSavedHint]    = useState<string | null>(null);
  const [mapState,     setMapState]     = useState<MapState>({ kind: "idle", map: null });
  const toast = useToast();

  // Load both the brief AND any existing audience_map on mount.
  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        // memoryList scoped to book to find audience_map without a
        // separate IPC. Falls back to null when no map saved.
        const memBook = await ipc.memoryList({ scope: "book" }).catch(() => []);
        if (!cancelled) {
          const mapEntry = memBook.find((m) => m.key === "audience_map");
          if (mapEntry) {
            try {
              const parsed = JSON.parse(mapEntry.value_json) as ReaderExpectationMap;
              setMapState({ kind: "idle", map: parsed });
            } catch {
              /* keep null; let the writer regenerate */
            }
          }
        }
      } catch {
        /* non-fatal */
      }
    })();
    return () => { cancelled = true; };
  }, []);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const r = await ipc.projectBriefLoad();
        if (cancelled) return;
        setLoaded(r.loaded);
        if (r.loaded && r.brief_json && typeof r.brief_json === "object") {
          const b = r.brief_json as Partial<{
            audience:               string;
            comp_titles_or_authors: string[];
            theme_keywords:         string[];
            forbidden_tropes:       string[];
          }> & Record<string, unknown>;
          setForm({
            audience:               asStr(b.audience),
            comp_titles_or_authors: (b.comp_titles_or_authors ?? []).join("\n"),
            theme_keywords:         (b.theme_keywords         ?? []).join("\n"),
            forbidden_tropes:       (b.forbidden_tropes       ?? []).join("\n"),
            _fullBrief:             b,
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

  function update<K extends keyof AudienceForm>(k: K, v: AudienceForm[K]) {
    setForm((f) => ({ ...f, [k]: v }));
    setError(null); setSavedHint(null);
  }

  /**
   * Run the audience-mapper agent against the saved brief. Saves
   * audience fields first so the agent reads the writer's current
   * intent. The IPC layer persists the resulting
   * `ReaderExpectationMap` to `book:audience_map` — we don't have to
   * do anything else on the frontend after a successful run.
   */
  async function handleGenerateMap() {
    if (!loaded) {
      const msg = "Save the brief first (Stage 1 → Book Setup).";
      setError(msg);
      toast.push({ severity: "warning", body: msg });
      return;
    }
    // Save audience-form changes first so the agent sees them.
    // handleSave returns false on failure and already surfaces a toast.
    const ok = await handleSave();
    if (!ok) return;
    setMapState({ kind: "running", startedAt: Date.now() });
    try {
      const r = await ipc.agentRunAudienceMapper({
        project_id: project.project_id,
        model:      null,  // Light tier auto-resolves
      });
      if (r.status !== "completed" || !r.proposal_json) {
        setMapState({
          kind: "error",
          message: r.error ?? `Agent returned status: ${r.status}`,
        });
        return;
      }
      const map = JSON.parse(r.proposal_json) as ReaderExpectationMap;
      setMapState({ kind: "ready", map });
      onChanged?.();
    } catch (e) {
      setMapState({ kind: "error", message: errorMessage(e) });
    }
  }

  /**
   * Returns true on success, false on failure. Errors surface via
   * toast so the writer notices even after they've scrolled past
   * the inline banner.
   */
  async function handleSave(): Promise<boolean> {
    if (!form._fullBrief) {
      const msg = "Cannot save — the brief hasn't loaded. Visit Stage 1 first.";
      setError(msg);
      toast.push({ severity: "warning", body: msg });
      return false;
    }
    setSaving(true); setError(null);
    try {
      const lines = (s: string) =>
        s.split("\n").map((l) => l.trim()).filter(Boolean);
      // Merge audience-map fields into the existing brief so we don't
      // clobber Stage 1's premise / key_promises / etc.
      const payload = {
        ...form._fullBrief,
        audience:               form.audience.trim() || "general readers",
        comp_titles_or_authors: lines(form.comp_titles_or_authors),
        theme_keywords:         lines(form.theme_keywords),
        forbidden_tropes:       lines(form.forbidden_tropes),
      };
      const r = await ipc.projectBriefSave({ brief_json: payload });
      setLoaded(true);
      setForm((f) => ({ ...f, _fullBrief: r.brief_json as Record<string, unknown> }));
      setSavedHint("Audience map saved. Every agent run after this picks up the new values.");
      onChanged?.();
      return true;
    } catch (e) {
      const msg = errorMessage(e);
      setError(msg);
      toast.push({
        severity: "error",
        title: "Audience save failed",
        body: msg,
      });
      return false;
    } finally {
      setSaving(false);
    }
  }

  /**
   * F5 — Save the audience map and advance to Stage 3 (Bibles) on
   * success. Stays on this stage if the save fails.
   */
  async function handleSaveAndContinue() {
    const ok = await handleSave();
    if (ok) {
      toast.push({
        severity: "success",
        body: "Audience saved. Next: Bibles (optional).",
      });
      onAdvance?.();
    }
  }

  // ── Render ──────────────────────────────────────────────────────────────

  return (
    <div style={s.root}>
      <div style={s.col}>
        <header style={s.header}>
          <p style={s.stageNum}>Stage 2 of 6</p>
          <h1 style={s.title}>Audience Map</h1>
          <p style={s.lede}>
            Who is this book for, and what do they expect? The audience
            map drives the AI's pacing, voice, and emotional targets.
            A book without a named reader becomes generic. Saved to the
            project brief.
          </p>
        </header>

        {!loading && loaded && (
          <div style={s.bannerOk}>
            ✓ Audience map loaded from the project brief. Edit any field
            below and click Save — every agent run after that picks up
            the new values.
          </div>
        )}
        {!loading && !loaded && (
          <div style={s.bannerWarn}>
            <div style={{ marginBottom: 8 }}>
              <b>No brief saved yet.</b> The audience map saves into the
              same brief that Stage 1 owns, so complete Stage 1 first.
            </div>
            {onJumpToStage && (
              <button
                style={s.jumpBtn}
                onClick={() => onJumpToStage("setup")}
              >
                ← Go to Stage 1 — Book Setup
              </button>
            )}
          </div>
        )}

        {loading && <p style={s.muted}>Loading audience map…</p>}

        {!loading && loaded && (
          <>
            <section style={s.section}>
              <header style={s.sectionHeader}>
                <h2 style={s.sectionTitle}>Reader profile</h2>
                <p style={s.sectionHint}>
                  Who picks this book up off the shelf, library hold,
                  or Kindle "for you" list?
                </p>
              </header>
              <div style={s.sectionBody}>
                <Field label="Primary audience">
                  <input style={s.input} value={form.audience}
                    onChange={(e) => update("audience", e.target.value)}
                    placeholder="adult literary readers" />
                </Field>
              </div>
            </section>

            <section style={s.section}>
              <header style={s.sectionHeader}>
                <h2 style={s.sectionTitle}>What readers expect</h2>
                <p style={s.sectionHint}>
                  One per line. The drafter weaves these in across chapters.
                </p>
              </header>
              <div style={s.sectionBody}>
                <Field label="Reader expectations / themes" hint="What the reader hopes to feel, learn, or witness.">
                  <textarea
                    style={{ ...s.input, minHeight: 90, fontFamily: "var(--font-prose, serif)" }}
                    value={form.theme_keywords}
                    onChange={(e) => update("theme_keywords", e.target.value)}
                    placeholder={
                      "the weight of accumulated time\n" +
                      "a marriage holding a thing the other person doesn't know\n" +
                      "inheritance as silence, not as wealth"
                    }
                  />
                </Field>
              </div>
            </section>

            <section style={s.section}>
              <header style={s.sectionHeader}>
                <h2 style={s.sectionTitle}>What to avoid</h2>
                <p style={s.sectionHint}>
                  Tropes or patterns the writer wants the AI to steer clear of.
                  AI-tell phrases like "delve" / "tapestry" go in the project
                  vocab; this is for narrative tropes.
                </p>
              </header>
              <div style={s.sectionBody}>
                <Field label="Forbidden tropes" hint="One per line.">
                  <textarea
                    style={{ ...s.input, minHeight: 90, fontFamily: "var(--font-prose, serif)" }}
                    value={form.forbidden_tropes}
                    onChange={(e) => update("forbidden_tropes", e.target.value)}
                    placeholder={
                      "love-triangle\n" +
                      "chosen-one prophecy\n" +
                      "amnesia plot twist"
                    }
                  />
                </Field>
              </div>
            </section>

            <section style={s.section}>
              <header style={s.sectionHeader}>
                <h2 style={s.sectionTitle}>Positioning</h2>
                <p style={s.sectionHint}>
                  Comparable titles or authors. Used for positioning, not
                  for imitation. AI is forbidden from copying their plots
                  or unique phrases.
                </p>
              </header>
              <div style={s.sectionBody}>
                <Field label="Comparable books or authors" hint="One per line.">
                  <textarea
                    style={{ ...s.input, minHeight: 80, fontFamily: "var(--font-prose, serif)" }}
                    value={form.comp_titles_or_authors}
                    onChange={(e) => update("comp_titles_or_authors", e.target.value)}
                    placeholder={"Marilynne Robinson — Gilead\nCormac McCarthy — The Road"}
                  />
                </Field>
              </div>
            </section>

            {error && <div style={s.error}>{error}</div>}
            {savedHint && <div style={s.savedHint}>{savedHint}</div>}

            <div style={s.actionsRow}>
              <button
                style={s.ghostBtn}
                onClick={() => { void handleSave(); }}
                disabled={saving || !loaded}
                title="Save without leaving this stage"
              >
                {saving ? "Saving…" : "Save"}
              </button>
              <button
                style={{ ...s.primaryBtn, ...(saving ? s.primaryBtnBusy : {}) }}
                onClick={handleSaveAndContinue}
                disabled={saving || !loaded}
                title="Save the audience map and move to Stage 3 — Bibles"
              >
                {saving ? "Saving…" : "Save & continue →"}
              </button>
            </div>

            <MapSection
              state={mapState}
              loaded={loaded}
              onGenerate={handleGenerateMap}
              onClear={() => setMapState({ kind: "idle", map: null })}
            />
          </>
        )}
      </div>
    </div>
  );
}

// ── Subcomponents ───────────────────────────────────────────────────────────

/**
 * Renders the audience-mapper's output. Idle without a map shows a
 * "Generate" CTA; Idle with a previously-generated map shows the map
 * + Regenerate. Running shows a spinner. Ready / Error are similar to
 * Stage 1's ScoreSection.
 */
function MapSection({
  state, loaded, onGenerate, onClear,
}: {
  state:      MapState;
  loaded:     boolean;
  onGenerate: () => void;
  onClear:    () => void;
}) {
  const currentMap: ReaderExpectationMap | null =
    state.kind === "ready" ? state.map :
    state.kind === "idle"  ? state.map :
    null;

  return (
    <section style={s.section}>
      <header style={s.sectionHeader}>
        <h2 style={s.sectionTitle}>Reader Expectation Map (AI-generated)</h2>
        <p style={s.sectionHint}>
          The <code style={s.code}>audience-mapper</code> agent reads your
          brief and emits a structured map. Persisted to
          {" "}<code style={s.code}>book:audience_map</code> so the
          scene drafter + polish stack braid these signals through every
          chapter.
        </p>
      </header>
      <div style={s.sectionBody}>
        {state.kind === "running" && (
          <div style={s.mapRunning}>
            <span style={s.mapSpinner} aria-hidden="true" />
            <span>Running on Light tier (qwen3.5:9b). Expected ~30–60 s.</span>
          </div>
        )}
        {state.kind === "error" && (
          <>
            <div style={s.error}>{state.message}</div>
            <div style={{ display: "flex", justifyContent: "flex-end" }}>
              <button style={s.smallBtn} onClick={onClear}>Dismiss</button>
            </div>
          </>
        )}
        {currentMap && (
          <MapView map={currentMap} />
        )}
        <div style={s.actionsRow}>
          <button
            style={s.primaryBtn}
            onClick={onGenerate}
            disabled={!loaded || state.kind === "running"}
            title={!loaded ? "Save the brief first (Stage 1)" : undefined}
          >
            {state.kind === "running"
              ? "Generating…"
              : currentMap
              ? "✨ Regenerate map"
              : "✨ Generate Reader Expectation Map"}
          </button>
        </div>
      </div>
    </section>
  );
}

function MapView({ map }: { map: ReaderExpectationMap }) {
  return (
    <div style={s.mapView}>
      {map.overall_note && (
        <p style={s.mapOverallNote}>{map.overall_note}</p>
      )}
      <div style={s.mapGrid}>
        <MapList label="Genre expectations"  items={map.genre_expectations} tone="ok" />
        <MapList label="Genre anti-patterns" items={map.genre_anti_patterns ?? []} tone="warn" />
        <MapList label="Emotional promises"  items={map.emotional_promises}  tone="ok" />
        <MapList label="Recommended themes"  items={map.recommended_themes}  tone="ok" />
        <MapList label="Recommended tropes"  items={map.recommended_tropes ?? []} tone="ok" />
        <MapList label="Tropes to avoid"     items={map.tropes_to_avoid}     tone="warn" />
      </div>
      <div style={s.mapPacingRow}>
        <span style={s.mapPacingLabel}>Pacing:</span>
        <span style={s.mapPacingValue}>
          {pacingLabel(map.pacing_expectation ?? "slow_build")}
        </span>
      </div>
    </div>
  );
}

function MapList({ label, items, tone }: {
  label: string; items: string[]; tone: "ok" | "warn";
}) {
  if (items.length === 0) return null;
  return (
    <div style={s.mapListBlock}>
      <h4 style={s.mapListLabel}>{label}</h4>
      <ul style={s.mapList}>
        {items.map((item, i) => (
          <li key={i} style={tone === "warn" ? s.mapItemWarn : s.mapItem}>
            <span style={tone === "warn" ? s.mapDotWarn : s.mapDot} aria-hidden="true" />
            {item}
          </li>
        ))}
      </ul>
    </div>
  );
}

function pacingLabel(p: PacingExpectation): string {
  switch (p) {
    case "slow_build":  return "Slow build — literary, deferred payoff";
    case "page_turner": return "Page-turner — high velocity, chapter-level resolution";
    case "episodic":    return "Episodic — chapters stand alone";
    case "lyrical":     return "Lyrical — voice-led, image-dense";
  }
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

function asStr(v: unknown): string {
  if (typeof v === "string") return v;
  if (v == null) return "";
  return String(v);
}

const s: Record<string, React.CSSProperties> = {
  root: {
    height: "100%", overflow: "auto",
    padding: "32px 24px 48px",
    display: "flex", justifyContent: "center",
    fontFamily: "var(--font-ui)",
  },
  col: { width: "min(760px, 100%)", display: "flex", flexDirection: "column", gap: 16 },
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
    borderRadius: 6, overflow: "hidden",
  },
  sectionHeader: {
    padding: "12px 16px",
    background: "var(--color-neutral-50)",
    borderBottom: "1px solid var(--color-neutral-200)",
  },
  sectionTitle: {
    // F9 — Section titles are 15px 600 mixed-case (matching Stage 4
    // and downstream). Previous 11px uppercase collided visually
    // with the 11px uppercase field labels below, leaving the form
    // with no hierarchy. Title now reads as a real heading.
    margin: 0, fontSize: 15, fontWeight: 600,
    color: "var(--color-neutral-900)",
  },
  sectionHint: {
    margin: "4px 0 0", fontSize: 12,
    color: "var(--color-neutral-600)", lineHeight: 1.5,
  },
  sectionBody: { padding: 16, display: "flex", flexDirection: "column", gap: 12 },
  field: { display: "flex", flexDirection: "column", gap: 4 },
  fieldLabel: {
    fontSize: 11, fontWeight: 600,
    color: "var(--color-neutral-700)",
    textTransform: "uppercase", letterSpacing: "0.04em",
  },
  fieldHint: { fontSize: 11, color: "var(--color-neutral-500)" },
  input: {
    width: "100%", boxSizing: "border-box",
    padding: "8px 12px",
    border: "1px solid var(--color-neutral-300)", borderRadius: 4,
    background: "#fff", color: "var(--color-neutral-900)",
    fontFamily: "var(--font-ui)", fontSize: 14, outline: "none",
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
  // F12 — Inline jump button rendered inside warning banners that
  // point the writer at a missing-prereq stage. Reads as a primary
  // amber CTA so it stands out from the prose.
  jumpBtn: {
    padding: "8px 14px",
    background: "var(--color-amber-600)", color: "#fff",
    border: "none", borderRadius: 4,
    fontSize: 13, fontWeight: 600, cursor: "pointer",
    fontFamily: "var(--font-ui)",
  },
  code: {
    fontFamily: "var(--font-mono)", fontSize: 11,
    padding: "1px 4px",
    background: "var(--color-neutral-100)", borderRadius: 3,
  },
  // Map-section styles -----------------------------------------------------
  mapRunning: {
    display: "flex", alignItems: "center", gap: 10,
    padding: "12px 14px",
    background: "var(--color-neutral-50)",
    border: "1px solid var(--color-neutral-200)",
    borderRadius: 4,
    fontSize: 13, color: "var(--color-neutral-700)",
  },
  mapSpinner: {
    width: 14, height: 14, flexShrink: 0,
    borderRadius: "50%",
    border: "2px solid var(--color-neutral-300)",
    borderTopColor: "var(--color-amber-600)",
    animation: "bf-stage2-spin 0.9s linear infinite",
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
  mapView: { display: "flex", flexDirection: "column", gap: 12 },
  mapOverallNote: {
    margin: 0,
    padding: "10px 14px",
    background: "var(--color-neutral-50)",
    border: "1px solid var(--color-neutral-200)",
    borderRadius: 4,
    fontSize: 13, color: "var(--color-neutral-800)",
    lineHeight: 1.6,
    fontFamily: "var(--font-prose, serif)",
  },
  mapGrid: {
    display: "grid",
    gridTemplateColumns: "1fr 1fr",
    gap: "8px 16px",
  },
  mapListBlock: { display: "flex", flexDirection: "column", gap: 4 },
  mapListLabel: {
    margin: 0,
    fontSize: 11, fontWeight: 600,
    textTransform: "uppercase", letterSpacing: "0.06em",
    color: "var(--color-neutral-600)",
  },
  mapList: { listStyle: "none", margin: 0, padding: 0, display: "flex", flexDirection: "column", gap: 2 },
  mapItem: {
    display: "flex", alignItems: "flex-start", gap: 8,
    fontSize: 12, color: "var(--color-neutral-800)", lineHeight: 1.5,
  },
  mapItemWarn: {
    display: "flex", alignItems: "flex-start", gap: 8,
    fontSize: 12, color: "var(--color-neutral-700)", lineHeight: 1.5,
  },
  mapDot: {
    width: 4, height: 4, borderRadius: "50%",
    background: "var(--color-green-500, #22c55e)",
    flexShrink: 0, marginTop: 7,
  },
  mapDotWarn: {
    width: 4, height: 4, borderRadius: "50%",
    background: "var(--color-red-500, #ef4444)",
    flexShrink: 0, marginTop: 7,
  },
  mapPacingRow: {
    display: "flex", alignItems: "center", gap: 8,
    padding: "8px 12px",
    background: "var(--color-amber-50, #fffbeb)",
    border: "1px solid var(--color-amber-200, #fde68a)",
    borderRadius: 4,
    fontSize: 12,
  },
  mapPacingLabel: {
    fontWeight: 600,
    color: "var(--color-amber-700, #b45309)",
    textTransform: "uppercase",
    letterSpacing: "0.06em",
    fontSize: 10,
  },
  mapPacingValue: { color: "var(--color-neutral-800)" },
};

// Inject the spinner keyframes once on module load (HMR-safe).
if (typeof document !== "undefined" && !document.getElementById("bf-stage2-anim")) {
  const styleEl = document.createElement("style");
  styleEl.id = "bf-stage2-anim";
  styleEl.textContent = `@keyframes bf-stage2-spin {
    from { transform: rotate(0deg); } to { transform: rotate(360deg); }
  }`;
  document.head.appendChild(styleEl);
}
