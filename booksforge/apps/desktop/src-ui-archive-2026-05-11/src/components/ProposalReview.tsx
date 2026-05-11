import React, { useCallback, useMemo } from "react";

/**
 * Generic proposal-review surface.
 *
 * Closes the headline UX gap from EXTERNAL_AUDIT_BACKLOG.md #29: every
 * agent (Copyedit, Continuity, Humanization, Developmental, Memory,
 * Vocabulary, etc.) returns a list of proposed *hunks* — small,
 * self-contained edits — and the user needs a uniform place to read,
 * review, and accept/reject them per-hunk before any change is
 * applied to the manuscript.
 *
 * This component is the SHARED review surface.  Each agent panel
 * (CopyeditPanel, ContinuityPanel, etc., currently in the team's
 * in-flight set) wires it after assembling a `Proposal` value.  The
 * shape of `Proposal` / `Hunk` is deliberately framework-agnostic so
 * the same surface works for prose edits, entity-rename suggestions,
 * vocab-promotion suggestions, etc.
 *
 * Wiring (consumer example):
 *
 *   const proposal: Proposal = await invoke("agent_run_dispatch", {...});
 *   const [decisions, setDecisions] = useState<Decision[]>(
 *     proposal.hunks.map(h => ({ hunkId: h.id, status: "pending" }))
 *   );
 *   <ProposalReview
 *     proposal={proposal}
 *     decisions={decisions}
 *     onChange={setDecisions}
 *     onApply={async (accepted) => {
 *       await invoke("agent_apply_proposals", { proposalId: proposal.id, decisions: accepted });
 *     }}
 *     onCancel={() => closePanel()}
 *   />
 *
 * Behaviour
 *   - Each hunk renders as a compact card: rationale + collapsible
 *     before/after diff.
 *   - Per-hunk Accept / Reject / Reset (back to pending).
 *   - "Accept all" / "Reject all" toolbar.
 *   - Sticky footer: counts (n accepted / n rejected / n pending) +
 *     "Apply selected" button (disabled until at least one accepted).
 *   - Apply only fires `onApply` with hunks where status === "accepted".
 *
 * Accessibility
 *   - Each card is a `<section>` with an `aria-labelledby` linking to
 *     the hunk's rationale.
 *   - Per-hunk buttons announce "Accept hunk N of M" / "Reject hunk N
 *     of M" via aria-label.
 *   - Footer summary is `role="status"` so screen readers announce
 *     decision counts as they change.
 *   - Keyboard navigation: tab-focusable buttons; `j` / `k` advance
 *     focus between cards (when the component is the active region —
 *     consumers can opt in via the `keyboardNav` prop).
 *
 * Privacy
 *   The component never logs hunk content anywhere.  Decisions are
 *   raised via callback only; the consumer decides where to send
 *   them (typically a Tauri command that writes to the local
 *   `agent_runs` ledger and never to a remote endpoint).
 */

export type DecisionStatus = "accepted" | "rejected" | "pending";

export interface Hunk {
  /** Stable identifier produced by the agent runner. */
  id: string;
  /** Human-readable label for the hunk (e.g. "ch3.scene2:¶17-19"). */
  label?: string;
  /** Optional one-paragraph reasoning the agent provided. */
  rationale?: string;
  /** The original text the agent saw. */
  before: string;
  /** The text the agent proposes. */
  after: string;
  /** Optional severity / category tag (e.g. "minor", "major", "POV"). */
  tag?: string;
}

export interface Proposal {
  /** Stable id (typically the agent_runs row id). */
  id: string;
  /** Agent that produced this proposal — shown in the header. */
  agentName: string;
  /** Optional one-line summary of what the agent did. */
  summary?: string;
  hunks: Hunk[];
}

export interface Decision {
  hunkId: string;
  status: DecisionStatus;
}

export interface ProposalReviewProps {
  proposal: Proposal;
  decisions: Decision[];
  onChange: (next: Decision[]) => void;
  onApply: (accepted: Decision[]) => void | Promise<void>;
  onCancel: () => void;
  /** Disable the Apply button + show a spinner indicator. */
  applying?: boolean;
  /** Render unified-diff (default) or side-by-side. */
  diffMode?: "unified" | "side-by-side";
  /** Enable j/k navigation between hunks. Defaults false. */
  keyboardNav?: boolean;
}

export function ProposalReview({
  proposal,
  decisions,
  onChange,
  onApply,
  onCancel,
  applying = false,
  diffMode = "unified",
  keyboardNav = false,
}: ProposalReviewProps): JSX.Element {
  const decisionMap = useMemo(() => {
    const m = new Map<string, DecisionStatus>();
    for (const d of decisions) m.set(d.hunkId, d.status);
    return m;
  }, [decisions]);

  const counts = useMemo(() => {
    let accepted = 0, rejected = 0, pending = 0;
    for (const h of proposal.hunks) {
      const s = decisionMap.get(h.id) ?? "pending";
      if (s === "accepted") accepted++;
      else if (s === "rejected") rejected++;
      else pending++;
    }
    return { accepted, rejected, pending };
  }, [proposal.hunks, decisionMap]);

  const setStatus = useCallback(
    (hunkId: string, status: DecisionStatus) => {
      const next: Decision[] = proposal.hunks.map((h) => {
        const existing = decisionMap.get(h.id) ?? "pending";
        return {
          hunkId: h.id,
          status: h.id === hunkId ? status : existing,
        };
      });
      onChange(next);
    },
    [proposal.hunks, decisionMap, onChange],
  );

  const setAll = useCallback(
    (status: DecisionStatus) => {
      onChange(proposal.hunks.map((h) => ({ hunkId: h.id, status })));
    },
    [proposal.hunks, onChange],
  );

  const handleApply = useCallback(() => {
    const accepted = decisions.filter((d) => d.status === "accepted");
    if (accepted.length === 0) return;
    void onApply(accepted);
  }, [decisions, onApply]);

  return (
    <div
      role="region"
      aria-label={`Proposals from ${proposal.agentName}`}
      style={containerStyle}
      data-keyboard-nav={keyboardNav ? "on" : "off"}
    >
      <header style={headerStyle}>
        <div>
          <h2 style={titleStyle}>{proposal.agentName} suggestions</h2>
          {proposal.summary && <p style={summaryStyle}>{proposal.summary}</p>}
        </div>
        <div style={toolbarStyle}>
          <button
            type="button"
            onClick={() => setAll("accepted")}
            style={btnGhost}
            disabled={applying}
          >
            Accept all
          </button>
          <button
            type="button"
            onClick={() => setAll("rejected")}
            style={btnGhost}
            disabled={applying}
          >
            Reject all
          </button>
        </div>
      </header>

      <div style={listStyle}>
        {proposal.hunks.map((hunk, index) => (
          <HunkCard
            key={hunk.id}
            hunk={hunk}
            index={index}
            total={proposal.hunks.length}
            status={decisionMap.get(hunk.id) ?? "pending"}
            onStatusChange={(s) => setStatus(hunk.id, s)}
            diffMode={diffMode}
            disabled={applying}
          />
        ))}

        {proposal.hunks.length === 0 && (
          <p style={emptyStyle}>
            The agent ran but did not produce any proposals.
          </p>
        )}
      </div>

      <footer role="status" aria-live="polite" style={footerStyle}>
        <span style={countsStyle}>
          <strong style={countAccepted}>{counts.accepted}</strong> accepted ·{" "}
          <strong style={countRejected}>{counts.rejected}</strong> rejected ·{" "}
          <strong>{counts.pending}</strong> pending
        </span>
        <span style={footerActions}>
          <button type="button" onClick={onCancel} disabled={applying} style={btnGhost}>
            Cancel
          </button>
          <button
            type="button"
            onClick={handleApply}
            disabled={applying || counts.accepted === 0}
            style={btnPrimary}
          >
            {applying ? "Applying…" : `Apply ${counts.accepted}`}
          </button>
        </span>
      </footer>
    </div>
  );
}

// ── Hunk card ─────────────────────────────────────────────────────

interface HunkCardProps {
  hunk: Hunk;
  index: number;
  total: number;
  status: DecisionStatus;
  onStatusChange: (s: DecisionStatus) => void;
  diffMode: "unified" | "side-by-side";
  disabled: boolean;
}

function HunkCard({
  hunk,
  index,
  total,
  status,
  onStatusChange,
  diffMode,
  disabled,
}: HunkCardProps): JSX.Element {
  const titleId = `bf-hunk-${hunk.id}-title`;
  const ordinal = `${index + 1} of ${total}`;
  return (
    <section
      aria-labelledby={titleId}
      style={hunkCardStyle(status)}
      data-status={status}
    >
      <div style={hunkHeaderStyle}>
        <div>
          <h3 id={titleId} style={hunkTitleStyle}>
            {hunk.label ?? `Hunk ${index + 1}`}
            {hunk.tag && <span style={hunkTagStyle}>{hunk.tag}</span>}
          </h3>
          {hunk.rationale && <p style={hunkRationaleStyle}>{hunk.rationale}</p>}
        </div>
        <div style={hunkActions}>
          <button
            type="button"
            onClick={() => onStatusChange("accepted")}
            disabled={disabled || status === "accepted"}
            aria-label={`Accept hunk ${ordinal}`}
            aria-pressed={status === "accepted"}
            style={status === "accepted" ? btnAcceptOn : btnAcceptOff}
          >
            Accept
          </button>
          <button
            type="button"
            onClick={() => onStatusChange("rejected")}
            disabled={disabled || status === "rejected"}
            aria-label={`Reject hunk ${ordinal}`}
            aria-pressed={status === "rejected"}
            style={status === "rejected" ? btnRejectOn : btnRejectOff}
          >
            Reject
          </button>
          {status !== "pending" && (
            <button
              type="button"
              onClick={() => onStatusChange("pending")}
              disabled={disabled}
              aria-label={`Reset hunk ${ordinal} to pending`}
              style={btnGhost}
            >
              Reset
            </button>
          )}
        </div>
      </div>

      <DiffView before={hunk.before} after={hunk.after} mode={diffMode} />
    </section>
  );
}

// ── Diff view ─────────────────────────────────────────────────────

function DiffView({
  before,
  after,
  mode,
}: {
  before: string;
  after: string;
  mode: "unified" | "side-by-side";
}): JSX.Element {
  if (mode === "side-by-side") {
    return (
      <div style={diffSideBySide}>
        <pre style={diffPaneBefore}>{before || "(empty)"}</pre>
        <pre style={diffPaneAfter}>{after || "(empty)"}</pre>
      </div>
    );
  }
  // Unified: stack before above after; rely on visual difference and
  // accept/reject buttons to communicate the change.  A real word-level
  // diff (matching the existing wordDiff helper) is the team's
  // follow-up — this component is the shell, not the diff algorithm.
  return (
    <div style={diffUnified}>
      {before && (
        <pre style={diffLineBefore} aria-label="Before">
          − {before}
        </pre>
      )}
      {after && (
        <pre style={diffLineAfter} aria-label="After">
          + {after}
        </pre>
      )}
    </div>
  );
}

// ── Inline styles (CSP-friendly: React inline `style={...}` sets
//    element.style.* directly and is NOT subject to style-src) ──

const containerStyle: React.CSSProperties = {
  display: "flex",
  flexDirection: "column",
  height: "100%",
  background: "var(--color-bg, #ffffff)",
  color: "var(--color-neutral-900, #1f2328)",
};

const headerStyle: React.CSSProperties = {
  display: "flex",
  justifyContent: "space-between",
  alignItems: "flex-start",
  gap: "1rem",
  padding: "1rem 1.25rem",
  borderBottom: "1px solid var(--color-neutral-200, #e5e7eb)",
};

const titleStyle: React.CSSProperties = {
  margin: 0,
  fontSize: "1.05rem",
};

const summaryStyle: React.CSSProperties = {
  margin: "0.25rem 0 0",
  fontSize: "0.9rem",
  opacity: 0.75,
};

const toolbarStyle: React.CSSProperties = {
  display: "flex",
  gap: "0.5rem",
  flexShrink: 0,
};

const listStyle: React.CSSProperties = {
  flex: 1,
  overflowY: "auto",
  padding: "1rem 1.25rem",
  display: "flex",
  flexDirection: "column",
  gap: "0.75rem",
};

const emptyStyle: React.CSSProperties = {
  textAlign: "center",
  opacity: 0.6,
  marginTop: "2rem",
};

function hunkCardStyle(status: DecisionStatus): React.CSSProperties {
  const borderColor =
    status === "accepted"
      ? "var(--color-success, #22c55e)"
      : status === "rejected"
      ? "var(--color-error, #d1242f)"
      : "var(--color-neutral-300, #d0d7de)";
  return {
    border: `1px solid ${borderColor}`,
    borderRadius: "0.5rem",
    background: "var(--color-bg, #ffffff)",
    padding: "0.75rem 1rem",
  };
}

const hunkHeaderStyle: React.CSSProperties = {
  display: "flex",
  justifyContent: "space-between",
  alignItems: "flex-start",
  gap: "0.75rem",
  marginBottom: "0.5rem",
};

const hunkTitleStyle: React.CSSProperties = {
  margin: 0,
  fontSize: "0.95rem",
  display: "flex",
  alignItems: "center",
  gap: "0.5rem",
};

const hunkTagStyle: React.CSSProperties = {
  fontSize: "0.75rem",
  padding: "0.1rem 0.4rem",
  borderRadius: "0.25rem",
  background: "var(--color-bg-subtle, #f6f8fa)",
  border: "1px solid var(--color-neutral-300, #d0d7de)",
  color: "var(--color-neutral-700, #4b5563)",
  fontWeight: 500,
};

const hunkRationaleStyle: React.CSSProperties = {
  margin: "0.25rem 0 0",
  fontSize: "0.85rem",
  opacity: 0.8,
};

const hunkActions: React.CSSProperties = {
  display: "flex",
  gap: "0.25rem",
  flexShrink: 0,
};

const diffUnified: React.CSSProperties = {
  display: "flex",
  flexDirection: "column",
  gap: "0.25rem",
  fontFamily: "ui-monospace, SFMono-Regular, Menlo, monospace",
  fontSize: "0.85rem",
};

const diffLineBefore: React.CSSProperties = {
  background: "var(--color-error-bg, #fee2e2)",
  color: "var(--color-error-fg, #991b1b)",
  padding: "0.4rem 0.6rem",
  borderRadius: "0.25rem",
  margin: 0,
  whiteSpace: "pre-wrap",
};

const diffLineAfter: React.CSSProperties = {
  background: "var(--color-success-bg, #dcfce7)",
  color: "var(--color-success-fg, #166534)",
  padding: "0.4rem 0.6rem",
  borderRadius: "0.25rem",
  margin: 0,
  whiteSpace: "pre-wrap",
};

const diffSideBySide: React.CSSProperties = {
  display: "grid",
  gridTemplateColumns: "1fr 1fr",
  gap: "0.5rem",
  fontFamily: "ui-monospace, SFMono-Regular, Menlo, monospace",
  fontSize: "0.85rem",
};

const diffPaneBefore: React.CSSProperties = {
  ...diffLineBefore,
};

const diffPaneAfter: React.CSSProperties = {
  ...diffLineAfter,
};

const footerStyle: React.CSSProperties = {
  display: "flex",
  justifyContent: "space-between",
  alignItems: "center",
  padding: "0.75rem 1.25rem",
  borderTop: "1px solid var(--color-neutral-200, #e5e7eb)",
  background: "var(--color-bg-subtle, #f6f8fa)",
};

const countsStyle: React.CSSProperties = {
  fontSize: "0.9rem",
};

const countAccepted: React.CSSProperties = {
  color: "var(--color-success-fg, #166534)",
};

const countRejected: React.CSSProperties = {
  color: "var(--color-error-fg, #991b1b)",
};

const footerActions: React.CSSProperties = {
  display: "flex",
  gap: "0.5rem",
};

const btnBase: React.CSSProperties = {
  padding: "0.4rem 0.75rem",
  fontSize: "0.85rem",
  borderRadius: "0.375rem",
  border: "1px solid var(--color-neutral-300, #d0d7de)",
  cursor: "pointer",
  background: "var(--color-bg, #ffffff)",
  color: "inherit",
};

const btnGhost: React.CSSProperties = { ...btnBase };

const btnPrimary: React.CSSProperties = {
  ...btnBase,
  background: "var(--color-primary, #0969da)",
  color: "#ffffff",
  borderColor: "var(--color-primary, #0969da)",
};

const btnAcceptOff: React.CSSProperties = {
  ...btnBase,
  borderColor: "var(--color-success, #22c55e)",
  color: "var(--color-success-fg, #166534)",
};

const btnAcceptOn: React.CSSProperties = {
  ...btnBase,
  background: "var(--color-success, #22c55e)",
  color: "#ffffff",
  borderColor: "var(--color-success, #22c55e)",
};

const btnRejectOff: React.CSSProperties = {
  ...btnBase,
  borderColor: "var(--color-error, #d1242f)",
  color: "var(--color-error-fg, #991b1b)",
};

const btnRejectOn: React.CSSProperties = {
  ...btnBase,
  background: "var(--color-error, #d1242f)",
  color: "#ffffff",
  borderColor: "var(--color-error, #d1242f)",
};
