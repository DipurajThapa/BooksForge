import React from "react";
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render } from "@testing-library/react";
import { axe } from "vitest-axe";
import { ErrorBoundary } from "./components/ErrorBoundary";
import { ToastProvider, useToast } from "./components/ToastProvider";
import StageRail, {
  MVP_STAGES,
  type StageInfo,
  type StageStatus,
} from "./components/StageRail";

/**
 * Top-level accessibility audit.
 *
 * Closes the automated half of EXTERNAL_AUDIT_BACKLOG.md #34 (WCAG
 * 2.2 AA). axe-core catches a meaningful subset of WCAG issues —
 * Deque measures ~57% of real-world violations are auto-detectable —
 * and is a good first guard against regressions. The manual
 * AT-testing half (VoiceOver + NVDA on a real device) is the
 * pre-release human task tracked in `MILESTONES.md M6 §I1`.
 *
 * Components covered, drawn from the May 2026 editor redesign:
 *   - ErrorBoundary fallback dialog          (always-mounted)
 *   - ToastProvider toast region             (always-mounted)
 *   - StageRail nav rail                     (always-visible in editor shell)
 *
 * **History note.** A prior version of this file also tested
 * `ProposalReview` and `ShortcutHelp` — both belong to the pre-2026-05
 * UI architecture (archived under `src-ui-archive-2026-05-11/`).
 * When the StageRail-based redesign landed they were removed from
 * the active build; this file now covers their conceptual
 * replacements. Wizard / picker / stage panels are deferred until
 * the F1 binder + manuscript-view PR lands and there's a stable
 * surface to assert against.
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

describe("a11y — StageRail", () => {
  // Render the rail in three representative states the writer
  // actually encounters: fresh project (everything available), mid
  // journey (one passed, one in-progress, rest available), and a
  // gated state (later stages locked behind prereqs). axe should
  // pass on all three.
  function railWithStatuses(statuses: Partial<Record<StageInfo["id"], StageStatus>>): StageInfo[] {
    return MVP_STAGES.map((s) => ({
      ...s,
      status: statuses[s.id] ?? "available",
    }));
  }

  it("fresh project (all available) has no axe violations", async () => {
    const stages = railWithStatuses({ setup: "in_progress" });
    const { container } = render(
      <StageRail stages={stages} active="setup" onSelect={() => undefined} />,
    );
    const result = await axe(container);
    expect(result).toHaveNoViolations();
  });

  it("mid-journey state has no axe violations", async () => {
    const stages = railWithStatuses({
      setup:      "passed",
      audience:   "passed",
      characters: "in_progress",
    });
    const { container } = render(
      <StageRail stages={stages} active="characters" onSelect={() => undefined} />,
    );
    const result = await axe(container);
    expect(result).toHaveNoViolations();
  });

  it("gated state (later stages locked) has no axe violations", async () => {
    const stages = railWithStatuses({
      setup:      "in_progress",
      audience:   "locked",
      characters: "locked",
      outline:    "locked",
      drafting:   "locked",
      export:     "locked",
    });
    const { container } = render(
      <StageRail stages={stages} active="setup" onSelect={() => undefined} />,
    );
    const result = await axe(container);
    expect(result).toHaveNoViolations();
  });
});
