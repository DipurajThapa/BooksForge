import React from "react";
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render } from "@testing-library/react";
import { axe } from "vitest-axe";
import { ErrorBoundary } from "./components/ErrorBoundary";
import { ToastProvider, useToast } from "./components/ToastProvider";
import {
  ProposalReview,
  type Decision,
  type Proposal,
} from "./components/ProposalReview";
import { ShortcutHelp } from "./components/ShortcutHelp";

/**
 * Top-level accessibility audit.
 *
 * Closes the automated half of EXTERNAL_AUDIT_BACKLOG.md #34 (WCAG
 * 2.2 AA).  axe-core catches a meaningful subset of WCAG issues —
 * Deque measures ~57% of real-world violations are auto-detectable
 * — and is a good first guard against regressions.  The manual
 * AT-testing half (VoiceOver + NVDA on a real device) is the
 * pre-release human task tracked in `MILESTONES.md M6 §I1`.
 *
 * One test per shipped component that's part of the always-mounted
 * app shell or the user-action-modal set:
 *   - ErrorBoundary fallback dialog
 *   - ToastProvider toast region (info, error, with action)
 *   - ProposalReview region (loaded + empty state)
 *   - ShortcutHelp overlay
 *
 * Every test builds a small DOM, runs axe, and asserts no
 * violations.  When axe DOES flag something, the report is verbose
 * enough that the contributor can see the rule, the offending
 * selector, and the fix in the test failure output.
 *
 * **Adding a new component:** import it, render a representative
 * snapshot, and `expect(await axe(container)).toHaveNoViolations()`.
 * Don't game the test by hiding violations — fix the component or
 * file an issue.
 */

function Boom(): JSX.Element {
  throw new Error("kaboom-a11y");
}

describe("a11y — ErrorBoundary fallback", () => {
  let consoleErrorSpy: ReturnType<typeof vi.spyOn>;
  beforeEach(() => {
    consoleErrorSpy = vi.spyOn(console, "error").mockImplementation(() => undefined);
  });
  afterEach(() => {
    consoleErrorSpy.mockRestore();
  });

  it("default fallback dialog has no axe violations", async () => {
    const { container } = render(
      <ErrorBoundary>
        <Boom />
      </ErrorBoundary>,
    );
    const result = await axe(container);
    expect(result).toHaveNoViolations();
  });

  it("happy path (no error) has no axe violations", async () => {
    const { container } = render(
      <ErrorBoundary>
        <main>
          <h1>Hello</h1>
          <p>regular content</p>
        </main>
      </ErrorBoundary>,
    );
    const result = await axe(container);
    expect(result).toHaveNoViolations();
  });
});

describe("a11y — ToastProvider", () => {
  function HarnessWithToast(props: {
    severity: "info" | "success" | "warning" | "error";
    body: string;
    withAction?: boolean;
  }): JSX.Element {
    return (
      <ToastProvider>
        <ToastTrigger {...props} />
      </ToastProvider>
    );
  }

  function ToastTrigger({
    severity,
    body,
    withAction,
  }: {
    severity: "info" | "success" | "warning" | "error";
    body: string;
    withAction?: boolean;
  }): JSX.Element {
    const toast = useToast();
    React.useEffect(() => {
      toast.push({
        body,
        severity,
        durationMs: 0, // persist for the test
        action: withAction ? { label: "Undo", onClick: () => undefined } : undefined,
      });
    }, [toast, severity, body, withAction]);
    return <div data-testid="anchor">anchor</div>;
  }

  it("error toast region has no axe violations", async () => {
    const { container } = render(<HarnessWithToast severity="error" body="boom" />);
    const result = await axe(container);
    expect(result).toHaveNoViolations();
  });

  it("info toast with action button has no axe violations", async () => {
    const { container } = render(
      <HarnessWithToast severity="info" body="heads up" withAction />,
    );
    const result = await axe(container);
    expect(result).toHaveNoViolations();
  });
});

describe("a11y — ProposalReview", () => {
  function buildProposal(empty: boolean): Proposal {
    return empty
      ? { id: "empty", agentName: "Copyedit", hunks: [] }
      : {
          id: "p1",
          agentName: "Copyedit",
          summary: "3 small wording fixes.",
          hunks: [
            { id: "h1", before: "old1", after: "new1", rationale: "shorter" },
            { id: "h2", before: "old2", after: "new2" },
          ],
        };
  }
  const decisions = (p: Proposal): Decision[] =>
    p.hunks.map((h) => ({ hunkId: h.id, status: "pending" }));

  it("loaded proposal review has no axe violations", async () => {
    const p = buildProposal(false);
    const { container } = render(
      <ProposalReview
        proposal={p}
        decisions={decisions(p)}
        onChange={() => undefined}
        onApply={() => undefined}
        onCancel={() => undefined}
      />,
    );
    const result = await axe(container);
    expect(result).toHaveNoViolations();
  });

  it("empty proposal review has no axe violations", async () => {
    const p = buildProposal(true);
    const { container } = render(
      <ProposalReview
        proposal={p}
        decisions={[]}
        onChange={() => undefined}
        onApply={() => undefined}
        onCancel={() => undefined}
      />,
    );
    const result = await axe(container);
    expect(result).toHaveNoViolations();
  });
});

describe("a11y — ShortcutHelp overlay", () => {
  it("modal dialog has no axe violations", async () => {
    const { container } = render(<ShortcutHelp onClose={() => undefined} />);
    const result = await axe(container);
    expect(result).toHaveNoViolations();
  });
});
