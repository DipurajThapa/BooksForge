import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { mockIpcModule } from "../test-utils/mockIpc";

vi.mock("../lib/ipc", () => mockIpcModule());

import KnowledgePanel from "./KnowledgePanel";

/**
 * Smoke tests for KnowledgePanel.
 *
 * Read-only memory + vocab inspector. The shared IPC mock returns
 * empty arrays for both, so the panel should mount with the empty-state
 * hint visible.
 */
describe("KnowledgePanel", () => {
  it("render-without-crash", () => {
    render(<KnowledgePanel onClose={() => undefined} />);
    expect(screen.getByRole("dialog")).toBeTruthy();
  });

  it("dialog-a11y", () => {
    render(<KnowledgePanel onClose={() => undefined} />);
    const dialog = screen.getByRole("dialog");
    expect(dialog.getAttribute("aria-modal")).toBe("true");
    expect(dialog.getAttribute("aria-labelledby")).toBeTruthy();
    expect(screen.getByText("Knowledge")).toBeTruthy();
  });
});
