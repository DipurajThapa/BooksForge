/**
 * useDialogA11y unit tests — drop-in modal a11y plumbing that returns
 * dialog props (role/aria-modal/aria-labelledby/tabIndex/ref/onKeyDown)
 * + the title id to wire into the dialog heading.
 */
import { describe, it, expect, vi } from "vitest";
import { renderHook } from "@testing-library/react";
import { useDialogA11y } from "./useDialogA11y";

describe("useDialogA11y", () => {
  it("returns dialogProps with the expected a11y attributes", () => {
    const { result } = renderHook(() => useDialogA11y(vi.fn()));
    const { dialogProps } = result.current;
    expect(dialogProps.role).toBe("dialog");
    expect(dialogProps["aria-modal"]).toBe("true");
    expect(dialogProps.tabIndex).toBe(-1);
    expect(typeof dialogProps.onKeyDown).toBe("function");
  });

  it("returns a non-empty titleId that matches dialogProps['aria-labelledby']", () => {
    const { result } = renderHook(() => useDialogA11y(vi.fn()));
    const { dialogProps, titleId } = result.current;
    expect(titleId).toBeTruthy();
    expect(titleId.length).toBeGreaterThan(0);
    expect(dialogProps["aria-labelledby"]).toBe(titleId);
  });

  it("provides a ref slot for the dialog root element", () => {
    const { result } = renderHook(() => useDialogA11y(vi.fn()));
    expect(result.current.dialogProps.ref).toBeDefined();
    // useRef returns { current: T | null } — verify the shape rather
    // than the value (jsdom never assigns since no element mounts).
    expect("current" in result.current.dialogProps.ref).toBe(true);
  });

  it("onKeyDown invokes onClose when Escape is pressed", () => {
    const onClose = vi.fn();
    const { result } = renderHook(() => useDialogA11y(onClose));
    const fakeEvent = {
      key: "Escape",
      stopPropagation: vi.fn(),
    } as unknown as React.KeyboardEvent;
    result.current.dialogProps.onKeyDown(fakeEvent);
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("onKeyDown ignores non-Escape keys", () => {
    const onClose = vi.fn();
    const { result } = renderHook(() => useDialogA11y(onClose));
    const fakeEvent = {
      key: "Enter",
      stopPropagation: vi.fn(),
    } as unknown as React.KeyboardEvent;
    result.current.dialogProps.onKeyDown(fakeEvent);
    expect(onClose).not.toHaveBeenCalled();
  });
});
