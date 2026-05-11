/**
 * DevelopmentalReviewPanel smoke tests — chapter-level dev-editor +
 * per-scene continuity linter chained workflow (BACKLOG §F2).  Note
 * the panel signature only takes projectId/model/onClose (no sceneId,
 * no onApplied) — the chapter-id is entered by the user inside the
 * form.  Closes part of EXTERNAL_AUDIT_BACKLOG.md #22.
 */
// eslint-disable-next-line @typescript-eslint/no-unused-vars
import * as _React from "react";
import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { mockIpcModule } from "../../test-utils/mockIpc";

vi.mock("../../lib/ipc", () => mockIpcModule());

import DevelopmentalReviewPanel from "./DevelopmentalReviewPanel";

const PROJECT_ID = "01TESTPROJECT00000000000000";
const MODEL      = "qwen3.5:9b";

describe("DevelopmentalReviewPanel", () => {
  it("renders without crashing", () => {
    render(
      <DevelopmentalReviewPanel
        projectId={PROJECT_ID}
        model={MODEL}
        onClose={vi.fn()}
      />,
    );
    expect(screen.getByRole("dialog")).toBeTruthy();
  });

  it("dialog has correct a11y attributes and a close affordance", () => {
    const onClose = vi.fn();
    render(
      <DevelopmentalReviewPanel
        projectId={PROJECT_ID}
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
