/**
 * OutlinePreview — read-only renderer for an `OutlineProposal`.
 *
 * Shape mirrors `crates/booksforge-domain/src/outline.rs`:
 *   { parts: [ { title, purpose, chapters: [ {
 *       title, purpose, scenes: [ { synopsis, pov, beat, target_word_count } ]
 *   } ] } ], rationale, notes_to_user }
 *
 * After 2026-05-11 schema relaxation, every nested field is `#[serde(default)]`
 * so we tolerate missing `purpose` / `scenes` etc. without crashing.
 */
import type { CSSProperties, ReactNode } from "react";

export interface ScenePlan {
  synopsis:          string;
  pov?:              string | null;
  beat?:             string | null;
  target_word_count?: number | null;
}
export interface ChapterPlan {
  title:    string;
  purpose?: string;
  scenes?:  ScenePlan[];
}
export interface PartPlan {
  title:     string;
  purpose?:  string;
  chapters?: ChapterPlan[];
}
export interface OutlineProposal {
  parts:           PartPlan[];
  rationale?:      string;
  notes_to_user?:  string[];
}

interface Props {
  proposal: OutlineProposal;
}

export default function OutlinePreview({ proposal }: Props) {
  const totalChapters = proposal.parts.reduce(
    (n, p) => n + (p.chapters?.length ?? 0), 0
  );
  const totalScenes = proposal.parts.reduce(
    (n, p) => n + (p.chapters ?? []).reduce((m, c) => m + (c.scenes?.length ?? 0), 0), 0
  );
  const totalWords = proposal.parts.reduce(
    (n, p) => n + (p.chapters ?? []).reduce(
      (m, c) => m + (c.scenes ?? []).reduce(
        (k, s) => k + (s.target_word_count ?? 0), 0
      ), 0
    ), 0
  );

  return (
    <div style={s.root}>
      <header style={s.header}>
        <span style={s.headerStat}>
          <b>{proposal.parts.length}</b> {plural(proposal.parts.length, "part")}
        </span>
        <span style={s.headerStat}>
          <b>{totalChapters}</b> {plural(totalChapters, "chapter")}
        </span>
        <span style={s.headerStat}>
          <b>{totalScenes}</b> {plural(totalScenes, "scene")}
        </span>
        {totalWords > 0 && (
          <span style={s.headerStat}>
            <b>{totalWords.toLocaleString()}</b> target words
          </span>
        )}
      </header>

      <ol style={s.partList}>
        {proposal.parts.map((part, pi) => (
          <li key={pi} style={s.part}>
            <h3 style={s.partTitle}>
              <span style={s.idx}>Part {pi + 1}</span> · {part.title}
            </h3>
            {part.purpose && (
              <p style={s.purpose}><em>{part.purpose}</em></p>
            )}
            <ol style={s.chapterList}>
              {(part.chapters ?? []).map((chap, ci) => (
                <li key={ci} style={s.chapter}>
                  <h4 style={s.chapterTitle}>
                    <span style={s.idx}>Ch {ci + 1}</span> · {chap.title}
                  </h4>
                  {chap.purpose && (
                    <p style={s.purpose}><em>{chap.purpose}</em></p>
                  )}
                  {(chap.scenes ?? []).length > 0 && (
                    <ol style={s.sceneList}>
                      {(chap.scenes ?? []).map((scene, si) => (
                        <li key={si} style={s.scene}>
                          <span style={s.sceneSynopsis}>
                            {scene.synopsis || <em style={s.muted}>(empty synopsis)</em>}
                          </span>
                          <SceneMeta scene={scene} />
                        </li>
                      ))}
                    </ol>
                  )}
                </li>
              ))}
            </ol>
          </li>
        ))}
      </ol>

      {proposal.rationale && (
        <details style={s.details}>
          <summary style={s.detailsSummary}>Architect's rationale</summary>
          <p style={s.rationale}>{proposal.rationale}</p>
        </details>
      )}

      {(proposal.notes_to_user ?? []).length > 0 && (
        <details style={s.details}>
          <summary style={s.detailsSummary}>
            Notes from the architect ({proposal.notes_to_user?.length})
          </summary>
          <ul style={s.notesList}>
            {(proposal.notes_to_user ?? []).map((n, i) => (
              <li key={i} style={s.notesItem}>{n}</li>
            ))}
          </ul>
        </details>
      )}
    </div>
  );
}

function SceneMeta({ scene }: { scene: ScenePlan }) {
  const bits: ReactNode[] = [];
  if (scene.pov)               bits.push(<span key="pov">POV: {scene.pov}</span>);
  if (scene.beat)              bits.push(<span key="beat">Beat: {scene.beat}</span>);
  if (scene.target_word_count) bits.push(<span key="wc">{scene.target_word_count.toLocaleString()} words</span>);
  if (bits.length === 0) return null;
  return (
    <span style={s.sceneMeta}>
      {bits.map((b, i) => (
        <span key={i}>{i > 0 && <span style={s.dot}> · </span>}{b}</span>
      ))}
    </span>
  );
}

function plural(n: number, singular: string): string {
  return n === 1 ? singular : `${singular}s`;
}

const s: Record<string, CSSProperties> = {
  root: {
    fontFamily: "var(--font-ui)",
    background: "#fff",
    border: "1px solid var(--color-neutral-200)",
    borderRadius: 6,
    overflow: "hidden",
  },
  header: {
    display: "flex", gap: 24,
    padding: "10px 16px",
    background: "var(--color-neutral-50)",
    borderBottom: "1px solid var(--color-neutral-200)",
    fontSize: 12, color: "var(--color-neutral-700)",
  },
  headerStat: { fontVariantNumeric: "tabular-nums" },
  partList: {
    listStyle: "none", margin: 0, padding: "8px 0",
    display: "flex", flexDirection: "column", gap: 4,
  },
  part: {
    padding: "8px 16px",
  },
  partTitle: {
    margin: "0 0 4px",
    fontFamily: "var(--font-prose, serif)",
    fontSize: 16, fontWeight: 600,
    color: "var(--color-neutral-900)",
  },
  chapterList: {
    listStyle: "none", margin: "4px 0 0", padding: 0,
    display: "flex", flexDirection: "column", gap: 4,
    paddingLeft: 16,
  },
  chapter: {
    padding: "8px 12px",
    background: "var(--color-neutral-50)",
    border: "1px solid var(--color-neutral-200)",
    borderRadius: 4,
  },
  chapterTitle: {
    margin: 0,
    fontFamily: "var(--font-prose, serif)",
    fontSize: 14, fontWeight: 600,
    color: "var(--color-neutral-900)",
  },
  sceneList: {
    listStyle: "none", margin: "6px 0 0", padding: 0,
    display: "flex", flexDirection: "column", gap: 2,
    paddingLeft: 12,
  },
  scene: {
    display: "flex", flexDirection: "column", gap: 2,
    padding: "4px 8px",
    borderLeft: "2px solid var(--color-neutral-200)",
    fontSize: 13, color: "var(--color-neutral-800)",
  },
  sceneSynopsis: { lineHeight: 1.5 },
  sceneMeta: {
    fontSize: 11, color: "var(--color-neutral-500)",
    fontVariantNumeric: "tabular-nums",
  },
  dot: { color: "var(--color-neutral-300)" },
  purpose: {
    margin: "0 0 4px",
    fontSize: 12, color: "var(--color-neutral-600)",
    lineHeight: 1.5,
  },
  idx: {
    fontSize: 10, fontWeight: 700, letterSpacing: "0.06em",
    color: "var(--color-amber-600)",
    marginRight: 6, textTransform: "uppercase",
  },
  muted: { color: "var(--color-neutral-400)" },
  details: {
    borderTop: "1px solid var(--color-neutral-200)",
    fontSize: 13,
  },
  detailsSummary: {
    padding: "10px 16px",
    cursor: "pointer",
    fontSize: 11, fontWeight: 600,
    textTransform: "uppercase", letterSpacing: "0.06em",
    color: "var(--color-neutral-600)",
    userSelect: "none",
  },
  rationale: {
    margin: 0, padding: "0 16px 12px",
    fontSize: 13, color: "var(--color-neutral-800)",
    lineHeight: 1.6,
    fontFamily: "var(--font-prose, serif)",
  },
  notesList: {
    margin: 0, padding: "0 16px 12px 32px",
    display: "flex", flexDirection: "column", gap: 4,
  },
  notesItem: {
    fontSize: 13, color: "var(--color-neutral-800)",
    lineHeight: 1.5,
  },
};
