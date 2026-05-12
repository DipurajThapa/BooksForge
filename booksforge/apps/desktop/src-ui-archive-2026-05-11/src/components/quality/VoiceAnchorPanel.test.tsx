/**
 * VoiceAnchorPanel smoke tests — sets the project's voice anchor from
 * comp-sample prose; persists to book-scope memory under voice:anchor.
 */
// eslint-disable-next-line @typescript-eslint/no-unused-vars
import * as _React from "react";
import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { mockIpcModule } from "../../test-utils/mockIpc";

vi.mock("../../lib/ipc", () => mockIpcModule());

import VoiceAnchorPanel from "./VoiceAnchorPanel";

describe("VoiceAnchorPanel", () => {
  it("renders without crashing", () => {
    render(<VoiceAnchorPanel onClose={vi.fn()} />);
    expect(screen.getByRole("dialog")).toBeTruthy();
    // "Voice Anchor" appears as both the header and the section legend;
    // assert ≥1 matching element rather than uniqueness.
    expect(screen.getAllByText(/Voice Anchor/i).length).toBeGreaterThan(0);
  });

  it("dialog has correct a11y attributes and Close button fires onClose", () => {
    const onClose = vi.fn();
    render(<VoiceAnchorPanel onClose={onClose} />);
    const dialog = screen.getByRole("dialog");
    expect(dialog.getAttribute("aria-modal")).toBe("true");
    expect(dialog.getAttribute("aria-labelledby")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: /close.*panel/i }));
    expect(onClose).toHaveBeenCalledTimes(1);
  });
});
