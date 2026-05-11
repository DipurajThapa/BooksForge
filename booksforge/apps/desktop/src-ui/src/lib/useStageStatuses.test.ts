/**
 * Unit tests for the pure `computeStatuses` function inside
 * `useStageStatuses`. The hook itself does IPC + React state and is
 * exercised via component tests; here we lock down the rule table
 * documented in the module header.
 */
import { describe, expect, it } from "vitest";
import { __test } from "./useStageStatuses";

const { computeStatuses, isDefaultAudience } = __test;

const BLANK = {
  brief:              null,
  briefLoaded:        false,
  hasCharacterBible:  false,
  hasWorldBible:      false,
  sceneCount:         0,
  draftedSceneCount:  0,
};

describe("isDefaultAudience", () => {
  it("returns true for the wizard's stock strings", () => {
    expect(isDefaultAudience("general readers")).toBe(true);
    expect(isDefaultAudience("Adult Literary Readers")).toBe(true);
    expect(isDefaultAudience("")).toBe(true);
    expect(isDefaultAudience(undefined)).toBe(true);
  });
  it("returns false for any customised audience string", () => {
    expect(isDefaultAudience("women aged 35-50 who grew up reading horror")).toBe(false);
    expect(isDefaultAudience("Polish-Catholic family-saga readers")).toBe(false);
  });
});

describe("computeStatuses", () => {
  it("returns all-locked-ish for a blank project", () => {
    const r = computeStatuses(BLANK);
    expect(r.setup).toBe("available");
    expect(r.audience).toBe("locked");
    expect(r.characters).toBe("available");  // bibles are optional
    expect(r.outline).toBe("locked");
    expect(r.drafting).toBe("locked");
    expect(r.export).toBe("locked");
  });

  it("marks setup passed when brief has premise + a key promise", () => {
    const r = computeStatuses({
      ...BLANK,
      briefLoaded: true,
      brief: {
        premise:      "A widow finds twenty-three sealed letters.",
        key_promises: ["Sustained dread anchored in everyday objects"],
      },
    });
    expect(r.setup).toBe("passed");
    expect(r.audience).toBe("available");  // unlocked now
  });

  it("marks setup in_progress when brief is loaded but premise empty", () => {
    const r = computeStatuses({
      ...BLANK,
      briefLoaded: true,
      brief: { premise: "", key_promises: [] },
    });
    expect(r.setup).toBe("in_progress");
    expect(r.audience).toBe("locked");  // setup not passed yet
  });

  it("marks audience passed when 2+ audience fields are non-default", () => {
    const r = computeStatuses({
      ...BLANK,
      briefLoaded: true,
      brief: {
        premise:                "Premise here.",
        key_promises:           ["promise one"],
        audience:               "horror readers aged 30-55",
        comp_titles_or_authors: ["Marilynne Robinson"],
      },
    });
    expect(r.setup).toBe("passed");
    expect(r.audience).toBe("passed");
  });

  it("marks audience in_progress with only one audience field filled", () => {
    const r = computeStatuses({
      ...BLANK,
      briefLoaded: true,
      brief: {
        premise:      "Premise.",
        key_promises: ["promise"],
        audience:     "general readers",  // stock default
        theme_keywords: ["loneliness"],   // one field filled
      },
    });
    expect(r.audience).toBe("in_progress");
  });

  it("marks bibles passed when at least one bible exists", () => {
    expect(computeStatuses({ ...BLANK, hasCharacterBible: true }).characters)
      .toBe("passed");
    expect(computeStatuses({ ...BLANK, hasWorldBible: true }).characters)
      .toBe("passed");
    expect(computeStatuses({
      ...BLANK,
      hasCharacterBible: true,
      hasWorldBible:     true,
    }).characters).toBe("passed");
  });

  it("marks outline passed once scenes exist", () => {
    const r = computeStatuses({
      ...BLANK,
      briefLoaded: true,
      brief: { premise: "x", key_promises: ["y"] },
      sceneCount: 12,
    });
    expect(r.outline).toBe("passed");
    expect(r.drafting).toBe("available");
  });

  it("marks drafting passed only when every scene has prose", () => {
    expect(computeStatuses({
      ...BLANK, sceneCount: 12, draftedSceneCount: 12,
    }).drafting).toBe("passed");
    expect(computeStatuses({
      ...BLANK, sceneCount: 12, draftedSceneCount: 4,
    }).drafting).toBe("in_progress");
    expect(computeStatuses({
      ...BLANK, sceneCount: 12, draftedSceneCount: 0,
    }).drafting).toBe("available");
  });

  it("unlocks export only once at least one scene has prose", () => {
    expect(computeStatuses({ ...BLANK }).export).toBe("locked");
    expect(computeStatuses({
      ...BLANK, sceneCount: 12, draftedSceneCount: 1,
    }).export).toBe("available");
  });

  it("never locks the bibles stage (it's optional in MVP)", () => {
    // Even with a totally blank project, bibles is reachable so the
    // writer can pre-author them before generating an outline.
    expect(computeStatuses(BLANK).characters).toBe("available");
  });
});
