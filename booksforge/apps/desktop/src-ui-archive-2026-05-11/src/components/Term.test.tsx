/**
 * <Term> smoke tests — inline glossary tooltip used throughout the UI.
 * Pure React; no IPC mock needed.
 */
// eslint-disable-next-line @typescript-eslint/no-unused-vars
import * as _React from "react";
import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";

import Term from "./Term";

describe("Term", () => {
  it("renders the glossary entry's label for a known key", () => {
    const { container } = render(<Term k="KDP" />);
    expect(container.textContent).toContain("KDP");
  });

  it("falls back to the raw key when the glossary key is unknown", () => {
    const { container } = render(<Term k="not-a-real-key" />);
    expect(container.textContent).toContain("not-a-real-key");
  });

  it("respects children as a label override", () => {
    render(<Term k="EPUB">EPUB-3 file</Term>);
    expect(screen.getByText("EPUB-3 file")).toBeTruthy();
  });

  it("sets a title attribute with the short description (a11y/tooltip)", () => {
    const { container } = render(<Term k="KDP" />);
    const span = container.querySelector("[title]");
    expect(span).toBeTruthy();
    expect(span?.getAttribute("title")?.length).toBeGreaterThan(0);
  });
});
