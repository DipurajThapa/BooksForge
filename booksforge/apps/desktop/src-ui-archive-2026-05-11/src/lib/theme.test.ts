import { describe, it, expect, beforeEach, vi } from "vitest";
import {
  applyTheme,
  getThemePreference,
  resolveTheme,
  setThemePreference,
} from "./theme";

const STORAGE_KEY = "bf-theme-preference";

describe("theme", () => {
  beforeEach(() => {
    localStorage.clear();
    document.documentElement.removeAttribute("data-theme");
  });

  it("getThemePreference defaults to system", () => {
    expect(getThemePreference()).toBe("system");
  });

  it("getThemePreference returns the persisted value", () => {
    localStorage.setItem(STORAGE_KEY, "dark");
    expect(getThemePreference()).toBe("dark");
  });

  it("getThemePreference falls back to system on garbage", () => {
    localStorage.setItem(STORAGE_KEY, "puce");
    expect(getThemePreference()).toBe("system");
  });

  it("setThemePreference persists and applies", () => {
    setThemePreference("dark");
    expect(localStorage.getItem(STORAGE_KEY)).toBe("dark");
    expect(document.documentElement.getAttribute("data-theme")).toBe("dark");
  });

  it("setThemePreference('light') applies even when system is dark", () => {
    // Mock matchMedia to return dark.
    vi.stubGlobal("matchMedia", () =>
      ({ matches: true, addEventListener: () => undefined, removeEventListener: () => undefined } as unknown as MediaQueryList),
    );
    setThemePreference("light");
    expect(document.documentElement.getAttribute("data-theme")).toBe("light");
    vi.unstubAllGlobals();
  });

  it("resolveTheme honours explicit preferences", () => {
    expect(resolveTheme("light")).toBe("light");
    expect(resolveTheme("dark")).toBe("dark");
  });

  it("resolveTheme('system') reads matchMedia", () => {
    vi.stubGlobal("matchMedia", () =>
      ({ matches: true, addEventListener: () => undefined, removeEventListener: () => undefined } as unknown as MediaQueryList),
    );
    expect(resolveTheme("system")).toBe("dark");
    vi.stubGlobal("matchMedia", () =>
      ({ matches: false, addEventListener: () => undefined, removeEventListener: () => undefined } as unknown as MediaQueryList),
    );
    expect(resolveTheme("system")).toBe("light");
    vi.unstubAllGlobals();
  });

  it("applyTheme is idempotent", () => {
    applyTheme("dark");
    applyTheme("dark");
    expect(document.documentElement.getAttribute("data-theme")).toBe("dark");
  });
});
