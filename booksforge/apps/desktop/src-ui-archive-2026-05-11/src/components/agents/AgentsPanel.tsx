/**
 * Top-level agents panel — switchboard for the 11 registered agents
 * (BACKLOG §E0d.9).
 *
 * Three categories of routing:
 *
 *   - **Mutating prose agents** (Copyedit, Humanization, Continuity)
 *     get full-featured panels with per-edit Accept buttons and inline
 *     verdict display.
 *
 *   - **Generating / advising agents** (Outline, Chapter Drafter,
 *     Dev Editor, Memory Curator, Vocab Dictionary, Intake) use the
 *     generic `<GenericAgentForm>` which dispatches the run, shows the
 *     proposal JSON + verification report, and lets the user copy or
 *     dismiss (apply paths for these are tracked under §E0d.7+ and
 *     follow-ups).
 *
 *   - **Internal / meta agents** (Proposal Validator) are surfaced for
 *     visibility only — invoked automatically by the orchestrator.
 */
import React, { useState } from "react";
import { useDialogA11y } from "../../lib/useDialogA11y";
import CopyeditPanel        from "./CopyeditPanel";
import HumanizationPanel    from "./HumanizationPanel";
import ContinuityPanel      from "./ContinuityPanel";
import VocabDictionaryPanel from "./VocabDictionaryPanel";
import IntakeAndOutlinePanel from "./IntakeAndOutlinePanel";
import DevelopmentalReviewPanel from "./DevelopmentalReviewPanel";
import EntityBiblePanel     from "./EntityBiblePanel";
import GenericAgentForm     from "./GenericAgentForm";
// Fiction agents (BACKLOG §A13 / Phase 1).
import CharacterBiblePanel  from "./CharacterBiblePanel";
import WorldBiblePanel      from "./WorldBiblePanel";
import SceneDrafterFicPanel from "./SceneDrafterFicPanel";
// Specialist polish stack (BACKLOG §A15 / Phase 2).
import PolishStackPanel     from "./PolishStackPanel";
// Quality stack (BACKLOG §A16 / Phase 3).
import VoiceAnchorPanel     from "../quality/VoiceAnchorPanel";
import TellsInspectorPanel  from "../quality/TellsInspectorPanel";
import HonestScorePanel     from "../quality/HonestScorePanel";

interface Props {
  projectId: string;
  sceneId:   string | null;
  onClose:   () => void;
  /** Called after a successful Apply so the editor reloads the active scene. */
  onApplied?: () => void;
}

type AgentKey =
  | "copyeditor"     | "humanization"  | "continuity"
  | "outline"        | "chapter-drafter" | "dev-editor"
  | "memory-curator" | "vocab-dictionary"
  | "intake"         | "intake-and-outline"
  | "developmental-review" | "entity-bible"
  | "proposal-validator" | "peer-review"
  // Fiction agents (BACKLOG §A13 / Phase 1).
  | "character-bible" | "world-bible" | "scene-drafter-fic"
  // Specialist polish stack (BACKLOG §A15 / Phase 2).
  | "polish-stack"
  // Quality stack (BACKLOG §A16 / Phase 3).
  | "voice-anchor" | "tells-inspector" | "honest-score";

interface AgentEntry {
  key:       AgentKey;
  name:      string;
  category:  "prose" | "generating" | "memory" | "meta" | "fiction" | "polish" | "quality";
  /**
   * Phase 6A — workflow-intent grouping. Drives the user-visible panel
   * layout (4 cards: Plan / Draft / Polish / Publish + an Advanced
   * disclosure for auto-invoked meta agents).
   */
  intent:    "plan" | "draft" | "polish" | "publish" | "advanced";
  blurb:     string;
  hasApply:  boolean;
}

const AGENTS: AgentEntry[] = [
  // ── Plan ─────────────────────────────────────────────────────────────
  { key: "intake",             name: "Intake",                       category: "generating", intent: "plan",     blurb: "Free-text idea → typed ProjectBrief.", hasApply: false },
  { key: "intake-and-outline", name: "Brief → Outline (chained)",   category: "generating", intent: "plan",     blurb: "Free-text idea → ProjectBrief → outline. One form, two model calls.", hasApply: false },
  { key: "outline",            name: "Outline Architect",           category: "generating", intent: "plan",     blurb: "Brief → full outline with chapters + scenes.", hasApply: false },
  { key: "character-bible",    name: "Character Bible",             category: "fiction",    intent: "plan",     blurb: "Per-character objective, need, wound, voice, per-chapter arc. Saves to project memory.", hasApply: true },
  { key: "world-bible",        name: "World Bible",                 category: "fiction",    intent: "plan",     blurb: "Locations, social rules, sensory palette, motifs, continuity constraints.", hasApply: true },
  { key: "voice-anchor",       name: "Voice Anchor",                category: "quality",    intent: "plan",     blurb: "Paste comp samples → measure cadence + lexicon as numeric constraints → drafter and polish honour them.", hasApply: true },

  // ── Draft ────────────────────────────────────────────────────────────
  { key: "scene-drafter-fic",  name: "Scene Drafter (Fiction)",     category: "fiction",    intent: "draft",    blurb: "Drafts a scene from goal/conflict/reveal, with character + world bibles loaded.", hasApply: true },
  { key: "chapter-drafter",    name: "Chapter Drafter",             category: "generating", intent: "draft",    blurb: "Synopsis → scene draft in the project's voice. (Non-fiction-shaped — fiction projects use Scene Drafter (Fiction).)", hasApply: false },
  { key: "dev-editor",         name: "Developmental Editor",        category: "generating", intent: "draft",    blurb: "Per-chapter structural notes (pacing, stakes, arcs).", hasApply: false },
  { key: "developmental-review", name: "Developmental Review (chained)", category: "generating", intent: "draft", blurb: "Dev-editor + per-scene continuity linter on a chapter. One LLM call.", hasApply: false },

  // ── Polish ───────────────────────────────────────────────────────────
  { key: "polish-stack",       name: "Polish Stack (4 stages)",     category: "polish",     intent: "polish",   blurb: "Voice-preserving 4-pass polish: dialogue → metaphor → voice → scene-tension. Per-stage Accept.", hasApply: true },
  { key: "copyeditor",         name: "Copyeditor",                  category: "prose",      intent: "polish",   blurb: "Mechanical fixes — punctuation, spacing, casing. Per-edit Accept.", hasApply: true },
  { key: "humanization",       name: "Humanization",                category: "prose",      intent: "polish",   blurb: "Detect AI-tells and propose human-sounding rewrites.", hasApply: true },
  { key: "continuity",         name: "Continuity",                  category: "prose",      intent: "polish",   blurb: "Find name / POV / tense / timeline drift. Apply renames or annotate.", hasApply: true },
  { key: "tells-inspector",    name: "AI-Tells Inspector",          category: "quality",    intent: "polish",   blurb: "Scan prose for AI fingerprints (delve / tapestry / hedge openers / cliché body phrases / em-dash overuse).", hasApply: false },

  // ── Publish ──────────────────────────────────────────────────────────
  { key: "honest-score",       name: "Honest Score",                category: "quality",    intent: "publish",  blurb: "Stylometric distance vs. anchor + AI-tells verdict + per-genre rubric weights. No fake 9/10.", hasApply: false },
  { key: "memory-curator",     name: "Memory Curator",              category: "memory",     intent: "publish",  blurb: "Refresh book / chapter / entity memory from accepted prose.", hasApply: false },
  { key: "vocab-dictionary",   name: "Vocab Dictionary",            category: "memory",     intent: "publish",  blurb: "Propose avoid / prefer rules from edit history. User picks which to promote.", hasApply: true },
  { key: "entity-bible",       name: "Entity Bible (auto-extract)", category: "memory",     intent: "publish",  blurb: "Memory-curator → review proposed characters/places/items → promote to bible.", hasApply: true },

  // ── Advanced (meta — auto-invoked, surfaced for visibility only) ────
  { key: "proposal-validator", name: "Proposal Validator (Tier 2)", category: "meta",       intent: "advanced", blurb: "LLM-backed validation of another agent's output. Auto-invoked.", hasApply: false },
  { key: "peer-review",        name: "Peer Review",                 category: "meta",       intent: "advanced", blurb: "Cross-agent verification on a focus axis. Auto-invoked.", hasApply: false },
];

export default function AgentsPanel({ projectId, sceneId, onClose, onApplied }: Props) {
  const { dialogProps, titleId } = useDialogA11y(onClose);
  const [selected, setSelected] = useState<AgentKey | null>(null);
  const [model,    setModel]    = useState("qwen2.5:7b-instruct-q4_K_M");

  if (selected === "copyeditor") {
    return <CopyeditPanel     projectId={projectId} sceneId={sceneId} model={model} onClose={() => setSelected(null)} />;
  }
  if (selected === "humanization") {
    return <HumanizationPanel projectId={projectId} sceneId={sceneId} model={model} onClose={() => setSelected(null)} />;
  }
  if (selected === "continuity") {
    return <ContinuityPanel   projectId={projectId} sceneId={sceneId} model={model} onClose={() => setSelected(null)} />;
  }
  if (selected === "vocab-dictionary") {
    return <VocabDictionaryPanel projectId={projectId} model={model} onClose={() => setSelected(null)} />;
  }
  if (selected === "intake-and-outline") {
    return <IntakeAndOutlinePanel projectId={projectId} model={model} onClose={() => setSelected(null)} />;
  }
  if (selected === "developmental-review") {
    return <DevelopmentalReviewPanel projectId={projectId} model={model} onClose={() => setSelected(null)} />;
  }
  if (selected === "entity-bible") {
    return <EntityBiblePanel projectId={projectId} sceneId={sceneId} model={model} onClose={() => setSelected(null)} />;
  }
  // Fiction agents (BACKLOG §A13 / Phase 1).
  if (selected === "character-bible") {
    return <CharacterBiblePanel projectId={projectId} model={model} onClose={() => setSelected(null)} onApplied={onApplied} />;
  }
  if (selected === "world-bible") {
    return <WorldBiblePanel projectId={projectId} model={model} onClose={() => setSelected(null)} onApplied={onApplied} />;
  }
  if (selected === "scene-drafter-fic") {
    return <SceneDrafterFicPanel projectId={projectId} sceneId={sceneId} model={model} onClose={() => setSelected(null)} onApplied={onApplied} />;
  }
  if (selected === "polish-stack") {
    return <PolishStackPanel projectId={projectId} sceneId={sceneId} model={model} onClose={() => setSelected(null)} onApplied={onApplied} />;
  }
  if (selected === "voice-anchor") {
    return <VoiceAnchorPanel onClose={() => setSelected(null)} />;
  }
  if (selected === "tells-inspector") {
    return <TellsInspectorPanel onClose={() => setSelected(null)} />;
  }
  if (selected === "honest-score") {
    return <HonestScorePanel onClose={() => setSelected(null)} />;
  }
  if (selected) {
    return (
      <GenericAgentForm
        agentKey={selected}
        projectId={projectId}
        sceneId={sceneId}
        model={model}
        onClose={() => setSelected(null)}
        onApplied={onApplied}
      />
    );
  }

  return (
    <div style={s.overlay} role="presentation">
      <div {...dialogProps} style={s.dialog}>
        <header style={s.header}>
          <strong id={titleId}>Agents</strong>
          <button style={s.close} onClick={onClose} aria-label="Close panel">✕</button>
        </header>
        <div style={s.modelRow}>
          <label style={{ fontSize: 12 }}>Model:</label>
          <input
            style={s.modelInput}
            value={model}
            onChange={e => setModel(e.target.value)}
          />
        </div>
        <div style={s.body}>
          {/* Phase 6A — workflow-intent grouping. Four big sections in
              the order a writer actually moves through them: Plan →
              Draft → Polish → Publish. Replaces the previous 7-category
              switchboard ("the switchboard tax" friction-point #F1
              from the prior UX audit). */}
          {(["plan", "draft", "polish", "publish"] as const).map(intent => (
            <section key={intent} style={s.intentSection}>
              <h3 style={s.intentTitle}>{intentLabel(intent)}</h3>
              <p style={s.intentBlurb}>{intentBlurb(intent)}</p>
              <div style={s.grid}>
                {AGENTS.filter(a => a.intent === intent).map(a => (
                  <button
                    key={a.key}
                    style={s.card}
                    onClick={() => setSelected(a.key)}
                    title={a.blurb}
                  >
                    <div style={s.cardHead}>
                      <span style={s.cardName}>{a.name}</span>
                      {a.hasApply && <span style={s.applyTag}>apply</span>}
                    </div>
                    <div style={s.cardBlurb}>{a.blurb}</div>
                  </button>
                ))}
              </div>
            </section>
          ))}

          {/* Phase 6B — meta agents hidden under Advanced. They are
              auto-invoked by the orchestrator alongside other agent
              runs; the user almost never needs to click them. Keeping
              them visible-but-collapsed preserves discoverability for
              power users without cluttering the first-time view. */}
          <details style={s.advancedDetails}>
            <summary style={s.advancedSummary}>
              Advanced (auto-invoked agents — for visibility only)
            </summary>
            <div style={s.grid}>
              {AGENTS.filter(a => a.intent === "advanced").map(a => (
                <button
                  key={a.key}
                  style={s.card}
                  onClick={() => setSelected(a.key)}
                  title={a.blurb}
                >
                  <div style={s.cardHead}>
                    <span style={s.cardName}>{a.name}</span>
                  </div>
                  <div style={s.cardBlurb}>{a.blurb}</div>
                </button>
              ))}
            </div>
          </details>
        </div>
      </div>
    </div>
  );
}

function intentLabel(intent: "plan" | "draft" | "polish" | "publish"): string {
  return ({
    plan:    "Plan",
    draft:   "Draft",
    polish:  "Polish",
    publish: "Publish",
  } as const)[intent];
}

function intentBlurb(intent: "plan" | "draft" | "polish" | "publish"): string {
  return ({
    plan:    "Brief, outline, character + world bibles, voice anchor.",
    draft:   "Scene-by-scene drafting + per-chapter developmental notes.",
    polish:  "Voice-preserving polish stack + per-edit copyedit / continuity / humanization + AI-tells inspector.",
    publish: "Honest score, memory + vocabulary curation for the next book in the series.",
  } as const)[intent];
}

const s: Record<string, React.CSSProperties> = {
  overlay:  { position: "fixed", inset: 0, background: "rgba(0,0,0,0.4)", zIndex: 50, display: "flex", alignItems: "center", justifyContent: "center" },
  dialog:   { width: "min(900px, 94vw)", maxHeight: "90vh", display: "flex", flexDirection: "column", background: "var(--color-surface)", border: "1px solid var(--color-border)", borderRadius: 6, overflow: "hidden" },
  header:   { display: "flex", alignItems: "center", justifyContent: "space-between", padding: "10px 14px", borderBottom: "1px solid var(--color-border)" },
  close:    { background: "transparent", border: "none", fontSize: 18, cursor: "pointer", color: "inherit" },
  modelRow: { display: "flex", alignItems: "center", gap: 10, padding: "8px 14px", borderBottom: "1px solid var(--color-border)" },
  modelInput: { flex: 1, padding: "4px 8px", border: "1px solid var(--color-border)", borderRadius: 3, fontSize: 12 },
  body:     { padding: "10px 14px", overflowY: "auto", flex: 1, display: "flex", flexDirection: "column", gap: 18 },
  section:  {},
  sectionTitle: { fontSize: 13, fontWeight: 600, marginBottom: 6, opacity: 0.85 },
  // Phase 6A — intent grouping headers stand out from the cards beneath.
  intentSection: { borderTop: "2px solid var(--color-border)", paddingTop: 10 },
  intentTitle:  { fontSize: 16, fontWeight: 700, margin: "0 0 4px" },
  intentBlurb:  { fontSize: 12, opacity: 0.75, margin: "0 0 8px" },
  // Phase 6B — Advanced collapsible disclosure for the meta agents.
  advancedDetails: { borderTop: "2px solid var(--color-border)", paddingTop: 10 },
  advancedSummary: { cursor: "pointer", fontSize: 13, fontWeight: 600, opacity: 0.7, marginBottom: 8 },
  grid:     { display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(240px, 1fr))", gap: 8 },
  card:     { textAlign: "left", padding: 10, border: "1px solid var(--color-border)", borderRadius: 4, cursor: "pointer", background: "var(--color-bg)", color: "inherit", display: "flex", flexDirection: "column", gap: 4 },
  cardHead: { display: "flex", alignItems: "center", gap: 8 },
  cardName: { fontWeight: 600, fontSize: 13 },
  applyTag: { fontSize: 10, padding: "1px 6px", borderRadius: 3, background: "var(--color-success-bg, rgba(46,125,50,0.15))", color: "var(--color-success, #2e7d32)" },
  cardBlurb:{ fontSize: 12, opacity: 0.8 },
};
