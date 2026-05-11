/**
 * OnboardingTour component smoke tests — the three-step welcome
 * overlay shown once per browser/local-storage session.
 *
 * Note: pure-function tests for `shouldShowOnboarding` /
 * `markOnboardingShown` live in `OnboardingTour.test.ts`; this file
 * is the React-render side of the same component.
 */
// eslint-disable-next-line @typescript-eslint/no-unused-vars
import * as _React from "react";
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";

import OnboardingTour from "./OnboardingTour";

describe("OnboardingTour", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it("renders without crashing on the first step", () => {
    render(<OnboardingTour onClose={vi.fn()} />);
    expect(screen.getByRole("dialog")).toBeTruthy();
    expect(screen.getByText(/Welcome to BooksForge/i)).toBeTruthy();
  });

  it("Skip dismisses and marks onboarding shown", () => {
    const onClose = vi.fn();
    render(<OnboardingTour onClose={onClose} />);
    fireEvent.click(screen.getByRole("button", { name: /skip onboarding/i }));
    expect(onClose).toHaveBeenCalledTimes(1);
    expect(localStorage.getItem("booksforge.onboarding.v1.shown")).toBe("1");
  });

  it("Next advances through the three steps then dismisses", () => {
    const onClose = vi.fn();
    render(<OnboardingTour onClose={onClose} />);
    // Step 1 → Step 2
    fireEvent.click(screen.getByRole("button", { name: /^next$/i }));
    expect(screen.getByText(/Snapshots have your back/i)).toBeTruthy();
    // Step 2 → Step 3
    fireEvent.click(screen.getByRole("button", { name: /^next$/i }));
    expect(screen.getByText(/Agents are optional/i)).toBeTruthy();
    // Step 3 → "Got it" closes
    fireEvent.click(screen.getByRole("button", { name: /got it/i }));
    expect(onClose).toHaveBeenCalledTimes(1);
  });
});
