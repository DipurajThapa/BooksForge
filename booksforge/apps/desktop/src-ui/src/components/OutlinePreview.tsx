/**
 * Read-only renderer for an OutlineProposal returned by the
 * outline-architect agent (MZ-05 → MZ-07).
 *
 * Displays the proposal as a Parts → Chapters → Scenes hierarchy with
 * synopsis, POV, beat, and target word count.  Editing the proposal in
 * place is deferred to a later milestone; for now the user accepts or
 * rejects the whole thing.
 */
import React from "react";

// Local mirror of the Rust OutlineProposal shape (no ts-rs binding because
// it's an agent-domain type, not an IPC type — the wire format is JSON
// strings inside `OutlineRunResult.proposal_json`).
export interface ScenePlan {
  synopsis: string;
  pov: string | null;
  beat: string | null;
  target_word_count: number | null;
}

export interface ChapterPlan {
  title: string;
  purpose: string;
  scenes: ScenePlan[];
}

export interface PartPlan {
  title: string;
  purpose: string;
  chapters: ChapterPlan[];
}

export interface OutlineProposal {
  parts: PartPlan[];
  rationale: string;
  notes_to_user: string[];
}

interface Props {
  proposal: OutlineProposal;
}

export default function OutlinePreview({ proposal }: Props) {
  const totalChapters = proposal.parts.reduce((n, p) => n + p.chapters.length, 0);
  const totalScenes   = proposal.parts.reduce(
    (n, p) => n + p.chapters.reduce((m, c) => m + c.scenes.length, 0),
    0,
  );
  const totalWords = proposal.parts.reduce(
    (n, p) => n + p.chapters.reduce(
      (m, c) => m + c.scenes.reduce((k, s) => k + (s.target_word_count ?? 0), 0),
      0,
    ),
    0,
  );

  return (
    <div style={s.root}>
      <div style={s.summary}>
        <span><b>{proposal.parts.length}</b> parts</span>
        <span><b>{totalChapters}</b> chapters</span>
        <span><b>{totalScenes}</b> scenes</span>
        <span><b>{totalWords.toLocaleString()}</b> target words</span>
      </div>

      {proposal.notes_to_user.length > 0 && (
        <ul style={s.notes}>
          {proposal.notes_to_user.map((n, i) => <li key={i}>{n}</li>)}
        </ul>
      )}

      <div style={s.tree}>
        {proposal.parts.map((part, pi) => (
          <section key={pi} style={s.part}>
            <h3 style={s.partTitle}>{part.title}</h3>
            <p style={s.purpose}>{part.purpose}</p>
            {part.chapters.map((ch, ci) => (
              <div key={ci} style={s.chapter}>
                <h4 style={s.chapterTitle}>
                  Chapter {pi + 1}.{ci + 1} — {ch.title}
                </h4>
                <p style={s.purpose}>{ch.purpose}</p>
                <ol style={s.sceneList}>
                  {ch.scenes.map((sc, si) => (
                    <li key={si} style={s.scene}>
                      <p style={s.sceneSynopsis}>{sc.synopsis}</p>
                      <div style={s.sceneMeta}>
                        {sc.pov  && <span>POV: {sc.pov}</span>}
                        {sc.beat && <span>Beat: {sc.beat}</span>}
                        {sc.target_word_count && (
                          <span>{sc.target_word_count.toLocaleString()} words</span>
                        )}
                      </div>
                    </li>
                  ))}
                </ol>
              </div>
            ))}
          </section>
        ))}
      </div>

      {proposal.rationale && (
        <details style={s.rationale}>
          <summary>Rationale</summary>
          <p>{proposal.rationale}</p>
        </details>
      )}
    </div>
  );
}

const s: Record<string, React.CSSProperties> = {
  root:     { display: "flex", flexDirection: "column", gap: 16, padding: 4 },
  summary:  {
    display: "flex", gap: 16, padding: "8px 12px",
    background: "var(--color-surface-raised)",
    border: "1px solid var(--color-border)", borderRadius: 6,
    fontSize: 12, color: "var(--color-text-secondary)",
  },
  notes: {
    margin: 0, paddingLeft: 20,
    fontSize: 12, color: "var(--color-text-secondary)",
    background: "var(--color-amber-50, #fffbeb)",
    border: "1px solid var(--color-amber-400, #fbbf24)",
    borderRadius: 6, padding: "8px 8px 8px 28px",
  },
  tree:    { display: "flex", flexDirection: "column", gap: 16 },
  part:    {
    border: "1px solid var(--color-border)", borderRadius: 6,
    padding: 12, background: "var(--color-surface)",
  },
  partTitle: { margin: "0 0 4px", fontSize: 15, color: "var(--color-text-primary)" },
  chapter: {
    marginTop: 10, padding: "8px 10px",
    border: "1px solid var(--color-border)", borderRadius: 4,
    background: "var(--color-surface-raised)",
  },
  chapterTitle: { margin: "0 0 2px", fontSize: 13, color: "var(--color-text-primary)" },
  purpose:      { margin: "0 0 6px", fontSize: 12, color: "var(--color-text-secondary)", fontStyle: "italic" },
  sceneList:    { margin: 0, paddingLeft: 18, display: "flex", flexDirection: "column", gap: 4 },
  scene:        { fontSize: 12, color: "var(--color-text-primary)" },
  sceneSynopsis:{ margin: 0 },
  sceneMeta:    {
    display: "flex", gap: 12, marginTop: 2,
    fontSize: 11, color: "var(--color-text-tertiary)",
  },
  rationale: {
    fontSize: 12, color: "var(--color-text-secondary)",
    border: "1px solid var(--color-border)", borderRadius: 6, padding: 10,
  },
};
