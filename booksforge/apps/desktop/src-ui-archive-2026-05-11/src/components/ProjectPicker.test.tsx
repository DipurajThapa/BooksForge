import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { mockIpcModule } from "../test-utils/mockIpc";

vi.mock("../lib/ipc", () => mockIpcModule());

// ProjectPicker uses the Tauri dialog plugin to pick a folder. Stub
// the import so the test environment does not require the IPC bridge.
vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn().mockResolvedValue(null),
}));

import ProjectPicker from "./ProjectPicker";

/**
 * Smoke tests for ProjectPicker.
 *
 * The landing screen — "New Project" / "Open" + Recents list. The
 * shared IPC mock returns an empty recents list.
 */
describe("ProjectPicker", () => {
  it("render-without-crash", () => {
    render(
      <ProjectPicker
        onProjectOpened={() => undefined}
        onNewProject={() => undefined}
      />,
    );
    expect(screen.getByText(/BooksForge/)).toBeTruthy();
  });

  it("renders the New Project + Open primary actions", () => {
    render(
      <ProjectPicker
        onProjectOpened={() => undefined}
        onNewProject={() => undefined}
      />,
    );
    expect(screen.getByRole("button", { name: /new project/i })).toBeTruthy();
    expect(screen.getByRole("button", { name: /open/i })).toBeTruthy();
  });
});
