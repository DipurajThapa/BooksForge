/**
 * StageRail — the left-side vertical strip showing the 6 MVP stages
 * with traffic-light status. This is the visible representation of
 * the writer's journey; clicking a stage jumps the editor's body to
 * that stage's panel.
 *
 * Per `book-output/design/WRITER_JOURNEY_REDESIGN_2026-05.md` §11.
 *
 * Stage statuses:
 *   - "locked"      — gated by an earlier stage; click is a no-op
 *   - "available"   — ready to start; orange dot
 *   - "in_progress" — partially complete; pulsing dot
 *   - "passed"      — quality gate cleared; green dot
 *   - "skipped"     — explicitly overridden; grey dot with bar
 *   - "failed"      — gate score < threshold; red dot
 *
 * The rail does not enforce gates itself — it visualises status the
 * editor passes in. Gate enforcement lives in each stage's panel.
 */
import React from "react";

export type StageStatus =
  | "locked" | "available" | "in_progress" | "passed" | "skipped" | "failed";

export type StageId =
  | "setup" | "audience" | "characters" | "outline" | "drafting" | "export";

export interface StageInfo {
  id:     StageId;
  label:  string;
  hint:   string;
  status: StageStatus;
}

/** The MVP's 6 stages, in order. Stage 1 is always first. */
export const MVP_STAGES: ReadonlyArray<Omit<StageInfo, "status">> = [
  { id: "setup",      label: "Book Setup",   hint: "Title, premise, format" },
  { id: "audience",   label: "Audience",     hint: "Who is this book for?" },
  { id: "characters", label: "Bibles",       hint: "Characters + world / setting" },
  { id: "outline",    label: "Outline",      hint: "Parts → chapters → scenes" },
  { id: "drafting",   label: "Drafting",     hint: "Prose for every scene" },
  { id: "export",     label: "Format & Ship", hint: "Cover, audit, export" },
];

interface Props {
  stages:    StageInfo[];
  active:    StageId;
  onSelect:  (id: StageId) => void;
}

export default function StageRail({ stages, active, onSelect }: Props) {
  return (
    <nav style={s.rail} aria-label="Writing stages">
      <h2 style={s.heading}>Journey</h2>
      <ol style={s.list}>
        {stages.map((stage, i) => {
          const isActive = stage.id === active;
          const isLocked = stage.status === "locked";
          return (
            <li key={stage.id}>
              <button
                style={{
                  ...s.item,
                  ...(isActive ? s.itemActive : {}),
                  ...(isLocked ? s.itemLocked : {}),
                }}
                onClick={() => !isLocked && onSelect(stage.id)}
                disabled={isLocked}
                aria-current={isActive ? "step" : undefined}
                aria-label={`Stage ${i + 1}: ${stage.label} — ${stage.status}`}
              >
                <span style={s.itemNumber}>{i + 1}</span>
                <span style={statusDotStyle(stage.status)} aria-hidden="true" />
                <span style={s.itemBody}>
                  <span style={s.itemLabel}>{stage.label}</span>
                  <span style={s.itemHint}>{stage.hint}</span>
                </span>
              </button>
            </li>
          );
        })}
      </ol>
    </nav>
  );
}

function statusDotStyle(status: StageStatus): React.CSSProperties {
  const base: React.CSSProperties = {
    width: 10, height: 10, borderRadius: "50%",
    flexShrink: 0,
  };
  switch (status) {
    case "passed":
      return { ...base, background: "var(--color-green-500, #22c55e)" };
    case "failed":
      return { ...base, background: "var(--color-red-500, #ef4444)" };
    case "in_progress":
      return {
        ...base,
        background: "var(--color-amber-500, #f59e0b)",
        animation: "bf-stage-pulse 1.6s ease-in-out infinite",
      };
    case "available":
      return {
        ...base,
        background: "transparent",
        border: "2px solid var(--color-amber-500, #f59e0b)",
      };
    case "skipped":
      return {
        ...base,
        background: "var(--color-neutral-300)",
        position: "relative",
      };
    case "locked":
    default:
      return { ...base, background: "var(--color-neutral-200)" };
  }
}

// Inject the keyframe once on module load (Vite-HMR safe).
if (typeof document !== "undefined" && !document.getElementById("bf-stage-anim")) {
  const styleEl = document.createElement("style");
  styleEl.id = "bf-stage-anim";
  styleEl.textContent = `@keyframes bf-stage-pulse {
    0%, 100% { opacity: 1; transform: scale(1); }
    50%      { opacity: 0.55; transform: scale(1.12); }
  }`;
  document.head.appendChild(styleEl);
}

const s: Record<string, React.CSSProperties> = {
  rail: {
    width: 220,
    flexShrink: 0,
    padding: "20px 12px",
    borderRight: "1px solid var(--color-neutral-200)",
    background: "var(--color-neutral-50)",
    display: "flex", flexDirection: "column", gap: 12,
    fontFamily: "var(--font-ui)",
  },
  heading: {
    fontSize: 11, fontWeight: 600,
    letterSpacing: "0.08em", textTransform: "uppercase",
    color: "var(--color-neutral-500)",
    margin: "0 0 4px 8px",
  },
  list: {
    listStyle: "none", margin: 0, padding: 0,
    display: "flex", flexDirection: "column", gap: 2,
  },
  item: {
    display: "grid",
    gridTemplateColumns: "20px 14px 1fr",
    alignItems: "center",
    gap: 10,
    width: "100%",
    padding: "8px 10px",
    background: "transparent",
    border: "1px solid transparent",
    borderRadius: 6,
    cursor: "pointer",
    textAlign: "left",
    fontFamily: "inherit",
    color: "var(--color-neutral-900)",
  },
  itemActive: {
    background: "var(--color-amber-50, #fffbeb)",
    borderColor: "var(--color-amber-300)",
  },
  itemLocked: {
    cursor: "not-allowed",
    color: "var(--color-neutral-400)",
  },
  itemNumber: {
    fontSize: 11, fontWeight: 600,
    color: "var(--color-neutral-400)",
    fontVariantNumeric: "tabular-nums",
    textAlign: "right",
  },
  itemBody: {
    display: "flex", flexDirection: "column", gap: 1,
    minWidth: 0,
  },
  itemLabel: {
    fontSize: 13, fontWeight: 500,
    color: "inherit",
  },
  itemHint: {
    fontSize: 11,
    color: "var(--color-neutral-500)",
    overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap",
  },
};
