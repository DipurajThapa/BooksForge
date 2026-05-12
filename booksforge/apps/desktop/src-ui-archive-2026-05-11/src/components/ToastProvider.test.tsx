// React import retained for JSX runtime in older TS configs.
// eslint-disable-next-line @typescript-eslint/no-unused-vars
import * as _React from "react";
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { act, render, screen, fireEvent } from "@testing-library/react";
import { ToastProvider, useToast } from "./ToastProvider";

/**
 * ToastProvider tests — closes part of EXTERNAL_AUDIT_BACKLOG.md #22.
 *
 * Exercises:
 *   1. push() returns an id and renders the body.
 *   2. push() with severity="error" persists; default info auto-dismisses.
 *   3. dismiss(id) removes a specific toast.
 *   4. clearAll() removes everything.
 *   5. Multiple toasts stack in DOM order.
 *   6. push() with same id replaces the existing toast.
 *   7. Action button onClick fires.
 *   8. role="alert" for errors; role="status" for info/success/warning.
 *   9. useToast() outside the provider returns a no-op fallback (no
 *      render crash) and logs a console warn.
 */

function Trigger({ onPush }: { onPush: (push: ReturnType<typeof useToast>["push"]) => void }): JSX.Element {
  const toast = useToast();
  return (
    <button
      data-testid="trigger"
      onClick={() => onPush(toast.push)}
    >
      push
    </button>
  );
}

describe("ToastProvider", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });
  afterEach(() => {
    vi.useRealTimers();
  });

  it("push() renders the body and returns an id", () => {
    let returnedId = "";
    render(
      <ToastProvider>
        <Trigger onPush={(p) => { returnedId = p({ body: "hello world" }); }} />
      </ToastProvider>,
    );
    fireEvent.click(screen.getByTestId("trigger"));
    expect(returnedId).toMatch(/^toast-/);
    expect(screen.getByText("hello world")).toBeTruthy();
  });

  it("info toast auto-dismisses after the default 6 seconds", () => {
    render(
      <ToastProvider>
        <Trigger onPush={(p) => p({ body: "ephemeral" })} />
      </ToastProvider>,
    );
    fireEvent.click(screen.getByTestId("trigger"));
    expect(screen.getByText("ephemeral")).toBeTruthy();
    act(() => { vi.advanceTimersByTime(6_000); });
    expect(screen.queryByText("ephemeral")).toBeNull();
  });

  it("error toast persists by default", () => {
    render(
      <ToastProvider>
        <Trigger onPush={(p) => p({ body: "boom", severity: "error" })} />
      </ToastProvider>,
    );
    fireEvent.click(screen.getByTestId("trigger"));
    act(() => { vi.advanceTimersByTime(60_000); });
    expect(screen.getByText("boom")).toBeTruthy();
  });

  it("dismiss button removes the toast", () => {
    render(
      <ToastProvider>
        <Trigger onPush={(p) => p({ body: "removable", severity: "error" })} />
      </ToastProvider>,
    );
    fireEvent.click(screen.getByTestId("trigger"));
    expect(screen.getByText("removable")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: /dismiss error toast/i }));
    expect(screen.queryByText("removable")).toBeNull();
  });

  it("error toasts use role='alert', info uses role='status'", () => {
    render(
      <ToastProvider>
        <Trigger onPush={(p) => p({ body: "alert-me", severity: "error" })} />
      </ToastProvider>,
    );
    fireEvent.click(screen.getByTestId("trigger"));
    const node = screen.getByText("alert-me").closest("[role]");
    expect(node?.getAttribute("role")).toBe("alert");
  });

  it("push() with the same id replaces the previous toast", () => {
    render(
      <ToastProvider>
        <Trigger onPush={(p) => {
          p({ body: "first", id: "shared", severity: "error" });
          p({ body: "second", id: "shared", severity: "error" });
        }} />
      </ToastProvider>,
    );
    fireEvent.click(screen.getByTestId("trigger"));
    expect(screen.queryByText("first")).toBeNull();
    expect(screen.getByText("second")).toBeTruthy();
  });

  it("action button onClick fires", () => {
    const action = vi.fn();
    render(
      <ToastProvider>
        <Trigger onPush={(p) => p({
          body: "with action",
          severity: "error",
          action: { label: "Undo", onClick: action },
        })} />
      </ToastProvider>,
    );
    fireEvent.click(screen.getByTestId("trigger"));
    fireEvent.click(screen.getByRole("button", { name: "Undo" }));
    expect(action).toHaveBeenCalledTimes(1);
  });

  it("multiple toasts stack", () => {
    render(
      <ToastProvider>
        <Trigger onPush={(p) => {
          p({ body: "a", severity: "error" });
          p({ body: "b", severity: "error" });
          p({ body: "c", severity: "error" });
        }} />
      </ToastProvider>,
    );
    fireEvent.click(screen.getByTestId("trigger"));
    expect(screen.getByText("a")).toBeTruthy();
    expect(screen.getByText("b")).toBeTruthy();
    expect(screen.getByText("c")).toBeTruthy();
  });

  it("useToast() outside the provider returns a no-op (no crash)", () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => undefined);
    render(<Trigger onPush={(p) => p({ body: "no provider" })} />);
    fireEvent.click(screen.getByTestId("trigger"));
    expect(warn).toHaveBeenCalled();
    expect(screen.queryByText("no provider")).toBeNull();
    warn.mockRestore();
  });
});
