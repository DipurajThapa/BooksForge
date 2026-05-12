/**
 * SettingsPanel smoke tests — the privacy-facing settings dialog
 * (telemetry toggles, diagnostic bundle, originality consent, export
 * deps, theme, workflow gates, app version).
 */
// eslint-disable-next-line @typescript-eslint/no-unused-vars
import * as _React from "react";
import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { mockIpcModule } from "../test-utils/mockIpc";

vi.mock("../lib/ipc", () => mockIpcModule());
// Tauri save-dialog plugin is invoked when the user clicks "Save
// diagnostic bundle"; stub it to a no-op so import doesn't blow up.
vi.mock("@tauri-apps/plugin-dialog", () => ({
  save: vi.fn().mockResolvedValue(null),
}));

import SettingsPanel from "./SettingsPanel";

describe("SettingsPanel", () => {
  it("renders without crashing", () => {
    render(<SettingsPanel onClose={vi.fn()} />);
    expect(screen.getByRole("dialog")).toBeTruthy();
    expect(screen.getByText("Settings")).toBeTruthy();
  });

  it("dialog has correct a11y attributes and Close button fires onClose", () => {
    const onClose = vi.fn();
    render(<SettingsPanel onClose={onClose} />);
    const dialog = screen.getByRole("dialog");
    expect(dialog.getAttribute("aria-modal")).toBe("true");
    expect(dialog.getAttribute("aria-labelledby")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: /close panel/i }));
    expect(onClose).toHaveBeenCalledTimes(1);
  });
});
