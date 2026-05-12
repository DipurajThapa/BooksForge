/**
 * RecoveryDialog smoke tests.
 *
 * The dialog is shown on app start when an unsaved scene from a prior
 * session is detected. It offers two destructive-choice buttons:
 * Restore (recover the unsaved version) or Discard (drop it). The
 * actual prop names are `onRestore` / `onDiscard` (not `onAccept` /
 * `onDismiss`) — the panel uses `useDialogA11y` and overrides role to
 * `alertdialog`.
 */
// eslint-disable-next-line @typescript-eslint/no-unused-vars
import * as _React from "react";
import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";

import RecoveryDialog from "./RecoveryDialog";

const STATUS = {
  has_pending: true,
  pending_at: "2026-05-08T10:30:00Z",
} as unknown as Parameters<typeof RecoveryDialog>[0]["status"];

describe("RecoveryDialog", () => {
  it("renders without crashing as an alertdialog", () => {
    render(
      <RecoveryDialog
        status={STATUS}
        onRestore={vi.fn()}
        onDiscard={vi.fn()}
      />,
    );
    expect(screen.getByRole("alertdialog")).toBeTruthy();
    expect(screen.getByText(/Unsaved changes found/i)).toBeTruthy();
  });

  it("Restore and Discard buttons fire their callbacks", () => {
    const onRestore = vi.fn();
    const onDiscard = vi.fn();
    render(
      <RecoveryDialog
        status={STATUS}
        onRestore={onRestore}
        onDiscard={onDiscard}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: /restore unsaved/i }));
    expect(onRestore).toHaveBeenCalledTimes(1);
    fireEvent.click(screen.getByRole("button", { name: /^discard$/i }));
    expect(onDiscard).toHaveBeenCalledTimes(1);
  });
});
