/**
 * SnapshotsPanel smoke tests — manual + auto snapshot timeline panel
 * with restore/compare flows.
 */
// eslint-disable-next-line @typescript-eslint/no-unused-vars
import * as _React from "react";
import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { mockIpcModule } from "../test-utils/mockIpc";

vi.mock("../lib/ipc", () => mockIpcModule());

import SnapshotsPanel from "./SnapshotsPanel";

describe("SnapshotsPanel", () => {
  it("renders without crashing", () => {
    render(<SnapshotsPanel onClose={vi.fn()} />);
    expect(screen.getByRole("dialog")).toBeTruthy();
    expect(screen.getByText("Snapshots")).toBeTruthy();
  });

  it("dialog has correct a11y attributes and Close button fires onClose", () => {
    const onClose = vi.fn();
    render(<SnapshotsPanel onClose={onClose} />);
    const dialog = screen.getByRole("dialog");
    expect(dialog.getAttribute("aria-modal")).toBe("true");
    expect(dialog.getAttribute("aria-labelledby")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: /close panel/i }));
    expect(onClose).toHaveBeenCalledTimes(1);
  });
});
