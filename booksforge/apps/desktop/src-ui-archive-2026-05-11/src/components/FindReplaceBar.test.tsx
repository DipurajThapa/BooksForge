import { describe, it, expect, vi } from "vitest";
import { render } from "@testing-library/react";
import { mockIpcModule } from "../test-utils/mockIpc";

vi.mock("../lib/ipc", () => mockIpcModule());

import FindReplaceBar from "./FindReplaceBar";

/**
 * Smoke tests for FindReplaceBar.
 *
 * The bar reads from a TipTap editor instance to compute matches; with
 * `editor={null}` it still renders its overlay shell when `open`. When
 * `open={false}` it should render nothing.
 */
describe("FindReplaceBar", () => {
  it("render-without-crash when open with no editor", () => {
    const { container } = render(
      <FindReplaceBar
        open={true}
        editor={null}
        onClose={() => undefined}
      />,
    );
    expect(container.firstChild).toBeTruthy();
  });

  it("renders nothing when closed", () => {
    const { container } = render(
      <FindReplaceBar
        open={false}
        editor={null}
        onClose={() => undefined}
      />,
    );
    // Either nothing in the container, or a comment placeholder — the
    // important assertion is that no visible toolbar markup is emitted.
    expect(container.querySelector("input")).toBeNull();
  });
});
