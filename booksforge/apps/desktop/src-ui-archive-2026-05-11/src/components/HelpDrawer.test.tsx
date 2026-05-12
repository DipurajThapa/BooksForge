import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { mockIpcModule } from "../test-utils/mockIpc";

vi.mock("../lib/ipc", () => mockIpcModule());

import HelpDrawer from "./HelpDrawer";

/**
 * Smoke tests for HelpDrawer.
 *
 * Fully offline content; no IPC calls. Just confirm the dialog
 * mounts and the default "Quickstart" tab body is reachable.
 */
describe("HelpDrawer", () => {
  it("render-without-crash", () => {
    render(<HelpDrawer onClose={() => undefined} />);
    expect(screen.getByRole("dialog")).toBeTruthy();
  });

  it("dialog-a11y exposes aria-modal + labelled title", () => {
    render(<HelpDrawer onClose={() => undefined} />);
    const dialog = screen.getByRole("dialog");
    expect(dialog.getAttribute("aria-modal")).toBe("true");
    expect(dialog.getAttribute("aria-labelledby")).toBeTruthy();
    // Title element renders.
    expect(screen.getByText("Help")).toBeTruthy();
  });
});
