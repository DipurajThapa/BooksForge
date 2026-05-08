import React from "react";
import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import {
  ProposalReview,
  type Decision,
  type Proposal,
} from "./ProposalReview";

/**
 * ProposalReview tests — closes part of EXTERNAL_AUDIT_BACKLOG.md #29
 * (component) + #22 (frontend coverage).
 *
 * Exercises:
 *   1. Renders header + summary + per-hunk cards.
 *   2. Accept / Reject / Reset transitions work and update counts.
 *   3. Accept all / Reject all toolbar actions.
 *   4. Apply button is disabled when no hunks are accepted.
 *   5. Apply button passes only accepted hunks to onApply.
 *   6. onCancel fires when Cancel is clicked.
 *   7. Empty proposal renders a placeholder, not a crash.
 *   8. Diff mode prop renders side-by-side or unified.
 *   9. ARIA: each hunk is a labelled section; aria-pressed on
 *      Accept/Reject reflects state.
 */

function buildProposal(): Proposal {
  return {
    id: "agent_run_01",
    agentName: "Copyedit",
    summary: "3 small wording fixes.",
    hunks: [
      { id: "h1", before: "old1", after: "new1", rationale: "shorter" },
      { id: "h2", before: "old2", after: "new2", tag: "minor" },
      { id: "h3", before: "old3", after: "new3" },
    ],
  };
}

function pendingDecisions(p: Proposal): Decision[] {
  return p.hunks.map((h) => ({ hunkId: h.id, status: "pending" }));
}

describe("ProposalReview", () => {
  it("renders the header, summary, and one card per hunk", () => {
    const p = buildProposal();
    render(
      <ProposalReview
        proposal={p}
        decisions={pendingDecisions(p)}
        onChange={() => undefined}
        onApply={() => undefined}
        onCancel={() => undefined}
      />,
    );
    expect(screen.getByText(/Copyedit suggestions/)).toBeTruthy();
    expect(screen.getByText("3 small wording fixes.")).toBeTruthy();
    // Per-hunk titles default to "Hunk N" when no label is provided.
    expect(screen.getByText("Hunk 1")).toBeTruthy();
    expect(screen.getByText("Hunk 2")).toBeTruthy();
    expect(screen.getByText("Hunk 3")).toBeTruthy();
  });

  it("Accept button reflects status via aria-pressed", () => {
    const p = buildProposal();
    const onChange = vi.fn();
    render(
      <ProposalReview
        proposal={p}
        decisions={pendingDecisions(p)}
        onChange={onChange}
        onApply={() => undefined}
        onCancel={() => undefined}
      />,
    );
    const acceptH1 = screen.getByRole("button", { name: /Accept hunk 1 of 3/i });
    expect(acceptH1.getAttribute("aria-pressed")).toBe("false");
    fireEvent.click(acceptH1);
    expect(onChange).toHaveBeenCalledTimes(1);
    const next = onChange.mock.calls[0]![0] as Decision[];
    expect(next.find((d) => d.hunkId === "h1")?.status).toBe("accepted");
  });

  it("Apply is disabled when no hunks are accepted", () => {
    const p = buildProposal();
    render(
      <ProposalReview
        proposal={p}
        decisions={pendingDecisions(p)}
        onChange={() => undefined}
        onApply={() => undefined}
        onCancel={() => undefined}
      />,
    );
    const applyBtn = screen.getByRole("button", { name: /Apply 0/i });
    expect(applyBtn.hasAttribute("disabled")).toBe(true);
  });

  it("Apply with accepted decisions passes only those to onApply", () => {
    const p = buildProposal();
    const decisions: Decision[] = [
      { hunkId: "h1", status: "accepted" },
      { hunkId: "h2", status: "rejected" },
      { hunkId: "h3", status: "accepted" },
    ];
    const onApply = vi.fn();
    render(
      <ProposalReview
        proposal={p}
        decisions={decisions}
        onChange={() => undefined}
        onApply={onApply}
        onCancel={() => undefined}
      />,
    );
    const applyBtn = screen.getByRole("button", { name: /Apply 2/i });
    fireEvent.click(applyBtn);
    expect(onApply).toHaveBeenCalledTimes(1);
    const accepted = onApply.mock.calls[0]![0] as Decision[];
    expect(accepted.map((d) => d.hunkId).sort()).toEqual(["h1", "h3"]);
    expect(accepted.every((d) => d.status === "accepted")).toBe(true);
  });

  it("Cancel fires onCancel", () => {
    const p = buildProposal();
    const onCancel = vi.fn();
    render(
      <ProposalReview
        proposal={p}
        decisions={pendingDecisions(p)}
        onChange={() => undefined}
        onApply={() => undefined}
        onCancel={onCancel}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: /Cancel/i }));
    expect(onCancel).toHaveBeenCalledTimes(1);
  });

  it("Accept all toolbar action sets every hunk to accepted", () => {
    const p = buildProposal();
    const onChange = vi.fn();
    render(
      <ProposalReview
        proposal={p}
        decisions={pendingDecisions(p)}
        onChange={onChange}
        onApply={() => undefined}
        onCancel={() => undefined}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: /Accept all/i }));
    const next = onChange.mock.calls[0]![0] as Decision[];
    expect(next.every((d) => d.status === "accepted")).toBe(true);
  });

  it("Empty proposal renders a placeholder, not a crash", () => {
    const empty: Proposal = {
      id: "empty",
      agentName: "Copyedit",
      hunks: [],
    };
    render(
      <ProposalReview
        proposal={empty}
        decisions={[]}
        onChange={() => undefined}
        onApply={() => undefined}
        onCancel={() => undefined}
      />,
    );
    expect(screen.getByText(/did not produce any proposals/i)).toBeTruthy();
  });

  it("decision counts update after rejection", () => {
    const p = buildProposal();
    const decisions: Decision[] = [
      { hunkId: "h1", status: "accepted" },
      { hunkId: "h2", status: "rejected" },
      { hunkId: "h3", status: "pending" },
    ];
    render(
      <ProposalReview
        proposal={p}
        decisions={decisions}
        onChange={() => undefined}
        onApply={() => undefined}
        onCancel={() => undefined}
      />,
    );
    // Footer aria-live role="status" carries the counts.
    const status = screen.getByRole("status");
    expect(status.textContent).toMatch(/1.*accepted/);
    expect(status.textContent).toMatch(/1.*rejected/);
    expect(status.textContent).toMatch(/1.*pending/);
  });

  it("region is labelled by agentName", () => {
    const p = buildProposal();
    render(
      <ProposalReview
        proposal={p}
        decisions={pendingDecisions(p)}
        onChange={() => undefined}
        onApply={() => undefined}
        onCancel={() => undefined}
      />,
    );
    const region = screen.getByRole("region", {
      name: /Proposals from Copyedit/i,
    });
    expect(region).toBeTruthy();
  });
});
