import { describe, it, expect, beforeEach } from "vitest";
import { t, getLocale, setLocale, useT } from "./i18n";

describe("i18n", () => {
  beforeEach(() => {
    setLocale("en");
  });

  it("returns the dictionary value for a known key", () => {
    expect(t("app.title")).toBe("BooksForge");
  });

  it("falls back loudly to the key for an unknown lookup", () => {
    expect(t("definitely.not.a.real.key")).toBe("definitely.not.a.real.key");
  });

  it("interpolates {placeholder} parameters", () => {
    // The default dictionary's export-format string uses {format}.
    expect(t("export.dialog.runButton", { format: "EPUB" })).toBe("Export to EPUB");
  });

  it("interpolates numeric parameters", () => {
    // We don't have a numeric key in the default dict — exercise the
    // path with an unknown key so the value is the key itself, then
    // confirm the placeholder substitutes.
    expect(t("custom.key.{n}", { n: 42 })).toBe("custom.key.{n}");
    // Realistic case: simulated key that DOES use {n}.
    // (We test interpolation logic via the fallback path.)
    const out = t("Created {n} chapters", { n: 3 });
    expect(out).toBe("Created 3 chapters");
  });

  it("getLocale defaults to en", () => {
    expect(getLocale()).toBe("en");
  });

  it("setLocale ignores unknown locales", () => {
    // @ts-expect-error testing runtime guard
    setLocale("zz-INVALID");
    expect(getLocale()).toBe("en");
  });

  it("useT returns t and current locale", () => {
    const hookResult = useT();
    expect(typeof hookResult.t).toBe("function");
    expect(hookResult.locale).toBe("en");
    expect(hookResult.t("app.title")).toBe("BooksForge");
  });
});
