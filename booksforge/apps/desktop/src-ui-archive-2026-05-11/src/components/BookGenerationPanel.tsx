/**
 * BookGenerationPanel — one-click "Generate the book" experience.
 *
 * Wraps `agent_run_book_pipeline`, which chains:
 *   1. character-bible (auto-saved to entity memory)
 *   2. world-bible    (auto-saved to book memory)
 *   3. for each scene in document order:
 *      scene-drafter-fic → applied to scene_content
 *
 * The user does NOT pick a model. The backend auto-resolves per-agent
 * tier from the installed Ollama models (Medium for bibles, Heavy for
 * drafter). Progress lands via `book-pipeline:progress` events.
 */
import React, { useEffect, useRef, useState } from "react";
import { useDialogA11y } from "../lib/useDialogA11y";
import { ipc } from "../lib/ipc";
import { errorMessage } from "../lib/errorMessage";
import type {
  BookPipelineProgressEvent,
  BookSceneStageResult,
  RunBookPipelineResult,
} from "../lib/ipc";

interface Props {
  projectId: string;
  /** Total scene count surfaced in the editor's binder, used to size
   *  the per-scene progress bar before the backend starts emitting. */
  sceneCount: number;
  onClose:    () => void;
  /** Fires after each successful scene apply so the editor can refresh
   *  its node list + reload the currently-open scene's prose. */
  onSceneApplied?: () => void;
}

interface StageState {
  stage:    string;
  status:   string;
  summary:  string;
  current:  number;
  total:    number;
  elapsed:  number;
}

export default function BookGenerationPanel({ projectId, sceneCount, onClose, onSceneApplied }: Props) {
  const { dialogProps, titleId } = useDialogA11y(onClose);

  // The user's two only knobs.
  const [maxScenes,             setMaxScenes]             = useState<number | null>(null);
  const [skipAlreadyDrafted,    setSkipAlreadyDrafted]    = useState<boolean>(true);

  const [running,    setRunning]    = useState(false);
  const [error,      setError]      = useState<string | null>(null);
  const [result,     setResult]     = useState<RunBookPipelineResult | null>(null);
  // Detect writer-supplied bibles so we can show "will skip" badges
  // and accurately set the user's expectation of pipeline duration.
  const [biblesStatus, setBiblesStatus] = useState<{ has_character: boolean; has_world: boolean }>({
    has_character: false, has_world: false,
  });
  useEffect(() => {
    ipc.biblesLoad().then((r) => {
      setBiblesStatus({ has_character: r.has_character_bible, has_world: r.has_world_bible });
    }).catch(() => null);
  }, []);
  // History of stage events. The latest of each stage wins; for
  // scene-drafter-fic we keep the latest per-scene event in `currentScene`.
  const [bibleStages,   setBibleStages]   = useState<Record<string, StageState>>({});
  const [currentScene,  setCurrentScene]  = useState<StageState | null>(null);
  const [completedScenes, setCompletedScenes] = useState<BookSceneStageResult[]>([]);

  // Wall-clock since Run was clicked. Survives event silence so the
  // user always sees the timer ticking.
  const startedAtRef = useRef<number | null>(null);
  const [softTick,   setSoftTick]   = useState(0);
  useEffect(() => {
    if (!running) return;
    const id = window.setInterval(() => setSoftTick((t) => t + 1), 500);
    return () => window.clearInterval(id);
  }, [running]);
  // The softTick state is incremented purely to trigger re-renders so
  // wallElapsedMs ticks visibly. Reading it here makes the dependency
  // explicit (and shuts up the unused-variable lint).
  void softTick;
  const wallElapsedMs = running && startedAtRef.current
    ? Date.now() - startedAtRef.current
    : (result ? result.total_elapsed_s * 1000 : 0);

  // Subscribe to `book-pipeline:progress` while running.
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
          // When a scene completes, surface its summary in the
          // completed list (the result payload arrives only after
          // the WHOLE pipeline finishes, but per-scene events fire live).
          if (e.status === "completed" || e.status === "skipped" || e.status === "failed") {
            setCompletedScenes((prev) => [
              ...prev,
              {
                scene_id:    "",         // not in the event; ok for live row
                scene_title: e.summary,  // contains the title in "Drafting scene N: title"
                status:      e.status,
                word_count:  0,
                elapsed_s:   e.elapsed_s,
                note:        "",
              },
            ]);
            // Tell the parent so the binder + open scene refresh.
            if (e.status === "completed") onSceneApplied?.();
          }
        } else {
          // bible stages
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
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [running, onSceneApplied]);

  async function handleRun() {
    setRunning(true);
    setError(null);
    setResult(null);
    setBibleStages({});
    setCurrentScene(null);
    setCompletedScenes([]);
    startedAtRef.current = Date.now();
    try {
      const r = await ipc.agentRunBookPipeline({
        project_id: projectId,
        skip_already_drafted_scenes: skipAlreadyDrafted,
        max_scenes: maxScenes,
      });
      setResult(r);
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setRunning(false);
    }
  }

  const cap = maxScenes ?? sceneCount;
  const sceneProgress = currentScene
    ? `${currentScene.current} / ${currentScene.total}`
    : `0 / ${cap}`;

  return (
    <div style={s.overlay} role="presentation">
      <div {...dialogProps} style={s.dialog}>
        <header style={s.header}>
          <strong id={titleId}>Generate Book</strong>
          <button style={s.close} onClick={onClose} disabled={running} aria-label="Close panel">✕</button>
        </header>

        <div style={s.body}>
          {!running && !result && (
            <>
              <p style={s.intro}>
                BooksForge will run the full pipeline against this project:
              </p>
              <ol style={s.steps}>
                <li>
                  <b>Character bible</b>{biblesStatus.has_character
                    ? <span style={s.skipBadge}>writer-supplied — will skip</span>
                    : <> — protagonists, antagonists, voices (Medium tier)</>}
                </li>
                <li>
                  <b>World bible</b>{biblesStatus.has_world
                    ? <span style={s.skipBadge}>writer-supplied — will skip</span>
                    : <> — locations, social rules, sensory palette (Medium tier)</>}
                </li>
                <li><b>Scene drafter</b> — prose for every scene in order (Heavy tier)</li>
              </ol>
              {(biblesStatus.has_character || biblesStatus.has_world) && (
                <p style={s.skipHint}>
                  ✓ One or both bibles are already in memory (added via
                  <b> ⋯ → Bibles </b>or a previous AI run). The pipeline
                  will skip the corresponding stage and save you ~2–5 min
                  per skipped bible.
                </p>
              )}
              <p style={s.note}>
                Models are picked automatically from your installed Ollama tags.
                Expect <b>~1–2 min per bible</b> and <b>~5–15 min per scene</b> on
                qwen3.6-class hardware. The pipeline can be cancelled mid-scene;
                everything already applied is preserved.
              </p>

              <div style={s.controls}>
                <label style={s.controlRow}>
                  <input
                    type="checkbox"
                    checked={skipAlreadyDrafted}
                    onChange={(e) => setSkipAlreadyDrafted(e.target.checked)}
                  />
                  <span>
                    <b>Skip scenes that already have prose</b>
                    <span style={s.controlHint}>
                      &nbsp;— recommended on a re-run after manual edits.
                    </span>
                  </span>
                </label>
                <label style={s.controlRow}>
                  <span style={{ flex: 0 }}>
                    <b>Max scenes this run</b>
                    <span style={s.controlHint}>
                      &nbsp;— blank for "all {sceneCount} scenes". Use 3 to test
                      with one chapter first.
                    </span>
                  </span>
                  <input
                    type="number"
                    min={1}
                    max={sceneCount}
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

              {error && <p style={s.error}>{error}</p>}
            </>
          )}

          {running && (
            <>
              <div style={s.runStatus} role="status" aria-live="polite">
                <span style={s.spinner} aria-hidden="true" />
                <div style={{ flex: 1 }}>
                  <p style={s.runTitle}>
                    Generating book — <b>{formatElapsed(wallElapsedMs)}</b> elapsed
                    &nbsp;·&nbsp; scene {sceneProgress}
                  </p>
                  <p style={s.runHint}>
                    {currentScene?.summary
                      ?? bibleStages["world-bible"]?.summary
                      ?? bibleStages["character-bible"]?.summary
                      ?? "Starting…"}
                  </p>
                </div>
              </div>

              <ul style={s.stageList}>
                <StageRow label="Character bible" state={bibleStages["character-bible"]} />
                <StageRow label="World bible"     state={bibleStages["world-bible"]} />
                <StageRow
                  label={`Scenes (${currentScene?.current ?? 0}/${currentScene?.total ?? cap})`}
                  state={currentScene}
                />
              </ul>

              {completedScenes.length > 0 && (
                <details style={s.completedDetails}>
                  <summary style={s.completedSummary}>
                    Completed events: {completedScenes.length}
                  </summary>
                  <ul style={s.completedList}>
                    {completedScenes.slice(-10).map((s2, i) => (
                      <li key={`${i}-${s2.scene_title}`} style={s.completedRow}>
                        <span style={statusDotStyle(s2.status)} />
                        <span style={s.completedTitle}>{s2.scene_title}</span>
                      </li>
                    ))}
                  </ul>
                </details>
              )}
              <p style={s.cancelHint}>
                Use the <b>Live Run</b> indicator in the bottom-right to cancel
                the in-flight stage. Already-applied bibles + scenes are kept.
              </p>
            </>
          )}

          {!running && result && (
            <>
              <p style={s.runTitle}>
                ✓ Done in <b>{formatElapsed(result.total_elapsed_s * 1000)}</b>
              </p>
              <ul style={s.stageList}>
                <StageRow
                  label="Character bible"
                  state={{ stage:"character-bible", status: result.character_bible_status,
                           summary: result.character_bible_status, current:0, total:0, elapsed:0 }}
                />
                <StageRow
                  label="World bible"
                  state={{ stage:"world-bible", status: result.world_bible_status,
                           summary: result.world_bible_status, current:0, total:0, elapsed:0 }}
                />
              </ul>
              <h4 style={s.scenesHeading}>Scenes ({result.scenes.length})</h4>
              <ul style={s.completedList}>
                {result.scenes.map((s2) => (
                  <li key={s2.scene_id} style={s.completedRow}>
                    <span style={statusDotStyle(s2.status)} />
                    <span style={s.completedTitle}>{s2.scene_title}</span>
                    <span style={s.completedMeta}>
                      {s2.word_count > 0 ? `${s2.word_count.toLocaleString()} words · ` : ""}
                      {s2.elapsed_s.toFixed(1)}s · {s2.status}
                    </span>
                  </li>
                ))}
              </ul>
              {error && <p style={s.error}>{error}</p>}
            </>
          )}
        </div>

        <footer style={s.footer}>
          {!running && !result && (
            <>
              <button style={s.ghostBtn} onClick={onClose}>Cancel</button>
              <button style={s.primaryBtn} onClick={handleRun}>
                Run pipeline
              </button>
            </>
          )}
          {running && (
            <button style={s.ghostBtn} disabled>Running… cancel via Live Run overlay</button>
          )}
          {!running && result && (
            <>
              <button style={s.ghostBtn} onClick={() => { setResult(null); }}>
                Run again
              </button>
              <button style={s.primaryBtn} onClick={onClose}>Done</button>
            </>
          )}
        </footer>
      </div>
    </div>
  );
}

// ── Subcomponents ───────────────────────────────────────────────────────────

function StageRow({ label, state }: { label: string; state: StageState | null | undefined }) {
  const status = state?.status ?? "pending";
  return (
    <li style={s.stageRow}>
      <span style={statusDotStyle(status)} />
      <span style={s.stageLabel}>{label}</span>
      <span style={s.stageStatus}>{status}</span>
      {state && state.elapsed > 0 && (
        <span style={s.stageElapsed}>{state.elapsed.toFixed(1)}s</span>
      )}
    </li>
  );
}

// ── Style helpers ───────────────────────────────────────────────────────────

function statusDotStyle(status: string): React.CSSProperties {
  const color =
    status === "completed" ? "var(--color-success, #22c55e)" :
    status === "failed"    ? "var(--color-error, #ef4444)" :
    status === "skipped"   ? "var(--color-text-tertiary)" :
    status === "running"   ? "var(--color-amber-600, #d97706)" :
    "var(--color-border)";
  return {
    width: 10, height: 10, borderRadius: "50%",
    background: color, flexShrink: 0,
  };
}

function formatElapsed(ms: number): string {
  const total = Math.floor(ms / 1000);
  if (total < 60) return `${total}s`;
  const m = Math.floor(total / 60);
  const sec = total % 60;
  return `${m}m ${sec.toString().padStart(2, "0")}s`;
}

// Inject spinner keyframes once on module load.
if (typeof document !== "undefined" && !document.getElementById("bf-bookgen-anim")) {
  const styleEl = document.createElement("style");
  styleEl.id = "bf-bookgen-anim";
  styleEl.textContent = `@keyframes bf-bookgen-spin {
    from { transform: rotate(0deg); }
    to   { transform: rotate(360deg); }
  }`;
  document.head.appendChild(styleEl);
}

const s: Record<string, React.CSSProperties> = {
  overlay: {
    position: "fixed", inset: 0,
    background: "rgba(0,0,0,0.45)",
    display: "flex", alignItems: "center", justifyContent: "center",
    zIndex: 1000,
  },
  dialog: {
    width: 620, maxHeight: "85vh",
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
  close: {
    background: "none", border: "none", cursor: "pointer", fontSize: 16,
    color: "var(--color-text-tertiary)",
  },
  body: {
    padding: "var(--space-4)", overflowY: "auto", flex: 1,
    display: "flex", flexDirection: "column", gap: "var(--space-3)",
  },
  intro: { margin: 0, fontSize: 13, color: "var(--color-text-secondary)" },
  steps: {
    margin: 0, paddingLeft: "var(--space-4)",
    fontSize: 13, color: "var(--color-text-primary)", lineHeight: 1.7,
  },
  note: { margin: 0, fontSize: 12, color: "var(--color-text-tertiary)", lineHeight: 1.6 },
  skipBadge: {
    marginLeft: "var(--space-2)",
    fontSize: 11,
    color: "var(--color-success, #22c55e)",
    background: "rgba(34,197,94,0.08)",
    border: "1px solid rgba(34,197,94,0.35)",
    borderRadius: 999,
    padding: "1px 8px",
    fontWeight: 500,
    fontFamily: "var(--font-ui)",
  },
  skipHint: {
    margin: 0, fontSize: 12,
    color: "var(--color-text-secondary)",
    lineHeight: 1.6,
    background: "rgba(34,197,94,0.05)",
    border: "1px solid rgba(34,197,94,0.25)",
    borderRadius: 4,
    padding: "var(--space-2) var(--space-3)",
  },
  controls: { display: "flex", flexDirection: "column", gap: "var(--space-2)" },
  controlRow: {
    display: "flex", alignItems: "flex-start", gap: "var(--space-2)",
    fontSize: 13, color: "var(--color-text-primary)",
  },
  controlHint: { color: "var(--color-text-tertiary)", fontWeight: 400 },
  numberInput: {
    width: 80, padding: "4px 8px",
    border: "1px solid var(--color-border)", borderRadius: 4,
    fontFamily: "var(--font-mono)", fontSize: 12,
  },
  error: {
    color: "var(--color-error)", fontFamily: "var(--font-mono)",
    fontSize: 12, margin: 0,
  },
  runStatus: {
    display: "flex", alignItems: "flex-start", gap: "var(--space-3)",
    padding: "var(--space-3)",
    background: "var(--color-neutral-50, rgba(0,0,0,0.03))",
    border: "1px solid var(--color-border)", borderRadius: 6,
  },
  runTitle: { margin: 0, fontSize: 14, color: "var(--color-text-primary)" },
  runHint:  { margin: "4px 0 0", fontSize: 12, color: "var(--color-text-secondary)" },
  spinner: {
    width: 18, height: 18, flexShrink: 0,
    borderRadius: "50%",
    border: "2px solid var(--color-border)",
    borderTopColor: "var(--color-amber-600)",
    animation: "bf-bookgen-spin 0.9s linear infinite",
    marginTop: 1,
  },
  stageList: {
    listStyle: "none", margin: 0, padding: 0,
    display: "flex", flexDirection: "column", gap: 4,
  },
  stageRow: {
    display: "flex", alignItems: "center", gap: "var(--space-2)",
    padding: "6px var(--space-3)",
    background: "var(--color-surface)",
    border: "1px solid var(--color-border)", borderRadius: 4,
    fontSize: 13,
  },
  stageLabel:   { flex: 1, color: "var(--color-text-primary)" },
  stageStatus:  { color: "var(--color-text-tertiary)", fontSize: 11, fontFamily: "var(--font-mono)" },
  stageElapsed: { color: "var(--color-text-tertiary)", fontSize: 11, fontVariantNumeric: "tabular-nums" },
  scenesHeading: {
    margin: "var(--space-2) 0 var(--space-1)",
    fontSize: 12, fontWeight: 600,
    textTransform: "uppercase", letterSpacing: "0.06em",
    color: "var(--color-text-tertiary)",
  },
  completedDetails: {
    fontSize: 12, color: "var(--color-text-secondary)",
  },
  completedSummary: {
    cursor: "pointer", padding: "4px 0",
  },
  completedList: {
    listStyle: "none", margin: 0, padding: 0,
    display: "flex", flexDirection: "column", gap: 2,
    maxHeight: 220, overflowY: "auto",
  },
  completedRow: {
    display: "flex", alignItems: "center", gap: "var(--space-2)",
    padding: "4px 8px",
    fontSize: 12,
  },
  completedTitle: { flex: 1, color: "var(--color-text-primary)" },
  completedMeta:  { color: "var(--color-text-tertiary)", fontSize: 11, fontFamily: "var(--font-mono)" },
  cancelHint: {
    margin: 0, fontSize: 11, color: "var(--color-text-tertiary)", lineHeight: 1.5,
  },
  footer: {
    display: "flex", justifyContent: "flex-end", gap: "var(--space-2)",
    padding: "var(--space-3) var(--space-4)",
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
