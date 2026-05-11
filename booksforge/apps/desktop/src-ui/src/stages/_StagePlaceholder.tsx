/**
 * Shared placeholder layout for stages that haven't been built out yet.
 * Each MVP stage starts as a placeholder + grows into a real panel as
 * we work through Phase B of the journey roadmap.
 *
 * The placeholder communicates three things to the writer:
 *   1. What this stage does
 *   2. Which gate criteria will apply when it's live
 *   3. Where they are in the journey (the rail handles this; this
 *      panel just confirms by repeating the stage name)
 */
import React from "react";
import type { OpenProjectResult } from "@booksforge/shared-types";

export interface StagePlaceholderProps {
  project:     OpenProjectResult;
  stageNumber: number;
  stageName:   string;
  blurb:       string;
  /** What this stage's quality gate measures, listed for the writer. */
  gateAxes?:   string[];
  /** When this stage will be wired up (informational only). */
  phaseEta?:   string;
}

export default function StagePlaceholder({
  project, stageNumber, stageName, blurb, gateAxes, phaseEta,
}: StagePlaceholderProps) {
  return (
    <div style={s.root}>
      <div style={s.card}>
        <p style={s.stageNum}>Stage {stageNumber} of 6</p>
        <h1 style={s.title}>{stageName}</h1>
        <p style={s.blurb}>{blurb}</p>

        {gateAxes && gateAxes.length > 0 && (
          <section style={s.section}>
            <h3 style={s.sectionH}>Quality gate (when live)</h3>
            <ul style={s.list}>
              {gateAxes.map((axis) => (
                <li key={axis} style={s.listItem}>
                  <span style={s.bullet} aria-hidden="true" />
                  {axis}
                </li>
              ))}
            </ul>
            <p style={s.threshold}>
              Pass threshold: each axis ≥ <b>8.5 / 10</b>. Below 8.5 the
              corresponding fix-agent runs (up to 3 iterations) before
              surfacing the diagnostic for manual edit.
            </p>
          </section>
        )}

        {phaseEta && (
          <p style={s.eta}>
            <b>Building this stage:</b> {phaseEta}
          </p>
        )}

        <p style={s.projectLine}>
          Active project: <b>{project.title}</b>
          {" "}<code style={s.code}>{project.project_id.slice(0, 8)}…</code>
        </p>
      </div>
    </div>
  );
}

const s: Record<string, React.CSSProperties> = {
  root: {
    height: "100%",
    display: "flex", alignItems: "flex-start", justifyContent: "center",
    padding: "48px 24px",
  },
  card: {
    width: "min(640px, 100%)",
    display: "flex", flexDirection: "column", gap: 12,
  },
  stageNum: {
    margin: 0,
    fontSize: 11, fontWeight: 600,
    textTransform: "uppercase", letterSpacing: "0.1em",
    color: "var(--color-amber-600)",
  },
  title: {
    margin: 0,
    fontFamily: "var(--font-prose)",
    fontSize: 32, fontWeight: 700, lineHeight: 1.2,
    color: "var(--color-neutral-900)",
  },
  blurb: {
    margin: "8px 0 16px",
    fontSize: 15, lineHeight: 1.6,
    color: "var(--color-neutral-700)",
  },
  section: {
    padding: 16,
    background: "var(--color-neutral-50)",
    border: "1px solid var(--color-neutral-200)",
    borderRadius: 6,
  },
  sectionH: {
    margin: "0 0 8px",
    fontSize: 11, fontWeight: 600,
    textTransform: "uppercase", letterSpacing: "0.06em",
    color: "var(--color-neutral-500)",
  },
  list: { listStyle: "none", margin: 0, padding: 0, display: "flex", flexDirection: "column", gap: 6 },
  listItem: {
    display: "flex", alignItems: "center", gap: 8,
    fontSize: 13, color: "var(--color-neutral-800)",
  },
  bullet: {
    width: 4, height: 4, borderRadius: "50%",
    background: "var(--color-amber-500)",
    flexShrink: 0,
  },
  threshold: {
    margin: "12px 0 0",
    fontSize: 12, color: "var(--color-neutral-600)",
  },
  eta: {
    margin: 0,
    padding: "12px 16px",
    background: "rgba(245,158,11,0.08)",
    border: "1px solid rgba(245,158,11,0.3)",
    borderRadius: 4,
    fontSize: 13, color: "var(--color-neutral-800)",
  },
  projectLine: {
    margin: "16px 0 0",
    fontSize: 12, color: "var(--color-neutral-500)",
    fontFamily: "var(--font-mono)",
  },
  code: {
    fontFamily: "var(--font-mono)", fontSize: 11,
    padding: "1px 4px",
    background: "var(--color-neutral-100)", borderRadius: 3,
  },
};
