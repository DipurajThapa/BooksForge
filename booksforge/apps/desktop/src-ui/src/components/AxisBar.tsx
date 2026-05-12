/**
 * Shared score-axis bar used by every Phase C quality-gate panel.
 *
 * Renders a 0-10 score as a coloured progress bar with a vertical
 * floor marker at the gate threshold. Originally triplicated across
 * Stage1 (concept-scorer), Stage5 (character-critic), and Stage7
 * (structure-critic); this is the single source of truth.
 *
 * The component carries its own CSS-in-JS styles so consumers only
 * need to import `<AxisBar>` and the AXIS_FLOOR / COMPOSITE_THRESHOLD
 * constants. The thresholds match `booksforge_domain::quality_gate`.
 */
import type { CSSProperties } from "react";

/** Mirror of `booksforge_domain::quality_gate::AXIS_FLOOR`. */
export const AXIS_FLOOR = 7.0;
/** Mirror of `booksforge_domain::quality_gate::COMPOSITE_THRESHOLD`. */
export const COMPOSITE_THRESHOLD = 8.5;

export interface AxisLike {
  score: number;
  reason?: string;
}

export interface AxisBarProps {
  label: string;
  axis: AxisLike;
  /** Override the gate threshold for one-off uses. Defaults to AXIS_FLOOR. */
  threshold?: number;
}

export function AxisBar({ label, axis, threshold = AXIS_FLOOR }: AxisBarProps) {
  const pct = Math.min(100, Math.max(0, axis.score * 10));
  const colour =
    axis.score >= threshold
      ? "var(--color-green-500, #22c55e)"
      : "var(--color-red-500, #ef4444)";
  return (
    <div style={s.axisRow} title={axis.reason ?? ""}>
      <div style={s.axisHeader}>
        <span style={s.axisLabel}>{label}</span>
        <span style={s.axisScore}>{axis.score.toFixed(1)}</span>
      </div>
      <div style={s.axisTrack}>
        <div style={{ ...s.axisFill, width: `${pct}%`, background: colour }} />
        <div style={{ ...s.axisFloor, left: `${threshold * 10}%` }} />
      </div>
      {axis.reason && <span style={s.axisReason}>{axis.reason}</span>}
    </div>
  );
}

const s: Record<string, CSSProperties> = {
  axisRow: { display: "flex", flexDirection: "column", gap: 4 },
  axisHeader: {
    display: "flex", justifyContent: "space-between", alignItems: "baseline",
    gap: 8,
  },
  axisLabel: {
    fontSize: 10, fontWeight: 600,
    color: "var(--color-neutral-700)",
    textTransform: "uppercase", letterSpacing: "0.04em",
  },
  axisScore: {
    fontFamily: "var(--font-mono)", fontSize: 12, fontWeight: 600,
    color: "var(--color-neutral-900)",
    fontVariantNumeric: "tabular-nums",
  },
  axisTrack: {
    position: "relative",
    height: 5, background: "var(--color-neutral-200)",
    borderRadius: 3, overflow: "hidden",
  },
  axisFill: {
    height: "100%",
    transition: "width 200ms ease, background 200ms ease",
  },
  axisFloor: {
    position: "absolute", top: -2, bottom: -2, width: 1,
    background: "var(--color-neutral-500)",
    pointerEvents: "none",
  },
  axisReason: {
    fontSize: 11, color: "var(--color-neutral-600)",
    fontStyle: "italic", lineHeight: 1.4,
  },
};
