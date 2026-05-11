import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { mockIpcModule } from "../test-utils/mockIpc";

vi.mock("../lib/ipc", () => mockIpcModule());

import ExportPanel from "./ExportPanel";

/**
 * Smoke tests for ExportPanel.
 *
 * The export panel is a dialog with a profile picker, run button, and
 * persisted history list. The shared IPC mock returns empty
 * dependency / history responses so it should mount cleanly.
 */
describe("ExportPanel", () => {
  it("render-without-crash", () => {
    render(<ExportPanel onClose={() => undefined} />);
    expect(screen.getByRole("dialog")).toBeTruthy();
  });

  it("dialog-a11y", () => {
    render(<ExportPanel onClose={() => undefined} />);
    const dialog = screen.getByRole("dialog");
    expect(dialog.getAttribute("aria-modal")).toBe("true");
    expect(dialog.getAttribute("aria-labelledby")).toBeTruthy();
  });
});
