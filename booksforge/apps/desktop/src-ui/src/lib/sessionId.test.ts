import { describe, it, expect, beforeEach } from "vitest";
import { getSessionId, __resetSessionIdForTests } from "./sessionId";

describe("sessionId", () => {
  beforeEach(() => {
    __resetSessionIdForTests();
  });

  it("returns a non-empty string", () => {
    const id = getSessionId();
    expect(typeof id).toBe("string");
    expect(id.length).toBeGreaterThan(10);
  });

  it("returns the same value across calls within a session", () => {
    const a = getSessionId();
    const b = getSessionId();
    expect(a).toBe(b);
  });

  it("rotates when reset", () => {
    const before = getSessionId();
    __resetSessionIdForTests();
    const after = getSessionId();
    expect(after).not.toBe(before);
  });

  it("supports an explicit override for tests", () => {
    __resetSessionIdForTests("01HFAKE000000000000000FAKE");
    expect(getSessionId()).toBe("01HFAKE000000000000000FAKE");
  });

  it("contains only base-36 uppercase characters", () => {
    __resetSessionIdForTests();
    const id = getSessionId();
    expect(id).toMatch(/^[0-9A-Z]+$/);
  });
});
