/**
 * ScorePanel — shared building blocks for the three Phase-C
 * quality-gate panels (concept-scorer on Stage 1, character-critic
 * on Stage 5, structure-critic on Stage 7).
 *
 * Before F3 each stage rendered its own composite read-out, findings
 * list, and edits list with subtle visual drift (different fonts,
 * spacings, badge colours). This file is the single source of truth
 * for those three repeating blocks. The stages keep their own
 * proposal shapes + pass logic; they just plug the shared parts in.
 *
 * The pieces:
 *   - `<ScoreSummary>`  big composite number + label + pass/fail tint
 *   - `<FindingsList>`  severity-coloured findings rows
 *   - `<EditsList>`     edit suggestions with optional "Apply" slot
 *
 * `AxisBar` already lives in its own file and is reused as-is here.
 */
import type { CSSProperties, ReactNode } from "react";
import { AxisBar, type AxisLike } from "./AxisBar";

// ── ScoreSummary ────────────────────────────────────────────────────────

export interface ScoreSummaryProps {
  /** 0–10 composite score. Rounded to one decimal for display. */
  composite:    number;
  /** Whether the gate passes — drives colour. */
  passing:      boolean;
  /** Right-side stat list (e.g. "X / 5 axes ≥ 7", "0 blocking
   *  findings", "3 suggested edits"). Each child is one stat line. */
  stats?:       ReactNode;
}

/**
 * The signature score readout used at the top of every gate panel.
 * Composite is rendered in the prose serif at 40px; the right slot
 * is a column of one-line stats the caller fills.
 */
export function ScoreSummary({ composite, passing, stats }: ScoreSummaryProps) {
  return (
    <div style={s.summaryRow}>
      <div style={s.summaryScoreCol}>
        <div style={{
          ...s.summaryBig,
          color: passing
            ? "var(--color-green-700, #15803d)"
            : "var(--color-amber-700, #b45309)",
        }}>
          {composite.toFixed(1)}
          <span style={s.summaryDenom}>/10</span>
        </div>
        <div style={s.summaryLabel}>composite</div>
      </div>
      {stats && <div style={s.summaryStats}>{stats}</div>}
    </div>
  );
}

// ── FindingsList ────────────────────────────────────────────────────────

export interface Finding {
  kind:     string;
  message:  string;
  /** "error" | "warning" | anything else (treated as warning). */
  severity: string;
}

export interface FindingsListProps {
  title:    string;
  findings: Finding[];
}

/**
 * Renders a typed list of findings with severity-coloured rows. If
 * the list is empty, nothing is rendered (no empty-state heading
 * noise — caller decides whether to show their own placeholder).
 */
export function FindingsList({ title, findings }: FindingsListProps) {
  if (findings.length === 0) return null;
  return (
    <div style={s.findingsBlock}>
      <h4 style={s.blockHeading}>{title} ({findings.length})</h4>
      <ul style={s.list}>
        {findings.map((f, i) => (
          <li
            key={i}
            style={{
              ...s.findingRow,
              ...(f.severity === "error" ? s.findingErr : s.findingWarn),
            }}
          >
            <span style={s.findingKind}>{f.kind}</span>
            <span>{f.message}</span>
          </li>
        ))}
      </ul>
    </div>
  );
}

// ── EditsList ───────────────────────────────────────────────────────────

export interface EditEntry {
  /** Field name or location label (e.g. "premise", "Ch 3 · beat 2"). */
  field:        string;
  /** Human-readable critique. */
  suggestion:   string;
  /** Optional concrete replacement. When present, the caller's
   *  `renderApply` slot decides whether to render an Apply button. */
  replacement?: string;
}

export interface EditsListProps {
  title:   string;
  edits:   EditEntry[];
  /** Optional per-edit Apply slot. Called for each edit; return
   *  `null` to suppress the button (e.g. structural-only edits). */
  renderApply?: (edit: EditEntry) => ReactNode;
  /** Optional footer hint shown below the list (e.g. for the
   *  structure-critic the edits are advisory, not auto-apply). */
  footerHint?: string;
}

/**
 * Renders an edit suggestion list with an optional per-row Apply
 * slot. The shape matches the three stages' edit payloads: each
 * has a field/location label, a human suggestion, and an optional
 * concrete replacement.
 */
export function EditsList({ title, edits, renderApply, footerHint }: EditsListProps) {
  if (edits.length === 0) return null;
  return (
    <div style={s.editsBlock}>
      <h4 style={s.blockHeading}>{title} ({edits.length})</h4>
      <ul style={s.list}>
        {edits.map((edit, i) => (
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
            {renderApply && renderApply(edit)}
          </li>
        ))}
      </ul>
      {footerHint && <p style={s.editsHint}>{footerHint}</p>}
    </div>
  );
}

// ── AxisGrid (small convenience) ────────────────────────────────────────

export interface AxisGridProps {
  axes: Array<[label: string, axis: AxisLike]>;
  /** Override the gate threshold per use. Defaults to AxisBar's AXIS_FLOOR. */
  threshold?: number;
}

/**
 * Two-column responsive grid of `<AxisBar>` components. Used by
 * every gate panel; extracted so the gap / column count is
 * consistent across stages.
 */
export function AxisGrid({ axes, threshold }: AxisGridProps) {
  return (
    <div style={s.axisGrid}>
      {axes.map(([label, axis]) => (
        <AxisBar key={label} label={label} axis={axis} threshold={threshold} />
      ))}
    </div>
  );
}

// ── Styles ──────────────────────────────────────────────────────────────

const s: Record<string, CSSProperties> = {
  summaryRow: {
    display: "flex", gap: 24, alignItems: "center",
    flexWrap: "wrap",
  },
  summaryScoreCol: {
    display: "flex", alignItems: "baseline", gap: 8,
  },
  summaryBig: {
    fontFamily: "var(--font-prose, serif)",
    fontSize: 40, fontWeight: 700, lineHeight: 1,
    fontVariantNumeric: "tabular-nums",
  },
  summaryDenom: {
    fontSize: 16, fontWeight: 500,
    color: "var(--color-neutral-500)",
    marginLeft: 4,
  },
  summaryLabel: {
    fontSize: 11, fontWeight: 600,
    textTransform: "uppercase", letterSpacing: "0.08em",
    color: "var(--color-neutral-500)",
  },
  summaryStats: {
    display: "flex", flexDirection: "column", gap: 2,
    fontSize: 12, color: "var(--color-neutral-700)",
  },
  blockHeading: {
    margin: 0,
    fontSize: 11, fontWeight: 600,
    textTransform: "uppercase", letterSpacing: "0.06em",
    color: "var(--color-neutral-500)",
  },
  list: {
    listStyle: "none", margin: 0, padding: 0,
    display: "flex", flexDirection: "column", gap: 4,
  },
  // Findings ────────────────────────────────────────────────────────────
  findingsBlock: {
    display: "flex", flexDirection: "column", gap: 6,
  },
  findingRow: {
    display: "flex", gap: 10, alignItems: "flex-start",
    padding: "6px 10px",
    borderRadius: 4,
    fontSize: 12, lineHeight: 1.5,
  },
  findingErr: {
    background: "rgba(220,38,38,0.06)",
    border: "1px solid rgba(220,38,38,0.25)",
    color: "var(--color-red-700, #b91c1c)",
  },
  findingWarn: {
    background: "rgba(245,158,11,0.08)",
    border: "1px solid rgba(245,158,11,0.3)",
    color: "var(--color-amber-700, #b45309)",
  },
  findingKind: {
    fontFamily: "var(--font-mono)", fontSize: 10,
    fontWeight: 600, textTransform: "uppercase", letterSpacing: "0.04em",
    flexShrink: 0,
  },
  // Edits ───────────────────────────────────────────────────────────────
  editsBlock: {
    display: "flex", flexDirection: "column", gap: 6,
  },
  editRow: {
    display: "flex", justifyContent: "space-between", alignItems: "flex-start",
    gap: 12,
    padding: "8px 12px",
    background: "#fff",
    border: "1px solid var(--color-neutral-200)",
    borderRadius: 4,
  },
  editLeft: {
    display: "flex", flexDirection: "column", gap: 2,
    flex: 1, minWidth: 0,
  },
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
  editsHint: {
    margin: 0,
    fontSize: 11, color: "var(--color-neutral-500)", lineHeight: 1.5,
    fontStyle: "italic",
  },
  // Axis grid ───────────────────────────────────────────────────────────
  axisGrid: {
    display: "grid",
    gridTemplateColumns: "1fr 1fr",
    gap: "8px 16px",
  },
};
