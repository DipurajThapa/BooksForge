/**
 * Vitest unit tests for the onboarding tour helpers (BACKLOG §K5).
 *
 * The tour itself is a React component (rendered tests would need
 * @testing-library/react setup); here we test the storage helpers
 * which are pure-logic and easily isolable.
 *
 * Run with `pnpm -C apps/desktop/src-ui test`.
 */
import { describe, it, expect, beforeEach } from "vitest";
import { shouldShowOnboarding, markOnboardingShown } from "./OnboardingTour";

const STORAGE_KEY = "booksforge.onboarding.v1.shown";

describe("OnboardingTour storage helpers", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it("shouldShowOnboarding returns true on a fresh install", () => {
    expect(shouldShowOnboarding()).toBe(true);
  });

  it("markOnboardingShown persists the flag", () => {
    markOnboardingShown();
    expect(localStorage.getItem(STORAGE_KEY)).toBe("1");
    expect(shouldShowOnboarding()).toBe(false);
  });

  it("shouldShowOnboarding returns false when flag is set to '1'", () => {
    localStorage.setItem(STORAGE_KEY, "1");
    expect(shouldShowOnboarding()).toBe(false);
  });

  it("treats arbitrary other values as 'not yet shown'", () => {
    // Any non-"1" value means the user hasn't completed the tour;
    // re-show it rather than swallowing data the next version of the
    // app might want to inspect.
    localStorage.setItem(STORAGE_KEY, "legacy");
    expect(shouldShowOnboarding()).toBe(true);
  });

  it("survives a localStorage exception by returning false", () => {
    // jsdom doesn't simulate quota exhaustion easily, but the helper
    // catches any throw and returns the safe default.  We exercise
    // this path by stubbing Storage.prototype.getItem.
    const original = Storage.prototype.getItem;
    Storage.prototype.getItem = () => { throw new Error("boom"); };
    try {
      // The helper catches the exception and returns false (safe
      // default — better to skip the tour than block the app on a
      // localStorage edge case).
      expect(shouldShowOnboarding()).toBe(false);
    } finally {
      Storage.prototype.getItem = original;
    }
  });
});
