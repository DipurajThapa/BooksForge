/**
 * Phase 4 — Manuscript validators panel.
 *
 * One-click "Check manuscript" runner for the 16 deterministic validators
 * shipped in `booksforge-validator`.  Results are grouped by severity
 * and clickable to jump to the offending node.  The panel also shows the
 * pre-export gate result so the writer knows whether their book is
 * exportable as-is.
 */
import React, { useCallback, useEffect, useState } from "react";
import type { ValidatorIssueDto, ValidatorReportDto } from "@booksforge/shared-types";
import { useDialogA11y } from "../lib/useDialogA11y";
import { ipc } from "../lib/ipc";

interface Props {
  /** Closes the panel. */
  onClose: () => void;
  /** Optional callback fired when the user clicks an issue's node link.   */
  onSelectNode?: (nodeId: string) => void;
}

const SEVERITY_COLOR: Record<string, string> = {
  error:   "var(--color-error, #ef4444)",
  warning: "var(--color-amber-500, #f59e0b)",
  info:    "var(--color-blue-500, #3b82f6)",
};

const SEVERITY_LABEL: Record<string, string> = {
  error:   "Error",
  warning: "Warning",
  info:    "Info",
};

export default function ValidatorPanel({ onClose, onSelectNode }: Props) {
  const { dialogProps, titleId } = useDialogA11y(onClose);
  const [report, setReport] = useState<ValidatorReportDto | null>(null);
  const [running, setRunning] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [fixing, setFixing] = useState<string | null>(null); // issueKey of in-flight fix

  const runChecks = useCallback(async () => {
    setRunning(true);
    setError(null);
    try {
      const r = await ipc.validatorsRun();
      setReport(r);
    } catch (e) {
      setError(String(e));
    } finally {
      setRunning(false);
    }
  }, []);

  const handleApplyFix = useCallback(async (issue: ValidatorIssueDto) => {
    if (!issue.node_id) return;
    setFixing(issueKey(issue));
    setError(null);
    try {
      await ipc.validatorsApplyFix({
        validator_id: issue.validator_id,
        node_id:      issue.node_id,
      });
      // Re-run validators so the fixed issue drops off the list.
      await runChecks();
    } catch (e) {
      setError(String(e));
    } finally {
      setFixing(null);
    }
  }, [runChecks]);

  // Run automatically on first open.
  useEffect(() => { void runChecks(); }, [runChecks]);

  const grouped = groupBySeverity(report?.issues ?? []);

  return (
    <div style={s.overlay} role="presentation">
      <div {...dialogProps} style={s.dialog}>
        <header style={s.header}>
          <span id={titleId} style={s.title}>Manuscript checks</span>
          {report && <Status report={report} />}
          <button style={s.closeBtn} onClick={onClose} disabled={running} aria-label="Close validator panel">✕</button>
        </header>

        <div style={s.body}>
          {error && <div style={s.error}>{error}</div>}

          {running && !report && (
            <div style={s.empty}>Running 16 validators…</div>
          )}

          {report && report.issues.length === 0 && (
            <div style={s.passBox}>
              <strong>All clear.</strong> No issues found by any of the 16 shipped
              validators. Your manuscript is ready to export.
            </div>
          )}

          {(["error", "warning", "info"] as const).map((sev) =>
            (grouped[sev]?.length ?? 0) > 0 && (
              <Section
                key={sev}
                severity={sev}
                count={grouped[sev]!.length}
                issues={grouped[sev]!}
                onSelectNode={onSelectNode}
                onApplyFix={handleApplyFix}
                fixing={fixing}
              />
            )
          )}
        </div>

        <footer style={s.footer}>
          <button style={s.ghostBtn} onClick={runChecks} disabled={running}>
            {running ? "Running…" : "Re-run checks"}
          </button>
          <button style={s.primaryBtn} onClick={onClose}>Done</button>
        </footer>
      </div>
    </div>
  );
}

function Status({ report }: { report: ValidatorReportDto }) {
  const blocking = report.error_count > 0;
  const warning  = !blocking && report.warning_count > 0;
  const label = blocking ? "Blocked"   :
                warning  ? "Warnings"  :
                "Pass";
  const color = blocking ? SEVERITY_COLOR.error :
                warning  ? SEVERITY_COLOR.warning :
                "var(--color-success, #22c55e)";
  return (
    <span style={{ ...s.statusBadge, background: color }}>
      {label}
      <span style={s.statusCounts}>
        {report.error_count > 0   && ` · ${report.error_count} err`}
        {report.warning_count > 0 && ` · ${report.warning_count} warn`}
        {report.info_count > 0    && ` · ${report.info_count} info`}
      </span>
    </span>
  );
}

function Section({
  severity, count, issues, onSelectNode, onApplyFix, fixing,
}: {
  severity: "error" | "warning" | "info";
  count:    number;
  issues:   ValidatorIssueDto[];
  onSelectNode?: (id: string) => void;
  onApplyFix?:   (issue: ValidatorIssueDto) => Promise<void>;
  fixing?:       string | null;
}) {
  return (
    <section style={s.section}>
      <h3 style={s.sectionHeader}>
        <span
          style={{
            ...s.severityDot,
            background: SEVERITY_COLOR[severity],
          }}
        />
        {SEVERITY_LABEL[severity]} <span style={s.sectionCount}>· {count}</span>
      </h3>
      <ul style={s.issueList}>
        {issues.map((i, idx) => {
          const fixingThis = fixing === issueKey(i);
          return (
          <li key={`${i.code}-${idx}`} style={s.issueRow}>
            <div style={s.issueMain}>
              <span style={s.issueCode}>{i.code}</span>
              <span style={s.issueMessage}>{i.message}</span>
            </div>
            <div style={s.issueMeta}>
              {i.auto_fixable && i.node_id && onApplyFix && (
                <button
                  style={s.fixBtn}
                  onClick={() => onApplyFix(i)}
                  disabled={!!fixing}
                  title="Apply the deterministic fix to this scene"
                >
                  {fixingThis ? "Fixing…" : "Fix"}
                </button>
              )}
              {i.auto_fixable && !i.node_id && (
                <span style={s.fixHint} title="Auto-fixable but the issue has no scene scope">auto</span>
              )}
              {i.node_id && onSelectNode && (
                <button
                  style={s.jumpBtn}
                  onClick={() => onSelectNode(i.node_id!)}
                  title="Jump to this scene"
                >
                  Jump →
                </button>
              )}
            </div>
          </li>);
        })}
      </ul>
    </section>
  );
}

function issueKey(i: ValidatorIssueDto): string {
  return `${i.validator_id}:${i.node_id ?? ""}:${i.code}`;
}

function groupBySeverity(issues: ValidatorIssueDto[]): Record<string, ValidatorIssueDto[]> {
  return issues.reduce<Record<string, ValidatorIssueDto[]>>((acc, i) => {
    (acc[i.severity] ||= []).push(i);
    return acc;
  }, {});
}

const s: Record<string, React.CSSProperties> = {
  overlay: {
    position: "fixed", inset: 0, background: "rgba(0,0,0,0.55)",
    display: "flex", alignItems: "flex-start", justifyContent: "center",
    zIndex: 500, paddingTop: 48,
  },
  dialog: {
    background:    "var(--color-surface)",
    border:        "1px solid var(--color-border)",
    borderRadius:  8,
    width:         "min(96vw, 880px)",
    maxHeight:     "calc(100vh - 72px)",
    display:       "flex",
    flexDirection: "column",
    overflow:      "hidden",
    boxShadow:     "0 8px 32px rgba(0,0,0,0.25)",
  },
  header: {
    display: "flex", alignItems: "center", gap: 12,
    padding: "12px 16px", borderBottom: "1px solid var(--color-border)",
    flexShrink: 0,
  },
  title: { fontWeight: 600, fontSize: 14, color: "var(--color-text-primary)", flex: 1 },
  statusBadge: {
    display: "inline-flex", alignItems: "center", gap: 4,
    padding: "2px 10px", borderRadius: 99,
    fontSize: 11, fontWeight: 700, color: "#fff",
    textTransform: "uppercase", letterSpacing: "0.04em",
  },
  statusCounts: { fontWeight: 500, textTransform: "none", letterSpacing: 0 },
  closeBtn: {
    background: "none", border: "none", cursor: "pointer",
    fontSize: 16, color: "var(--color-text-secondary)", padding: "0 4px",
  },
  body: { padding: 16, overflowY: "auto", display: "flex", flexDirection: "column", gap: 16, flex: 1 },
  empty: { fontSize: 13, color: "var(--color-text-tertiary)", textAlign: "center", padding: 32 },
  passBox: {
    padding: "12px 14px",
    border: "1px solid var(--color-success, #22c55e)",
    borderRadius: 6,
    background: "var(--color-surface-raised)",
    fontSize: 13, color: "var(--color-text-primary)",
    lineHeight: 1.5,
  },
  error: {
    padding: "8px 12px", borderRadius: 4,
    border: "1px solid var(--color-error, #ef4444)",
    color:  "var(--color-error, #ef4444)",
    fontSize: 12, fontFamily: "var(--font-mono)",
  },
  section: {
    border: "1px solid var(--color-border)", borderRadius: 6,
    background: "var(--color-surface-raised)",
    padding: "8px 12px",
  },
  sectionHeader: {
    margin: 0, padding: "4px 0", fontSize: 13, fontWeight: 600,
    display: "flex", alignItems: "center", gap: 8,
    color: "var(--color-text-primary)",
  },
  severityDot: { width: 10, height: 10, borderRadius: "50%" },
  sectionCount: { fontWeight: 400, color: "var(--color-text-tertiary)" },
  issueList: { listStyle: "none", padding: 0, margin: 0, display: "flex", flexDirection: "column", gap: 4 },
  issueRow: {
    display: "flex", alignItems: "center", gap: 8,
    padding: "6px 8px", borderRadius: 4,
    background: "var(--color-surface)",
    border: "1px solid var(--color-border)",
  },
  issueMain: { display: "flex", flexDirection: "column", gap: 2, flex: 1, minWidth: 0 },
  issueCode: { fontSize: 10, fontFamily: "var(--font-mono)", color: "var(--color-text-tertiary)" },
  issueMessage: { fontSize: 13, color: "var(--color-text-primary)" },
  issueMeta: { display: "flex", alignItems: "center", gap: 6, flexShrink: 0 },
  fixHint: {
    fontSize: 10, padding: "1px 6px", borderRadius: 99,
    background: "var(--color-amber-50, #fffbeb)",
    color: "var(--color-amber-600, #d97706)",
    fontWeight: 600,
  },
  fixBtn: {
    background: "var(--color-amber-600)",
    border: "1px solid var(--color-amber-600)",
    borderRadius: 4, fontSize: 11, padding: "2px 10px",
    cursor: "pointer", color: "#fff", fontWeight: 600,
  },
  jumpBtn: {
    background: "none", border: "1px solid var(--color-border)",
    borderRadius: 4, fontSize: 11, padding: "2px 8px",
    cursor: "pointer", color: "var(--color-text-secondary)",
  },
  footer: {
    display: "flex", justifyContent: "flex-end", gap: 8,
    padding: "10px 16px", borderTop: "1px solid var(--color-border)",
    flexShrink: 0,
  },
  primaryBtn: {
    padding: "6px 14px", background: "var(--color-amber-600)", color: "#fff",
    border: "none", borderRadius: 4, fontSize: 13, fontWeight: 600, cursor: "pointer",
  },
  ghostBtn: {
    padding: "6px 14px", background: "transparent", color: "var(--color-text-secondary)",
    border: "1px solid var(--color-border)", borderRadius: 4, fontSize: 13, cursor: "pointer",
  },
};
