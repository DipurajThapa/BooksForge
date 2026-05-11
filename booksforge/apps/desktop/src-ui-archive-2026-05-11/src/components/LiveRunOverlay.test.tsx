import { describe, it, expect, vi } from "vitest";
import { render } from "@testing-library/react";
import { mockIpcModule } from "../test-utils/mockIpc";

vi.mock("../lib/ipc", () => mockIpcModule());

import LiveRunOverlay from "./LiveRunOverlay";

/**
 * Smoke tests for LiveRunOverlay.
 *
 * The overlay subscribes to agent-run lifecycle events and renders
 * nothing when there are no in-flight runs. Since the shared IPC mock
 * returns no-op unlisten handles for those subscriptions, the overlay
 * should mount silently.
 */
describe("LiveRunOverlay", () => {
  it("render-without-crash with no active runs", () => {
    const { container } = render(<LiveRunOverlay />);
    // No runs in flight → either nothing rendered or an empty container.
    // The important thing is no exception is thrown during mount.
    expect(container).toBeTruthy();
  });

  it("does not render any visible run cards on mount", () => {
    const { container } = render(<LiveRunOverlay />);
    // No agent-run cards should be present until events fire.
    expect(container.querySelector("button[aria-label*='Cancel']")).toBeNull();
  });
});
