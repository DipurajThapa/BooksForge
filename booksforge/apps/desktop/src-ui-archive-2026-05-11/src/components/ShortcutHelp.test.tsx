/**
 * ShortcutHelp smoke tests — keyboard-shortcut help overlay rendered
 * from `lib/keymap.ts`. Pure-presentation; no IPC dependency.
 */
// eslint-disable-next-line @typescript-eslint/no-unused-vars
import * as _React from "react";
import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";

import { ShortcutHelp } from "./ShortcutHelp";

describe("ShortcutHelp", () => {
  it("renders without crashing", () => {
    render(<ShortcutHelp onClose={vi.fn()} />);
    expect(screen.getByRole("dialog")).toBeTruthy();
    expect(screen.getByText(/Keyboard shortcuts/i)).toBeTruthy();
  });

  it("Close button + Esc fire onClose", () => {
    const onClose = vi.fn();
    render(<ShortcutHelp onClose={onClose} />);
    fireEvent.click(screen.getByRole("button", { name: /close shortcuts/i }));
    expect(onClose).toHaveBeenCalled();

    // Esc closes too (via the window keydown listener).
    fireEvent.keyDown(window, { key: "Escape" });
    expect(onClose).toHaveBeenCalledTimes(2);
  });
});
