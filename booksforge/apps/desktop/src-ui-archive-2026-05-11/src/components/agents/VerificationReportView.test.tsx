/**
 * VerificationReportView smoke tests — shared component that renders
 * the council verdict + Tier-1/Tier-2 checks + peer reviews from any
 * `AgentRunResultDto.verification` payload.
 */
// eslint-disable-next-line @typescript-eslint/no-unused-vars
import * as _React from "react";
import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";

import VerificationReportView from "./VerificationReportView";

// Minimal report shape — VerificationReportDto requires tier_1
// (ProposalValidationDto), peer_reviews (array), and optionally tier_2
// + final_verdict. Cast through `unknown` so the test isn't tied to
// every nullable field's exact shape.
const REPORT = {
  final_verdict: "pass",
  tier_1: {
    verdict: "pass",
    summary: "All deterministic checks passed.",
    checks: [
      { axis: "schema", outcome: "pass", evidence: "Valid JSON proposal." },
    ],
  },
  tier_2: null,
  peer_reviews: [],
} as unknown as Parameters<typeof VerificationReportView>[0]["report"];

describe("VerificationReportView", () => {
  it("renders the council verdict and tier-1 section", () => {
    render(<VerificationReportView report={REPORT} />);
    expect(screen.getByText(/Council verdict:/i)).toBeTruthy();
    expect(screen.getByText(/Tier 1 — deterministic/i)).toBeTruthy();
  });

  it("renders an evidence row for each Tier-1 check", () => {
    render(<VerificationReportView report={REPORT} />);
    expect(screen.getByText(/Valid JSON proposal\./i)).toBeTruthy();
  });
});
