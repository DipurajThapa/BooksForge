/**
 * Shared component that renders an `AgentRunResultDto.verification`
 * report — Tier-1 cross-cutting checks, optional Tier-2 LLM validator,
 * and any peer-review results from the council.
 *
 * Used by every agent panel so verdicts look identical no matter which
 * agent produced them.
 */
import React from "react";
import type {
  PeerReviewResultDto,
  ProposalValidationDto,
  VerificationReportDto,
} from "@booksforge/shared-types";

interface Props {
  report: VerificationReportDto;
}

const verdictColor: Record<string, string> = {
  pass:  "var(--color-success, #2e7d32)",
  warn:  "var(--color-warn,    #f9a825)",
  block: "var(--color-error,   #c62828)",
};

export default function VerificationReportView({ report }: Props) {
  const final = (report.final_verdict ?? "pass").toLowerCase();
  return (
    <div style={s.root}>
      <header style={{ ...s.header, color: verdictColor[final] ?? "inherit" }}>
        <strong>Council verdict:</strong> {final.toUpperCase()}
      </header>

      <Section title={`Tier 1 — deterministic (${report.tier_1.checks.length} checks)`}>
        <ValidationBlock pv={report.tier_1} />
      </Section>

      {report.tier_2 && (
        <Section title="Tier 2 — LLM validator">
          <ValidationBlock pv={report.tier_2} />
        </Section>
      )}

      {report.peer_reviews.length > 0 && (
        <Section title={`Peer reviews (${report.peer_reviews.length})`}>
          {report.peer_reviews.map((pr, i) => (
            <PeerReviewRow key={i} review={pr} />
          ))}
        </Section>
      )}
    </div>
  );
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <section style={s.section}>
      <div style={s.sectionTitle}>{title}</div>
      {children}
    </section>
  );
}

function ValidationBlock({ pv }: { pv: ProposalValidationDto }) {
  const verdict = (pv.verdict ?? "pass").toLowerCase();
  return (
    <>
      <div style={{ ...s.summary, color: verdictColor[verdict] ?? "inherit" }}>
        {verdict.toUpperCase()} — {pv.summary}
      </div>
      <ul style={s.checkList}>
        {pv.checks.map((c, i) => (
          <li key={i} style={s.checkRow}>
            <span style={s.axisTag}>{c.axis}</span>
            <span style={{
              ...s.outcomeTag,
              color: verdictColor[(c.outcome ?? "pass").toLowerCase()] ?? "inherit",
            }}>
              {(c.outcome ?? "").toUpperCase()}
            </span>
            <span style={s.evidence}>{c.evidence}</span>
            {c.remediation && (
              <span style={s.remediation}>↳ {c.remediation}</span>
            )}
          </li>
        ))}
      </ul>
    </>
  );
}

function PeerReviewRow({ review }: { review: PeerReviewResultDto }) {
  const verdict = (review.verdict ?? "pass").toLowerCase();
  return (
    <div style={s.peerRow}>
      <div style={s.peerHeader}>
        <strong>{review.reviewer_agent_id}</strong>
        <span style={s.focusTag}>{review.focus}</span>
        <span style={{
          ...s.outcomeTag,
          color: verdictColor[verdict] ?? "inherit",
          marginLeft: "auto",
        }}>
          {verdict.toUpperCase()}
        </span>
      </div>
      {review.recommendation && (
        <div style={s.recommendation}>{review.recommendation}</div>
      )}
      {review.concerns.length > 0 && (
        <ul style={s.concerns}>
          {review.concerns.map((c, i) => (
            <li key={i} style={s.concernRow}>
              <span style={s.severityTag}>{c.severity}</span>
              <span style={s.quoted}>"{c.quote}"</span>
              <span style={s.reason}>{c.reason}</span>
              {c.evidence && <span style={s.evidence}>↳ {c.evidence}</span>}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

const s: Record<string, React.CSSProperties> = {
  root:         { display: "flex", flexDirection: "column", gap: 12, fontSize: 13 },
  header:       { fontSize: 14, fontWeight: 600 },
  section:      { borderTop: "1px solid var(--color-border)", paddingTop: 8 },
  sectionTitle: { fontSize: 12, fontWeight: 600, marginBottom: 6, opacity: 0.85 },
  summary:      { marginBottom: 6 },
  checkList:    { listStyle: "none", padding: 0, margin: 0, display: "flex", flexDirection: "column", gap: 4 },
  checkRow:     { display: "flex", flexWrap: "wrap", alignItems: "baseline", gap: 8, fontSize: 12 },
  axisTag:      { fontWeight: 600, minWidth: 110 },
  outcomeTag:   { fontWeight: 600, minWidth: 50, textTransform: "uppercase" },
  evidence:     { flex: 1, opacity: 0.85 },
  remediation:  { width: "100%", paddingLeft: 168, fontStyle: "italic", opacity: 0.7 },
  peerRow:      { padding: "8px 0", borderBottom: "1px dashed var(--color-border)" },
  peerHeader:   { display: "flex", gap: 8, alignItems: "center" },
  focusTag:     { fontSize: 11, padding: "1px 6px", border: "1px solid var(--color-border)", borderRadius: 3 },
  recommendation: { fontStyle: "italic", marginTop: 4, opacity: 0.85 },
  concerns:     { listStyle: "none", padding: 0, margin: "6px 0 0 0", display: "flex", flexDirection: "column", gap: 4 },
  concernRow:   { display: "flex", flexWrap: "wrap", alignItems: "baseline", gap: 8, fontSize: 12 },
  severityTag:  { fontSize: 10, padding: "1px 4px", border: "1px solid var(--color-border)", borderRadius: 3 },
  quoted:       { fontStyle: "italic" },
  reason:       { opacity: 0.85 },
};
