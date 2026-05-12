import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { mockIpcModule } from "../test-utils/mockIpc";

vi.mock("../lib/ipc", () => mockIpcModule());

import AgentDebugForm from "./AgentDebugForm";

/**
 * Smoke tests for AgentDebugForm.
 *
 * The component is a developer-only modal for invoking the
 * outline-architect agent with a free-form ProjectBrief JSON. These
 * tests confirm it mounts and surfaces the expected dialog role +
 * title without any IPC calls in flight.
 */
describe("AgentDebugForm", () => {
  it("render-without-crash", () => {
    render(
      <AgentDebugForm
        projectId="01TESTPROJECT00000000000000"
        onClose={() => undefined}
      />,
    );
    expect(screen.getByRole("dialog")).toBeTruthy();
  });

  it("dialog-a11y", () => {
    render(
      <AgentDebugForm
        projectId="01TESTPROJECT00000000000000"
        onClose={() => undefined}
      />,
    );
    const dialog = screen.getByRole("dialog");
    expect(dialog.getAttribute("aria-modal")).toBe("true");
    expect(dialog.getAttribute("aria-labelledby")).toBeTruthy();
    expect(screen.getByText(/AI Debug/i)).toBeTruthy();
  });
});
