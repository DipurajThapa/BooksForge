import { describe, it, expect, vi } from "vitest";
import { render } from "@testing-library/react";
import { mockIpcModule } from "../test-utils/mockIpc";

vi.mock("../lib/ipc", () => mockIpcModule());

import Binder from "./Binder";

/**
 * Smoke tests for Binder.
 *
 * Binder is the left-pane scene roll. With an empty `nodes` list it
 * should still render its container without throwing. The full
 * drag-drop / rank logic is exercised by the orchestrator and not
 * worth re-testing here.
 */
describe("Binder", () => {
  it("render-without-crash with empty node list", () => {
    const { container } = render(
      <Binder
        nodes={[]}
        selectedId={null}
        onSelect={() => undefined}
        onNodesChanged={() => undefined}
      />,
    );
    expect(container.firstChild).toBeTruthy();
  });

  it("renders a stable DOM shell with no nodes", () => {
    const { container } = render(
      <Binder
        nodes={[]}
        selectedId={null}
        onSelect={() => undefined}
        onNodesChanged={() => undefined}
      />,
    );
    // Binder mounts without throwing even when there are no scenes;
    // the container should hold at least one element.
    expect(container.querySelectorAll("*").length).toBeGreaterThan(0);
  });
});
