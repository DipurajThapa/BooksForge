/**
 * Polish Stack panel (BACKLOG §A15 / Phase 2).
 *
 * Sequences the four voice-preserving specialist polish stages on the
 * active scene:
 *   - dialogue       — sharpen, cut exposition, differentiate speakers
 *   - metaphor       — replace clichés, character-specific imagery
 *   - voice          — preserve and amplify author voice
 *   - scene-tension  — tighten rising line, strengthen hook ending
 *
 * Each stage runs as a separate orchestrator call. The user can run the
 * stages in order (default) or pick individual ones, review the diff
 * between original and revised pm_doc, and Accept/Skip per stage.
 *
 * Genre lens drives which stages get emphasis (per the genre packs):
 *   - literary_fiction → voice, metaphor, dialogue, scene_tension
 *   - genre_fiction    → scene_tension, dialogue, metaphor, voice
 *   - non_fiction      → (uses different stages — see Phase 3 NF stack)
 */
import React, { useState } from "react";
import { useDialogA11y } from "../../lib/useDialogA11y";
import type {
  AgentRunResultDto,
  RunPolishStageInput,
} from "@booksforge/shared-types";
import { ipc } from "../../lib/ipc";
import { errorMessage } from "../../lib/errorMessage";

interface Props {
  projectId: string;
  sceneId:   string | null;
  model:     string;
  onClose:   () => void;
  onApplied?: () => void;
}

type Stage = "dialogue" | "metaphor" | "voice" | "scene_tension";
type GenreLens = "literary_fiction" | "genre_fiction";

const STAGE_LABELS: Record<Stage, string> = {
  dialogue:      "Dialogue",
  metaphor:      "Metaphor",
  voice:         "Voice",
  scene_tension: "Scene Tension",
};

const STAGE_BLURBS: Record<Stage, string> = {
  dialogue:      "Sharpen dialogue. Cut exposition. Differentiate speakers by cadence.",
  metaphor:      "Replace clichés with character-specific images. Forbid generic-AI metaphors.",
  voice:         "Preserve and amplify author voice. Never flatten distinctive sentence shapes.",
  scene_tension: "Tighten rising line. Cut slack. Strengthen the scene-end hook.",
};

const GENRE_ORDER: Record<GenreLens, Stage[]> = {
  // Literary: prose-craft first, then narrative tension.
  literary_fiction: ["voice", "metaphor", "dialogue", "scene_tension"],
  // Genre: narrative tension first, voice last.
  genre_fiction:    ["scene_tension", "dialogue", "metaphor", "voice"],
};

interface StageState {
  status:   "idle" | "running" | "ready" | "applied" | "skipped" | "error";
  result?:  AgentRunResultDto;
  error?:   string;
}

interface PolishProposal {
  stage_id: Stage;
  revised_pm_doc: { type: string; content?: unknown[] };
  revised_word_count: number;
  edit_notes: string;
}

function tryParseProposal(json: string | null): PolishProposal | null {
  if (!json) return null;
  try {
    return JSON.parse(json);
  } catch {
    return null;
  }
}

function pmDocToPlainText(doc: unknown): string {
  if (!doc || typeof doc !== "object") return "";
  const out: string[] = [];
  function walk(n: unknown): void {
    if (!n || typeof n !== "object") return;
    const node = n as { type?: string; text?: string; content?: unknown[] };
    if (node.type === "text" && typeof node.text === "string") {
      out.push(node.text);
      return;
    }
    if (Array.isArray(node.content)) node.content.forEach(walk);
    if (node.type === "paragraph" || node.type === "heading") out.push("\n\n");
  }
  walk(doc);
  return out.join("").replace(/\n{3,}/g, "\n\n").trim();
}

export default function PolishStackPanel({ projectId, sceneId, model, onClose, onApplied }: Props) {
  const { dialogProps, titleId } = useDialogA11y(onClose);
  const [genreLens,        setGenreLens]        = useState<GenreLens>("literary_fiction");
  const [povCharacter,     setPovCharacter]     = useState("");
  const [voiceConstraints, setVoiceConstraints] = useState("");
  const [stageStates, setStageStates] = useState<Record<Stage, StageState>>({
    dialogue:      { status: "idle" },
    metaphor:      { status: "idle" },
    voice:         { status: "idle" },
    scene_tension: { status: "idle" },
  });
  const [error, setError] = useState<string | null>(null);

  const order = GENRE_ORDER[genreLens];

  function setStage(stage: Stage, patch: Partial<StageState>) {
    setStageStates(prev => ({ ...prev, [stage]: { ...prev[stage], ...patch } }));
  }

  async function runStage(stage: Stage) {
    if (!sceneId) {
      setError("Open a scene in the editor before polishing.");
      return;
    }
    setError(null);
    setStage(stage, { status: "running", result: undefined, error: undefined });
    try {
      const input: RunPolishStageInput = {
        project_id:        projectId,
        scene_id:          sceneId,
        stage,
        genre_label:       genreLens,
        voice_constraints: voiceConstraints,
        pov_character:     povCharacter,
        model,
      };
      const r = await ipc.agentRunPolishStage(input);
      setStage(stage, { status: "ready", result: r });
    } catch (e) {
      setStage(stage, { status: "error", error: errorMessage(e) });
    }
  }

  async function applyStage(stage: Stage) {
    const st = stageStates[stage];
    if (!st.result || !sceneId) return;
    try {
      await ipc.agentApplyPolish({ task_id: st.result.task_id, scene_id: sceneId });
      setStage(stage, { status: "applied" });
      onApplied?.();
    } catch (e) {
      setStage(stage, { status: "error", error: errorMessage(e) });
    }
  }

  async function runAll() {
    for (const stage of order) {
      await runStage(stage);
      // Caller decides whether to apply each one — we don't auto-apply.
    }
  }

  const allApplied = order.every(s => stageStates[s].status === "applied");

  return (
    <div style={s.overlay} role="presentation">
      <div {...dialogProps} style={s.dialog}>
        <header style={s.header}>
          <strong id={titleId}>Polish Stack — voice-preserving 4-stage editor</strong>
          <button style={s.close} onClick={onClose} aria-label="Close panel">✕</button>
        </header>

        <div style={s.body}>
          <p style={s.blurb}>
            Four specialist passes, each scoped to its remit. None of them
            tries to do everything at once — that&apos;s how the previous
            single-polisher flattened voice. Run individually or all at
            once. Review the diff and Accept per stage.
          </p>

          <div style={s.row}>
            <label style={s.label}>Genre lens</label>
            <select
              style={s.select}
              value={genreLens}
              onChange={e => setGenreLens(e.target.value as GenreLens)}
            >
              <option value="literary_fiction">literary fiction</option>
              <option value="genre_fiction">genre fiction</option>
            </select>
            <label style={s.label}>POV character</label>
            <input
              style={s.input}
              value={povCharacter}
              onChange={e => setPovCharacter(e.target.value)}
              placeholder="e.g. Ada (used by metaphor stage)"
            />
          </div>

          <label style={s.label}>Voice constraints (optional)</label>
          <textarea
            style={s.textarea}
            value={voiceConstraints}
            onChange={e => setVoiceConstraints(e.target.value)}
            placeholder="Numeric voice fingerprint (Phase 3 will fill this from the project's Voice Anchor automatically)."
            rows={3}
          />

          <div style={s.actionRow}>
            <button style={s.runBtn} onClick={runAll}>Run all 4 stages in genre order</button>
            <span style={s.hint}>Order: {order.map(s => STAGE_LABELS[s]).join(" → ")}</span>
          </div>

          {error && <div style={s.error}>{error}</div>}

          <div style={s.stageList}>
            {order.map((stage, i) => (
              <StageRow
                key={stage}
                index={i + 1}
                stage={stage}
                state={stageStates[stage]}
                onRun={() => runStage(stage)}
                onApply={() => applyStage(stage)}
              />
            ))}
          </div>

          {allApplied && (
            <div style={s.successBanner}>
              ✓ All four stages applied. Review the scene in the editor.
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

function StageRow({
  index, stage, state, onRun, onApply,
}: {
  index:  number;
  stage:  Stage;
  state:  StageState;
  onRun:  () => void;
  onApply: () => void;
}) {
  const proposal = tryParseProposal(state.result?.proposal_json ?? null);
  const previewText = proposal ? pmDocToPlainText(proposal.revised_pm_doc) : "";
  const previewWords = previewText.split(/\s+/).filter(Boolean).length;

  const statusBadge = (() => {
    switch (state.status) {
      case "idle":    return <span style={{ ...s.badge, ...s.badgeIdle }}>not run</span>;
      case "running": return <span style={{ ...s.badge, ...s.badgeRunning }}>running…</span>;
      case "ready":   return <span style={{ ...s.badge, ...s.badgeReady }}>ready</span>;
      case "applied": return <span style={{ ...s.badge, ...s.badgeApplied }}>✓ applied</span>;
      case "skipped": return <span style={{ ...s.badge, ...s.badgeIdle }}>skipped</span>;
      case "error":   return <span style={{ ...s.badge, ...s.badgeError }}>error</span>;
    }
  })();

  return (
    <div style={s.stageCard}>
      <div style={s.stageHead}>
        <div>
          <strong>{index}. {STAGE_LABELS[stage]}</strong>
          {statusBadge}
        </div>
        <div style={s.stageActions}>
          <button
            style={s.smallBtn}
            onClick={onRun}
            disabled={state.status === "running" || state.status === "applied"}
          >
            {state.status === "running" ? "Running…" : "Run"}
          </button>
          <button
            style={s.applyBtn}
            onClick={onApply}
            disabled={state.status !== "ready"}
          >
            Apply
          </button>
        </div>
      </div>
      <div style={s.stageBlurb}>{STAGE_BLURBS[stage]}</div>

      {state.error && <div style={s.error}>{state.error}</div>}

      {proposal && previewText && (
        <details style={s.diffDetails}>
          <summary>
            Revised prose ({previewWords.toLocaleString()} words)
            {proposal.edit_notes && <span style={s.notesInline}> · {proposal.edit_notes.slice(0, 80)}{proposal.edit_notes.length > 80 ? "…" : ""}</span>}
          </summary>
          <pre style={s.previewBody}>{previewText}</pre>
          {proposal.edit_notes && (
            <div style={s.notesBlock}>
              <strong>Edit notes:</strong> {proposal.edit_notes}
            </div>
          )}
        </details>
      )}
    </div>
  );
}

const s: Record<string, React.CSSProperties> = {
  overlay:  { position: "fixed", inset: 0, background: "rgba(0,0,0,0.4)", zIndex: 50, display: "flex", alignItems: "center", justifyContent: "center" },
  dialog:   { width: "min(900px, 96vw)", maxHeight: "94vh", display: "flex", flexDirection: "column", background: "var(--color-surface)", border: "1px solid var(--color-border)", borderRadius: 6, overflow: "hidden" },
  header:   { display: "flex", alignItems: "center", justifyContent: "space-between", padding: "10px 14px", borderBottom: "1px solid var(--color-border)" },
  close:    { background: "transparent", border: "none", fontSize: 18, cursor: "pointer", color: "inherit" },
  body:     { padding: "10px 14px", overflowY: "auto", display: "flex", flexDirection: "column", gap: 10 },
  blurb:    { margin: 0, fontSize: 13, opacity: 0.85 },
  row:      { display: "flex", alignItems: "center", gap: 8, flexWrap: "wrap" },
  label:    { fontSize: 12, fontWeight: 500 },
  hint:     { fontSize: 11, opacity: 0.65 },
  input:    { padding: "4px 8px", border: "1px solid var(--color-border)", borderRadius: 3, fontSize: 13, background: "var(--color-bg)", color: "inherit" },
  select:   { padding: "4px 8px", border: "1px solid var(--color-border)", borderRadius: 3, fontSize: 13, background: "var(--color-bg)", color: "inherit" },
  textarea: { padding: "8px", border: "1px solid var(--color-border)", borderRadius: 3, fontSize: 12, background: "var(--color-bg)", color: "inherit", fontFamily: "ui-monospace, SFMono-Regular, monospace", resize: "vertical" },
  actionRow:{ display: "flex", alignItems: "center", gap: 12, flexWrap: "wrap" },
  runBtn:   { padding: "6px 14px", border: "1px solid var(--color-border)", borderRadius: 4, cursor: "pointer", background: "var(--color-bg)", fontWeight: 600 },
  stageList:{ display: "flex", flexDirection: "column", gap: 8 },
  stageCard:{ border: "1px solid var(--color-border)", borderRadius: 4, padding: 10, background: "var(--color-bg)" },
  stageHead:{ display: "flex", justifyContent: "space-between", alignItems: "center", gap: 8 },
  stageActions:{ display: "flex", gap: 6 },
  stageBlurb:{ fontSize: 12, opacity: 0.75, marginTop: 4 },
  smallBtn: { padding: "4px 10px", border: "1px solid var(--color-border)", borderRadius: 3, cursor: "pointer", fontSize: 12, background: "var(--color-bg)" },
  applyBtn: { padding: "4px 10px", border: "1px solid var(--color-border)", borderRadius: 3, cursor: "pointer", fontSize: 12, background: "var(--color-success-bg, #e8f5e9)", color: "var(--color-success, #2e7d32)", fontWeight: 600 },
  badge:    { fontSize: 10, padding: "1px 6px", borderRadius: 3, marginLeft: 8 },
  badgeIdle:    { background: "#eee", color: "#666" },
  badgeRunning: { background: "#fff3cd", color: "#664d03" },
  badgeReady:   { background: "#cfe2ff", color: "#084298" },
  badgeApplied: { background: "var(--color-success-bg, #e8f5e9)", color: "var(--color-success, #2e7d32)" },
  badgeError:   { background: "#f8d7da", color: "#842029" },
  diffDetails:  { marginTop: 8, fontSize: 12 },
  notesInline:  { opacity: 0.65, fontSize: 11, fontStyle: "italic" },
  notesBlock:   { marginTop: 8, padding: 8, background: "var(--color-surface)", borderRadius: 3, fontSize: 11, fontStyle: "italic" },
  previewBody:  { margin: "8px 0 0", whiteSpace: "pre-wrap", fontSize: 12, lineHeight: 1.55, maxHeight: 240, overflowY: "auto", fontFamily: "Georgia, serif" },
  successBanner:{ padding: 10, background: "var(--color-success-bg, #e8f5e9)", color: "var(--color-success, #2e7d32)", borderRadius: 4, fontSize: 13, fontWeight: 600 },
  error:    { color: "var(--color-error, #c62828)", padding: "6px 10px", fontSize: 12 },
};
