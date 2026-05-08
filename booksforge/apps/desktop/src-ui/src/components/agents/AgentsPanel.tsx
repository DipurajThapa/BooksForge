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

interface Props {
  projectId: string;
  sceneId:   string | null;
  onClose:   () => void;
}

type AgentKey =
  | "copyeditor"     | "humanization"  | "continuity"
  | "outline"        | "chapter-drafter" | "dev-editor"
  | "memory-curator" | "vocab-dictionary"
  | "intake"         | "intake-and-outline"
  | "developmental-review" | "entity-bible"
  | "proposal-validator" | "peer-review";

interface AgentEntry {
  key:       AgentKey;
  name:      string;
  category:  "prose" | "generating" | "memory" | "meta";
  blurb:     string;
  hasApply:  boolean;
}

const AGENTS: AgentEntry[] = [
  { key: "copyeditor",     name: "Copyeditor",         category: "prose",      blurb: "Mechanical fixes — punctuation, spacing, casing.  Per-edit Accept.", hasApply: true  },
  { key: "humanization",   name: "Humanization",       category: "prose",      blurb: "Detect AI-tells and propose human-sounding rewrites.",                hasApply: true  },
  { key: "continuity",     name: "Continuity",         category: "prose",      blurb: "Find name / POV / tense / timeline drift.  Apply renames or annotate.", hasApply: true  },
  { key: "intake-and-outline", name: "Brief → Outline (chained)", category: "generating", blurb: "Free-text idea → ProjectBrief → outline.  One form, two model calls.", hasApply: false },
  { key: "outline",        name: "Outline Architect",  category: "generating", blurb: "Brief → full outline with chapters + scenes.",                       hasApply: false },
  { key: "chapter-drafter",name: "Chapter Drafter",    category: "generating", blurb: "Synopsis → scene draft in the project's voice.",                     hasApply: false },
  { key: "dev-editor",     name: "Developmental Editor", category: "generating", blurb: "Per-chapter structural notes (pacing, stakes, arcs).",            hasApply: false },
  { key: "developmental-review", name: "Developmental Review (chained)", category: "generating", blurb: "Dev-editor + per-scene continuity linter on a chapter.  One LLM call.", hasApply: false },
  { key: "entity-bible",   name: "Entity Bible (auto-extract)", category: "memory",     blurb: "Memory-curator → review proposed characters/places/items → promote to bible.", hasApply: true },
  { key: "memory-curator", name: "Memory Curator",     category: "memory",     blurb: "Refresh book / chapter / entity memory from accepted prose.",        hasApply: false },
  { key: "vocab-dictionary", name: "Vocab Dictionary", category: "memory",     blurb: "Propose avoid / prefer rules from edit history.  User picks which to promote.", hasApply: true  },
  { key: "intake",         name: "Intake",             category: "generating", blurb: "Free-text idea → typed ProjectBrief.",                              hasApply: false },
  { key: "proposal-validator", name: "Proposal Validator (Tier 2)", category: "meta", blurb: "LLM-backed validation of another agent's output. Auto-invoked.", hasApply: false },
  { key: "peer-review",    name: "Peer Review",        category: "meta",       blurb: "Cross-agent verification on a focus axis. Auto-invoked.",            hasApply: false },
];

export default function AgentsPanel({ projectId, sceneId, onClose }: Props) {
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
  if (selected) {
    return (
      <GenericAgentForm
        agentKey={selected}
        projectId={projectId}
        sceneId={sceneId}
        model={model}
        onClose={() => setSelected(null)}
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
          {(["prose", "generating", "memory", "meta"] as const).map(cat => (
            <section key={cat} style={s.section}>
              <h3 style={s.sectionTitle}>{labelFor(cat)}</h3>
              <div style={s.grid}>
                {AGENTS.filter(a => a.category === cat).map(a => (
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
        </div>
      </div>
    </div>
  );
}

function labelFor(cat: string): string {
  return ({
    prose:      "Prose-mutating (per-edit Accept)",
    generating: "Generating / advising",
    memory:     "Memory & vocabulary",
    meta:       "Internal / meta (auto-invoked)",
  } as Record<string, string>)[cat] ?? cat;
}

const s: Record<string, React.CSSProperties> = {
  overlay:  { position: "fixed", inset: 0, background: "rgba(0,0,0,0.4)", zIndex: 50, display: "flex", alignItems: "center", justifyContent: "center" },
  dialog:   { width: "min(900px, 94vw)", maxHeight: "90vh", display: "flex", flexDirection: "column", background: "var(--color-surface)", border: "1px solid var(--color-border)", borderRadius: 6, overflow: "hidden" },
  header:   { display: "flex", alignItems: "center", justifyContent: "space-between", padding: "10px 14px", borderBottom: "1px solid var(--color-border)" },
  close:    { background: "transparent", border: "none", fontSize: 18, cursor: "pointer", color: "inherit" },
  modelRow: { display: "flex", alignItems: "center", gap: 10, padding: "8px 14px", borderBottom: "1px solid var(--color-border)" },
  modelInput: { flex: 1, padding: "4px 8px", border: "1px solid var(--color-border)", borderRadius: 3, fontSize: 12 },
  body:     { padding: "10px 14px", overflowY: "auto", flex: 1, display: "flex", flexDirection: "column", gap: 16 },
  section:  {},
  sectionTitle: { fontSize: 13, fontWeight: 600, marginBottom: 6, opacity: 0.85 },
  grid:     { display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(240px, 1fr))", gap: 8 },
  card:     { textAlign: "left", padding: 10, border: "1px solid var(--color-border)", borderRadius: 4, cursor: "pointer", background: "var(--color-bg)", color: "inherit", display: "flex", flexDirection: "column", gap: 4 },
  cardHead: { display: "flex", alignItems: "center", gap: 8 },
  cardName: { fontWeight: 600, fontSize: 13 },
  applyTag: { fontSize: 10, padding: "1px 6px", borderRadius: 3, background: "var(--color-success-bg, rgba(46,125,50,0.15))", color: "var(--color-success, #2e7d32)" },
  cardBlurb:{ fontSize: 12, opacity: 0.8 },
};
