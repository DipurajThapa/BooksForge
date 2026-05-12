/**
 * HumanizationPanel smoke tests — humanizer agent with per-edit Accept
 * for AI-tell rewrites (BACKLOG §E0d.6).  Closes part of
 * EXTERNAL_AUDIT_BACKLOG.md #22.
 */
// eslint-disable-next-line @typescript-eslint/no-unused-vars
import * as _React from "react";
import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { mockIpcModule } from "../../test-utils/mockIpc";

vi.mock("../../lib/ipc", () => mockIpcModule());

import HumanizationPanel from "./HumanizationPanel";

const PROJECT_ID = "01TESTPROJECT00000000000000";
const SCENE_ID   = "01SCENE00000000000000000000";
const MODEL      = "qwen3.5:9b";

describe("HumanizationPanel", () => {
  it("renders without crashing", () => {
    render(
      <HumanizationPanel
        projectId={PROJECT_ID}
        sceneId={SCENE_ID}
        model={MODEL}
        onClose={vi.fn()}
      />,
    );
    expect(screen.getByRole("dialog")).toBeTruthy();
  });

  it("dialog has correct a11y attributes and a close affordance", () => {
    const onClose = vi.fn();
    render(
      <HumanizationPanel
        projectId={PROJECT_ID}
        sceneId={null}
        model={MODEL}
        onClose={onClose}
      />,
    );
    const dialog = screen.getByRole("dialog");
    expect(dialog.getAttribute("aria-modal")).toBe("true");
    expect(dialog.getAttribute("aria-labelledby")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: /close panel/i }));
    expect(onClose).toHaveBeenCalledTimes(1);
  });
});
