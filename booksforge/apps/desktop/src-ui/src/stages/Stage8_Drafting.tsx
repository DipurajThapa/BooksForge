/**
 * Stage 8 — Drafting (Phase B Step 4).
 *
 * Wraps `agent_run_book_pipeline` — the end-to-end:
 *   bibles (skipped if writer-supplied) → world bible (skipped if
 *   supplied) → for-each scene: drafter (with critic + polish).
 *
 * Auto-resolves models: bibles → Medium tier (qwen3.5:27b),
 * drafter → Heavy tier (qwen3.6:latest). Listens to
 * `book-pipeline:progress` events to render live stage + scene
 * progress.
 *
 * Pre-flight: detects writer-supplied bibles via `bibles_load` so the
 * panel can show "will skip" badges and accurate ETA.
 */
import { useEffect, useRef, useState } from "react";
import type { NodeInfo, OpenProjectResult } from "@booksforge/shared-types";
import {
  ipc,
  type BookPipelineProgressEvent,
  type BookSceneStageResult,
  type RunBookPipelineResult,
} from "../lib/ipc";
import { errorMessage } from "../lib/errorMessage";

interface Props {
  project:    OpenProjectResult;
  onChanged?: () => void;
  /** F1 — Called by the "Read manuscript →" CTA after a successful
   *  drafting run. EditorShell uses it to flip from Journey view
   *  to Manuscript view so the writer lands in the editor on the
   *  freshly drafted prose. Optional so the panel can be rendered
   *  standalone in tests. */
  onSwitchToManuscript?: () => void;
}

interface StageState {
  stage:    string;
  status:   string;
  summary:  string;
  current:  number;
  total:    number;
  elapsed:  number;
}

export default function Stage8_Drafting({ project, onChanged, onSwitchToManuscript }: Props) {
  // Pre-flight state
  const [nodes,          setNodes]          = useState<NodeInfo[]>([]);
  const [hasCharBible,   setHasCharBible]   = useState<boolean>(false);
  const [hasWorldBible,  setHasWorldBible]  = useState<boolean>(false);
  const [loading,        setLoading]        = useState<boolean>(true);

  // Pipeline knobs
  const [maxScenes,           setMaxScenes]           = useState<number | null>(null);
  const [skipAlreadyDrafted,  setSkipAlreadyDrafted]  = useState<boolean>(true);

  // Run state
  const [running,         setRunning]         = useState<boolean>(false);
  const [result,          setResult]          = useState<RunBookPipelineResult | null>(null);
  const [error,           setError]           = useState<string | null>(null);
  const [bibleStages,     setBibleStages]     = useState<Record<string, StageState>>({});
  const [currentScene,    setCurrentScene]    = useState<StageState | null>(null);
  const [completedEvents, setCompletedEvents] = useState<BookSceneStageResult[]>([]);

  // Wall-clock
  const startedAtRef = useRef<number | null>(null);
  const [softTick,   setSoftTick]   = useState(0);
  void softTick;
  useEffect(() => {
    if (!running) return;
    const id = window.setInterval(() => setSoftTick((t) => t + 1), 500);
    return () => window.clearInterval(id);
  }, [running]);

  const wallElapsedMs = running && startedAtRef.current
    ? Date.now() - startedAtRef.current
    : result ? result.total_elapsed_s * 1000 : 0;

  // Pre-flight load: nodes + bibles status.
  useEffect(() => {
    let cancelled = false;
    Promise.all([ipc.nodeList(), ipc.biblesLoad()])
      .then(([ns, b]) => {
        if (cancelled) return;
        setNodes(ns);
        setHasCharBible(b.has_character_bible);
        setHasWorldBible(b.has_world_bible);
      })
      .catch(() => null)
      .finally(() => { if (!cancelled) setLoading(false); });
    return () => { cancelled = true; };
  }, []);

  // Subscribe to pipeline progress while running.
  useEffect(() => {
    if (!running) return;
    let cancelled = false;
    let unlisten: (() => void) | undefined;
    (async () => {
      unlisten = await ipc.onBookPipelineProgress((e: BookPipelineProgressEvent) => {
        if (cancelled) return;
        if (e.stage === "scene-drafter-fic") {
          setCurrentScene({
            stage: e.stage, status: e.status, summary: e.summary,
            current: e.current, total: e.total, elapsed: e.elapsed_s,
          });
          if (e.status === "completed" || e.status === "skipped" || e.status === "failed") {
            setCompletedEvents((prev) => [
              ...prev,
              {
                scene_id:    "",
                scene_title: e.summary,
                status:      e.status,
                word_count:  0,
                elapsed_s:   e.elapsed_s,
                note:        "",
              },
            ]);
          }
        } else {
          setBibleStages((prev) => ({
            ...prev,
            [e.stage]: {
              stage: e.stage, status: e.status, summary: e.summary,
              current: e.current, total: e.total, elapsed: e.elapsed_s,
            },
          }));
        }
      });
    })();
    return () => { cancelled = true; unlisten?.(); };
  }, [running]);

  async function startPipeline() {
    setRunning(true); setResult(null); setError(null);
    setBibleStages({}); setCurrentScene(null); setCompletedEvents([]);
    startedAtRef.current = Date.now();
    try {
      const r = await ipc.agentRunBookPipeline({
        project_id:                  project.project_id,
        skip_already_drafted_scenes: skipAlreadyDrafted,
        max_scenes:                  maxScenes,
      });
      setResult(r);
      // Refresh nodes so the "X scenes drafted" count reflects reality.
      ipc.nodeList().then(setNodes).catch(() => null);
      // Notify the rail so its dots reflect the new drafting status.
      onChanged?.();
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setRunning(false);
    }
  }

  const sceneCount = nodes.filter((n) => n.kind === "scene").length;
  const chapterCount = nodes.filter((n) => n.kind === "chapter").length;
  const draftedScenes = nodes
    .filter((n) => n.kind === "scene" && n.word_count > 0)
    .length;

  // ── Render ──────────────────────────────────────────────────────────────

  if (loading) {
    return (
      <div style={s.root}>
        <div style={s.col}>
          <Header />
          <p style={s.muted}>Loading project state…</p>
        </div>
      </div>
    );
  }

  if (sceneCount === 0) {
    return (
      <div style={s.root}>
        <div style={s.col}>
          <Header />
          <div style={s.bannerWarn}>
            <b>No scenes to draft yet.</b> The drafter writes prose into
            existing scene nodes. Open <b>Stage 4 — Outline &amp; Structure</b>
            from the rail on the left, generate an outline, accept it, and
            come back here.
          </div>
        </div>
      </div>
    );
  }

  return (
    <div style={s.root}>
      <div style={s.col}>
        <Header />

        {!running && !result && (
          <section style={s.section}>
            <header style={s.sectionHeader}>
              <h2 style={s.sectionTitle}>Run drafting pipeline</h2>
              <p style={s.sectionHint}>
                Three stages, one click. Reads your project's brief +
                bibles, drafts prose for every scene, applies each draft
                to <code style={s.code}>scene_content</code> with a
                pre-edit snapshot.
              </p>
            </header>
            <div style={s.sectionBody}>
              <ol style={s.stagePreview}>
                <li>
                  <b>Character bible</b>
                  {hasCharBible
                    ? <span style={s.skipBadge}>writer-supplied — will skip</span>
                    : <> — Medium tier, ~2–4 min</>}
                </li>
                <li>
                  <b>World bible</b>
                  {hasWorldBible
                    ? <span style={s.skipBadge}>writer-supplied — will skip</span>
                    : <> — Medium tier, ~2–3 min</>}
                </li>
                <li>
                  <b>Scene drafter</b> — Heavy tier (<code style={s.code}>qwen3.6:latest</code>),
                  ~5–15 min per scene
                </li>
              </ol>

              {(hasCharBible || hasWorldBible) && (
                <div style={s.bannerOk}>
                  ✓ One or both bibles are already in memory. The
                  pipeline will skip the corresponding stage and save
                  ~2–5 min per skipped bible.
                </div>
              )}

              {draftedScenes > 0 && (
                <div style={s.bannerNote}>
                  This project has <b>{draftedScenes} of {sceneCount} scenes
                  </b> already drafted ({chapterCount} chapters total).
                  By default we skip those.
                </div>
              )}

              <div style={s.controls}>
                <label style={s.controlRow}>
                  <input
                    type="checkbox"
                    checked={skipAlreadyDrafted}
                    onChange={(e) => setSkipAlreadyDrafted(e.target.checked)}
                  />
                  <span>
                    <b>Skip scenes that already have prose</b>
                    <span style={s.muted}> — recommended on a re-run after manual edits.</span>
                  </span>
                </label>
                <label style={s.controlRow}>
                  <span>
                    <b>Max scenes this run</b>
                    <span style={s.muted}> — blank for "all {sceneCount}". Use 3 to test with one chapter first.</span>
                  </span>
                  <input
                    type="number" min={1} max={sceneCount}
                    placeholder={`all ${sceneCount}`}
                    value={maxScenes ?? ""}
                    onChange={(e) => {
                      const v = e.target.value.trim();
                      setMaxScenes(v === "" ? null : Math.max(1, Math.min(sceneCount, Number(v))));
                    }}
                    style={s.numberInput}
                  />
                </label>
              </div>

              {error && <div style={s.error}>{error}</div>}

              <div style={s.actionsRow}>
                <button style={s.primaryBtn} onClick={startPipeline}>
                  ✨ Run pipeline
                </button>
              </div>
            </div>
          </section>
        )}

        {running && (
          <section style={s.section}>
            <header style={s.sectionHeader}>
              <h2 style={s.sectionTitle}>
                Generating book · <b>{formatElapsed(wallElapsedMs)}</b> elapsed
              </h2>
              <p style={s.sectionHint}>
                {currentScene
                  ? <>scene {currentScene.current} / {currentScene.total}: {currentScene.summary}</>
                  : bibleStages["world-bible"]?.summary
                  ?? bibleStages["character-bible"]?.summary
                  ?? "Starting…"}
              </p>
            </header>
            <div style={s.sectionBody}>
              <div style={s.runStatus} role="status" aria-live="polite">
                <span style={s.spinner} aria-hidden="true" />
                <p style={s.muted}>
                  Cancel via the Live Run overlay (when wired). Already-applied
                  bibles + scenes are preserved on cancel.
                </p>
              </div>
              <ul style={s.stageList}>
                <StageRow label="Character bible" state={bibleStages["character-bible"]} />
                <StageRow label="World bible"     state={bibleStages["world-bible"]} />
                <StageRow
                  label={`Scenes (${currentScene?.current ?? 0}/${currentScene?.total ?? sceneCount})`}
                  state={currentScene}
                />
              </ul>
              {completedEvents.length > 0 && (
                <details style={s.details}>
                  <summary style={s.detailsSummary}>
                    Recent events ({completedEvents.length})
                  </summary>
                  <ul style={s.eventList}>
                    {completedEvents.slice(-10).map((evt, i) => (
                      <li key={`${i}-${evt.scene_title}`} style={s.eventRow}>
                        <span style={statusDotStyle(evt.status)} />
                        <span style={s.eventTitle}>{evt.scene_title}</span>
                      </li>
                    ))}
                  </ul>
                </details>
              )}
            </div>
          </section>
        )}

        {!running && result && (
          <section style={s.section}>
            <header style={s.sectionHeader}>
              <h2 style={s.sectionTitle}>
                ✓ Done in <b>{formatElapsed(result.total_elapsed_s * 1000)}</b>
              </h2>
            </header>
            <div style={s.sectionBody}>
              <ul style={s.stageList}>
                <StageRow
                  label="Character bible"
                  state={{
                    stage:   "character-bible",
                    status:  result.character_bible_status,
                    summary: result.character_bible_status,
                    current: 0, total: 0, elapsed: 0,
                  }}
                />
                <StageRow
                  label="World bible"
                  state={{
                    stage:   "world-bible",
                    status:  result.world_bible_status,
                    summary: result.world_bible_status,
                    current: 0, total: 0, elapsed: 0,
                  }}
                />
              </ul>
              <h3 style={s.subH}>Scenes ({result.scenes.length})</h3>
              <ul style={s.eventList}>
                {result.scenes.map((sc) => (
                  <li key={sc.scene_id} style={s.eventRow}>
                    <span style={statusDotStyle(sc.status)} />
                    <span style={s.eventTitle}>{sc.scene_title}</span>
                    <span style={s.eventMeta}>
                      {sc.word_count > 0 && `${sc.word_count.toLocaleString()} words · `}
                      {sc.elapsed_s.toFixed(1)}s · {sc.status}
                    </span>
                  </li>
                ))}
              </ul>
              <div style={s.actionsRow}>
                <button style={s.ghostBtn} onClick={() => setResult(null)}>
                  Run again
                </button>
                {onSwitchToManuscript && (
                  <button
                    style={s.primaryBtn}
                    onClick={onSwitchToManuscript}
                    title="Open the manuscript view with the just-drafted prose"
                  >
                    📖 Read manuscript →
                  </button>
                )}
              </div>
            </div>
          </section>
        )}
      </div>
    </div>
  );
}

// ── Subcomponents ───────────────────────────────────────────────────────────

function Header() {
  return (
    <header style={s.header}>
      <p style={s.stageNum}>Stage 5 of 6</p>
      <h1 style={s.title}>Drafting</h1>
      <p style={s.lede}>
        Prose for every scene. Character + world bibles drive voice and
        continuity; the drafter reads them per scene. Quality scoring +
        revision queue land in Phase B Step 6.
      </p>
    </header>
  );
}

function StageRow({ label, state }: { label: string; state: StageState | null | undefined }) {
  const status = state?.status ?? "pending";
  return (
    <li style={s.stageRow}>
      <span style={statusDotStyle(status)} aria-hidden="true" />
      <span style={s.stageLabel}>{label}</span>
      <span style={s.stageStatus}>{status}</span>
      {state && state.elapsed > 0 && (
        <span style={s.stageElapsed}>{state.elapsed.toFixed(1)}s</span>
      )}
    </li>
  );
}

function statusDotStyle(status: string): React.CSSProperties {
  const color =
    status === "completed" ? "var(--color-green-500, #22c55e)" :
    status === "failed"    ? "var(--color-red-500, #ef4444)"   :
    status === "skipped"   ? "var(--color-neutral-400)"        :
    status === "running"   ? "var(--color-amber-500, #f59e0b)" :
    "var(--color-neutral-200)";
  return {
    width: 10, height: 10, borderRadius: "50%",
    background: color, flexShrink: 0,
  };
}

function formatElapsed(ms: number): string {
  const total = Math.floor(ms / 1000);
  if (total < 60) return `${total}s`;
  const m = Math.floor(total / 60);
  const s_ = total % 60;
  return `${m}m ${s_.toString().padStart(2, "0")}s`;
}

// Inject spinner keyframes once (HMR-safe).
if (typeof document !== "undefined" && !document.getElementById("bf-stage8-anim")) {
  const styleEl = document.createElement("style");
  styleEl.id = "bf-stage8-anim";
  styleEl.textContent = `@keyframes bf-stage8-spin {
    from { transform: rotate(0deg); } to { transform: rotate(360deg); }
  }`;
  document.head.appendChild(styleEl);
}

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
    margin: 0, fontSize: 15, fontWeight: 600,
    color: "var(--color-neutral-900)",
  },
  sectionHint: {
    margin: "4px 0 0", fontSize: 12,
    color: "var(--color-neutral-600)", lineHeight: 1.5,
  },
  sectionBody: { padding: 16, display: "flex", flexDirection: "column", gap: 12 },
  bannerWarn: {
    padding: "10px 14px",
    background: "rgba(245,158,11,0.08)",
    border: "1px solid rgba(245,158,11,0.3)",
    borderRadius: 6,
    fontSize: 13, color: "var(--color-neutral-800)", lineHeight: 1.6,
  },
  bannerOk: {
    padding: "10px 14px",
    background: "rgba(34,197,94,0.06)",
    border: "1px solid rgba(34,197,94,0.25)",
    borderRadius: 6,
    fontSize: 13, color: "var(--color-neutral-800)", lineHeight: 1.6,
  },
  bannerNote: {
    padding: "10px 14px",
    background: "var(--color-neutral-50)",
    border: "1px solid var(--color-neutral-200)",
    borderRadius: 4,
    fontSize: 12, color: "var(--color-neutral-700)", lineHeight: 1.6,
  },
  stagePreview: {
    margin: 0, paddingLeft: 24,
    fontSize: 13, color: "var(--color-neutral-800)", lineHeight: 1.8,
  },
  skipBadge: {
    marginLeft: 8,
    fontSize: 11, color: "var(--color-green-700, #15803d)",
    background: "rgba(34,197,94,0.08)",
    border: "1px solid rgba(34,197,94,0.35)",
    borderRadius: 999,
    padding: "1px 8px",
    fontWeight: 500,
  },
  controls: { display: "flex", flexDirection: "column", gap: 8 },
  controlRow: {
    display: "flex", alignItems: "center", gap: 8,
    fontSize: 13, color: "var(--color-neutral-800)",
  },
  numberInput: {
    width: 80,
    padding: "4px 8px",
    border: "1px solid var(--color-neutral-300)", borderRadius: 4,
    fontFamily: "var(--font-mono)", fontSize: 12,
    marginLeft: "auto",
  },
  error: {
    padding: "8px 12px",
    background: "rgba(220,38,38,0.06)",
    color: "var(--color-red-700, #b91c1c)",
    border: "1px solid rgba(220,38,38,0.25)",
    borderRadius: 4, fontFamily: "var(--font-mono)", fontSize: 12,
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
  ghostBtn: {
    padding: "10px 16px",
    background: "transparent", color: "var(--color-neutral-700)",
    border: "1px solid var(--color-neutral-300)", borderRadius: 5,
    fontSize: 13, fontWeight: 500, cursor: "pointer",
    fontFamily: "var(--font-ui)",
  },
  runStatus: {
    display: "flex", alignItems: "flex-start", gap: 12,
    padding: 12,
    background: "var(--color-neutral-50)",
    border: "1px solid var(--color-neutral-200)",
    borderRadius: 6,
  },
  spinner: {
    width: 18, height: 18, flexShrink: 0,
    borderRadius: "50%",
    border: "2px solid var(--color-neutral-300)",
    borderTopColor: "var(--color-amber-600)",
    animation: "bf-stage8-spin 0.9s linear infinite",
    marginTop: 2,
  },
  stageList: {
    listStyle: "none", margin: 0, padding: 0,
    display: "flex", flexDirection: "column", gap: 4,
  },
  stageRow: {
    display: "flex", alignItems: "center", gap: 8,
    padding: "6px 12px",
    border: "1px solid var(--color-neutral-200)", borderRadius: 4,
    background: "#fff", fontSize: 13,
  },
  stageLabel: { flex: 1, color: "var(--color-neutral-900)" },
  stageStatus: { color: "var(--color-neutral-500)", fontSize: 11, fontFamily: "var(--font-mono)" },
  stageElapsed: { color: "var(--color-neutral-500)", fontSize: 11, fontVariantNumeric: "tabular-nums" },
  subH: {
    margin: "8px 0 0", fontSize: 11, fontWeight: 600,
    color: "var(--color-neutral-600)",
    textTransform: "uppercase", letterSpacing: "0.06em",
  },
  details: { fontSize: 12, color: "var(--color-neutral-600)" },
  detailsSummary: { cursor: "pointer", padding: "4px 0" },
  eventList: {
    listStyle: "none", margin: 0, padding: 0,
    display: "flex", flexDirection: "column", gap: 2,
    maxHeight: 240, overflow: "auto",
  },
  eventRow: {
    display: "flex", alignItems: "center", gap: 8,
    padding: "4px 8px", fontSize: 12,
  },
  eventTitle: { flex: 1, color: "var(--color-neutral-900)" },
  eventMeta: { color: "var(--color-neutral-500)", fontSize: 11, fontFamily: "var(--font-mono)" },
  code: {
    fontFamily: "var(--font-mono)", fontSize: 11,
    padding: "1px 4px",
    background: "var(--color-neutral-100)", borderRadius: 3,
  },
};
