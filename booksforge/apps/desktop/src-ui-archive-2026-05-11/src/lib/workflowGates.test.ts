/**
 * workflowGates unit tests — per-project four-gate state machine
 * persisted in localStorage.
 */
import { describe, it, expect, beforeEach } from "vitest";
import {
  loadWorkflowState,
  setGate,
  resetWorkflowState,
  nextPendingGate,
  firstUnapprovedGate,
  gatesEnabled,
  setGatesEnabled,
  type GateId,
} from "./workflowGates";

const PROJECT_ID = "01TESTPROJECT00000000000000";
const ORDER: GateId[] = ["topic", "plan", "bibles", "pre_final_polish"];

describe("workflowGates", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it("loadWorkflowState returns defaults when nothing is persisted", () => {
    const state = loadWorkflowState(PROJECT_ID);
    for (const g of ORDER) {
      expect(state[g].status).toBe("unset");
      expect(state[g].changed_at).toBe("");
    }
  });

  it("setGate round-trips status + note via load", () => {
    setGate(PROJECT_ID, "topic", "approved", "looks good");
    const reloaded = loadWorkflowState(PROJECT_ID);
    expect(reloaded.topic.status).toBe("approved");
    expect(reloaded.topic.note).toBe("looks good");
    expect(reloaded.topic.changed_at.length).toBeGreaterThan(0);
  });

  it("nextPendingGate returns the first 'pending' gate (skips unset/approved)", () => {
    let state = setGate(PROJECT_ID, "topic", "approved");
    state = setGate(PROJECT_ID, "plan", "pending");
    state = setGate(PROJECT_ID, "bibles", "pending");
    expect(nextPendingGate(state)).toBe("plan");
  });

  it("nextPendingGate returns null when no gates are pending", () => {
    const state = loadWorkflowState(PROJECT_ID);
    expect(nextPendingGate(state)).toBeNull();
  });

  it("firstUnapprovedGate returns the first non-approved gate", () => {
    let state = setGate(PROJECT_ID, "topic", "approved");
    state = setGate(PROJECT_ID, "plan", "approved");
    // bibles + pre_final_polish remain unset — the first unapproved is bibles
    expect(firstUnapprovedGate(state)).toBe("bibles");
  });

  it("firstUnapprovedGate returns null when gates are disabled (advanced mode)", () => {
    setGatesEnabled(false);
    try {
      const state = loadWorkflowState(PROJECT_ID);
      expect(firstUnapprovedGate(state)).toBeNull();
    } finally {
      setGatesEnabled(true);
    }
  });

  it("gatesEnabled defaults to true on a fresh install", () => {
    expect(gatesEnabled()).toBe(true);
  });

  it("resetWorkflowState clears all four gates back to unset", () => {
    setGate(PROJECT_ID, "topic", "approved");
    setGate(PROJECT_ID, "plan", "pending");
    const reset = resetWorkflowState(PROJECT_ID);
    for (const g of ORDER) {
      expect(reset[g].status).toBe("unset");
    }
  });
});
