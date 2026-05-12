import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { mockIpcModule } from "../test-utils/mockIpc";

vi.mock("../lib/ipc", () => mockIpcModule());

import BookKindOverlay from "./BookKindOverlay";

/**
 * Smoke tests for BookKindOverlay.
 *
 * The overlay is shown both at onboarding (non-dismissible) and from
 * SettingsPanel (dismissible). Both render the same kind grid; these
 * tests just confirm the dialog mounts in either mode.
 */
describe("BookKindOverlay", () => {
  it("render-without-crash in onboarding mode", () => {
    render(
      <BookKindOverlay
        mode="onboarding"
        currentKind={null}
        onSaved={() => undefined}
        onClose={() => undefined}
      />,
    );
    expect(screen.getByRole("dialog")).toBeTruthy();
  });

  it("dialog-a11y in settings mode", () => {
    render(
      <BookKindOverlay
        mode="settings"
        currentKind="literary-fiction"
        onSaved={() => undefined}
        onClose={() => undefined}
      />,
    );
    const dialog = screen.getByRole("dialog");
    expect(dialog.getAttribute("aria-modal")).toBe("true");
    expect(dialog.getAttribute("aria-labelledby")).toBeTruthy();
  });
});
