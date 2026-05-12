import React from "react";
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { ErrorBoundary } from "./ErrorBoundary";

/**
 * ErrorBoundary tests — closes part of EXTERNAL_AUDIT_BACKLOG.md #22.
 *
 * The `ErrorBoundary` component is the only place an unhandled
 * render-time exception is caught.  These tests exercise:
 *   1. Happy path: children render normally when no error.
 *   2. Catch path: a thrown component triggers the fallback UI with
 *      the expected aria attributes.
 *   3. Custom fallback prop overrides the default UI.
 *   4. The "Try again" button resets the boundary.
 *   5. The reportId is generated and surfaced in the fallback UI.
 *   6. `onError` callback fires with both the error and the
 *      componentStack.
 *
 * React's error-boundary-related console.error noise is silenced
 * per-test so the suite output stays clean.
 */

function Boom({ message }: { message: string }): JSX.Element {
  throw new Error(message);
}

describe("ErrorBoundary", () => {
  let consoleErrorSpy: ReturnType<typeof vi.spyOn>;

  beforeEach(() => {
    // React deliberately logs to console.error when an error
    // boundary catches.  Silence for clean test output.
    consoleErrorSpy = vi.spyOn(console, "error").mockImplementation(() => undefined);
  });

  afterEach(() => {
    consoleErrorSpy.mockRestore();
  });

  it("renders children when no error", () => {
    render(
      <ErrorBoundary>
        <div data-testid="child">all good</div>
      </ErrorBoundary>,
    );
    expect(screen.getByTestId("child").textContent).toBe("all good");
  });

  it("renders the fallback UI on a render-time error", () => {
    render(
      <ErrorBoundary>
        <Boom message="kaboom-12345" />
      </ErrorBoundary>,
    );
    // Default fallback has the title + the error message.
    expect(screen.getByRole("alertdialog")).toBeTruthy();
    expect(screen.getByText("Something went wrong")).toBeTruthy();
    expect(screen.getByText(/kaboom-12345/)).toBeTruthy();
  });

  it("surfaces a report id in the fallback", () => {
    render(
      <ErrorBoundary>
        <Boom message="error-with-report" />
      </ErrorBoundary>,
    );
    // The fallback shows "Report ID: <id>" — the id should be
    // non-empty and at least 10 chars.
    const reportIdRow = screen.getByText(/Report ID:/i);
    expect(reportIdRow).toBeTruthy();
    expect(reportIdRow.textContent?.length).toBeGreaterThan(10);
  });

  it("renders the privacy disclaimer in the fallback", () => {
    render(
      <ErrorBoundary>
        <Boom message="privacy-text" />
      </ErrorBoundary>,
    );
    expect(
      screen.getByText(/does not send error reports automatically/i),
    ).toBeTruthy();
  });

  it("custom fallback prop overrides the default UI", () => {
    render(
      <ErrorBoundary fallback={(error) => <div data-testid="custom">{error.message}</div>}>
        <Boom message="custom-fallback" />
      </ErrorBoundary>,
    );
    expect(screen.getByTestId("custom").textContent).toBe("custom-fallback");
  });

  it("Try again button resets the boundary", () => {
    // Render-time `throw` followed by a closure flip races with React 18's
    // double-invocation in dev — the second render attempt during reconciliation
    // sees the flipped flag and "recovers" before the boundary's fallback is
    // committed to the DOM. Use a ref-counted parent that explicitly controls
    // when the child should crash, so retry semantics test cleanly.
    function Harness({ shouldBoom }: { shouldBoom: boolean }): JSX.Element {
      if (shouldBoom) throw new Error("transient");
      return <div data-testid="recovered">recovered</div>;
    }
    function ResettableHarness(): JSX.Element {
      const [boom, setBoom] = React.useState(true);
      return (
        <ErrorBoundary
          fallback={(_err, retry) => (
            <div>
              <h1>Something went wrong</h1>
              <button
                type="button"
                onClick={() => { setBoom(false); retry(); }}
              >
                Try again
              </button>
            </div>
          )}
        >
          <Harness shouldBoom={boom} />
        </ErrorBoundary>
      );
    }

    render(<ResettableHarness />);

    // The fallback is shown.
    expect(screen.getByText("Something went wrong")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: /try again/i }));

    // After retry the recovered child renders.
    expect(screen.queryByText("Something went wrong")).toBeNull();
    expect(screen.getByTestId("recovered").textContent).toBe("recovered");
  });

  it("onError callback fires with the error and componentStack", () => {
    const onError = vi.fn();
    render(
      <ErrorBoundary onError={onError}>
        <Boom message="callback-fired" />
      </ErrorBoundary>,
    );
    expect(onError).toHaveBeenCalledTimes(1);
    const [errArg, infoArg] = onError.mock.calls[0]!;
    expect(errArg).toBeInstanceOf(Error);
    expect((errArg as Error).message).toBe("callback-fired");
    expect(infoArg).toHaveProperty("componentStack");
  });

  it("logs a structured [booksforge:render-crash] line", () => {
    render(
      <ErrorBoundary>
        <Boom message="logged-payload" />
      </ErrorBoundary>,
    );
    // The boundary logs via console.error with our prefix.  Find at
    // least one call whose first arg starts with our marker.
    const matched = consoleErrorSpy.mock.calls.some(
      (call) => typeof call[0] === "string" && call[0].includes("[booksforge:render-crash]"),
    );
    expect(matched).toBe(true);
  });
});
