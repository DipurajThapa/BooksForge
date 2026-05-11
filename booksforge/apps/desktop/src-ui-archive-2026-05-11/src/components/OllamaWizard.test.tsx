import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { mockIpcModule } from "../test-utils/mockIpc";

vi.mock("../lib/ipc", () => mockIpcModule());

// The wizard subscribes to `ollama:pull-progress` via the @tauri-apps
// event API during its pull step. Mock it to a no-op so no IPC bridge
// is required at test time.
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => undefined),
}));

import OllamaWizard from "./OllamaWizard";

/**
 * Smoke tests for OllamaWizard.
 *
 * Four-step modal: detect → install → pick → smoke. The shared IPC
 * mock returns a probe response so the wizard advances past step 1 to
 * the install step (api_reachable defaults to falsy in the mock).
 */
describe("OllamaWizard", () => {
  it("render-without-crash", () => {
    render(
      <OllamaWizard
        onClose={() => undefined}
        onComplete={() => undefined}
      />,
    );
    expect(screen.getByRole("dialog")).toBeTruthy();
  });

  it("dialog-a11y", () => {
    render(
      <OllamaWizard
        onClose={() => undefined}
        onComplete={() => undefined}
      />,
    );
    const dialog = screen.getByRole("dialog");
    expect(dialog.getAttribute("aria-modal")).toBe("true");
    expect(dialog.getAttribute("aria-labelledby")).toBeTruthy();
  });
});
