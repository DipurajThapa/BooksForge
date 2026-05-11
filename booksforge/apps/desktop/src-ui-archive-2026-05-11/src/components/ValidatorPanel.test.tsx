/**
 * ValidatorPanel smoke tests — runs the 16 deterministic validators
 * and renders results grouped by severity.
 */
// eslint-disable-next-line @typescript-eslint/no-unused-vars
import * as _React from "react";
import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { mockIpcModule } from "../test-utils/mockIpc";

vi.mock("../lib/ipc", () => mockIpcModule());

import ValidatorPanel from "./ValidatorPanel";

describe("ValidatorPanel", () => {
  it("renders without crashing", () => {
    render(<ValidatorPanel onClose={vi.fn()} />);
    expect(screen.getByRole("dialog")).toBeTruthy();
    expect(screen.getByText("Manuscript checks")).toBeTruthy();
  });

  it("dialog has correct a11y attributes and Close button fires onClose", async () => {
    const onClose = vi.fn();
    render(
      <ValidatorPanel onClose={onClose} onSelectNode={vi.fn()} />,
    );
    const dialog = screen.getByRole("dialog");
    expect(dialog.getAttribute("aria-modal")).toBe("true");
    expect(dialog.getAttribute("aria-labelledby")).toBeTruthy();
    // The panel auto-runs the validator on mount and disables the close
    // button until the run resolves; wait for the (mocked) run to settle
    // before asserting the click has effect.
    const closeBtn = screen.getByRole("button", { name: /close validator panel/i });
    await waitFor(() => expect(closeBtn.hasAttribute("disabled")).toBe(false));
    fireEvent.click(closeBtn);
    expect(onClose).toHaveBeenCalledTimes(1);
  });
});
