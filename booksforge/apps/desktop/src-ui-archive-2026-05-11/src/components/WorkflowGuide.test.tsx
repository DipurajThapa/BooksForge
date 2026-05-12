/**
 * WorkflowGuide smoke tests — Phase 9 four-gate approval guide.
 */
// eslint-disable-next-line @typescript-eslint/no-unused-vars
import * as _React from "react";
import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";

import WorkflowGuide from "./WorkflowGuide";

const PROJECT_ID = "01TESTPROJECT00000000000000";

describe("WorkflowGuide", () => {
  it("renders without crashing", () => {
    render(<WorkflowGuide projectId={PROJECT_ID} onClose={vi.fn()} />);
    expect(screen.getByRole("dialog")).toBeTruthy();
    expect(screen.getByText("Workflow guide")).toBeTruthy();
  });

  it("dialog has correct a11y attributes and Close button fires onClose", () => {
    const onClose = vi.fn();
    render(<WorkflowGuide projectId={PROJECT_ID} onClose={onClose} />);
    const dialog = screen.getByRole("dialog");
    expect(dialog.getAttribute("aria-modal")).toBe("true");
    expect(dialog.getAttribute("aria-labelledby")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: /close panel/i }));
    expect(onClose).toHaveBeenCalledTimes(1);
  });
});
