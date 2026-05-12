import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";

import OutlinePreview, { type OutlineProposal } from "./OutlinePreview";

/**
 * Smoke tests for OutlinePreview.
 *
 * Pure presentational component — renders a Parts/Chapters/Scenes
 * tree from a proposal. No IPC; no mocks needed.
 */
describe("OutlinePreview", () => {
  const emptyProposal: OutlineProposal = {
    parts: [],
    rationale: "",
    notes_to_user: [],
  };

  const sampleProposal: OutlineProposal = {
    parts: [
      {
        title: "Part One",
        purpose: "Setup",
        chapters: [
          {
            title: "Opening",
            purpose: "Introduce the world",
            scenes: [
              {
                synopsis: "The hero wakes.",
                pov: "Alex",
                beat: "ordinary world",
                target_word_count: 1200,
              },
            ],
          },
        ],
      },
    ],
    rationale: "Three-act with classic setup",
    notes_to_user: ["Heavy on interiority in Act 1."],
  };

  it("render-without-crash with an empty proposal", () => {
    const { container } = render(<OutlinePreview proposal={emptyProposal} />);
    expect(container.firstChild).toBeTruthy();
  });

  it("renders the parts/chapters/scenes summary line", () => {
    render(<OutlinePreview proposal={sampleProposal} />);
    expect(screen.getByText(/parts/)).toBeTruthy();
    expect(screen.getByText("Part One")).toBeTruthy();
    expect(screen.getByText(/Opening/)).toBeTruthy();
    expect(screen.getByText("The hero wakes.")).toBeTruthy();
  });
});
