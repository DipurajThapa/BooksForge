import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { mockIpcModule } from "../test-utils/mockIpc";

vi.mock("../lib/ipc", () => mockIpcModule());

import QuickActionBar from "./QuickActionBar";

/**
 * Smoke tests for QuickActionBar.
 *
 * The Cmd/Ctrl+K AI quick-action overlay. When `open={false}` it
 * renders nothing; when `open={true}` it renders a dialog with the
 * preset row.
 */
describe("QuickActionBar", () => {
  it("render-without-crash when open", () => {
    render(
      <QuickActionBar
        open={true}
        nodeId="01NODE00000000000000000000"
        getScopeText={() => ""}
        onClose={() => undefined}
      />,
    );
    expect(screen.getByRole("dialog")).toBeTruthy();
  });

  it("renders nothing when closed", () => {
    const { container } = render(
      <QuickActionBar
        open={false}
        nodeId="01NODE00000000000000000000"
        getScopeText={() => ""}
        onClose={() => undefined}
      />,
    );
    // `if (!open) return null;` — the component should emit no DOM.
    expect(container.firstChild).toBeNull();
  });
});
