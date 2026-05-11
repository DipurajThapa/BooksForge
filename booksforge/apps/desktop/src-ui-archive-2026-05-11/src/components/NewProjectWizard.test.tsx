import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { mockIpcModule } from "../test-utils/mockIpc";

vi.mock("../lib/ipc", () => mockIpcModule());

import NewProjectWizard from "./NewProjectWizard";

/**
 * Smoke tests for NewProjectWizard.
 *
 * Multi-step modal that opens on the BookKind picker. These tests just
 * confirm it mounts and exposes the expected dialog role.
 */
describe("NewProjectWizard", () => {
  it("render-without-crash", () => {
    render(
      <NewProjectWizard
        onCreated={() => undefined}
        onCancel={() => undefined}
      />,
    );
    expect(screen.getByRole("dialog")).toBeTruthy();
  });

  it("dialog-a11y", () => {
    render(
      <NewProjectWizard
        onCreated={() => undefined}
        onCancel={() => undefined}
      />,
    );
    const dialog = screen.getByRole("dialog");
    expect(dialog.getAttribute("aria-modal")).toBe("true");
    expect(dialog.getAttribute("aria-labelledby")).toBeTruthy();
  });
});
