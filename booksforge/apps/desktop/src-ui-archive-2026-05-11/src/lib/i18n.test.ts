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
    // The fallback path returns the key as the value, so when the key
    // contains `{n}` and we pass `{ n: 42 }`, interpolation runs over
    // the fallback key just like it would over a real translation —
    // this is the documented behaviour and lets us test the
    // interpolator without adding a numeric key to the default dict.
    expect(t("custom.key.{n}", { n: 42 })).toBe("custom.key.42");
    // Realistic case: simulated key that DOES use {n}.
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
