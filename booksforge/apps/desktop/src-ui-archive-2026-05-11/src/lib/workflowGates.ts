/**
 * Workflow approval-gate state — Phase 9 of `PRODUCT_ROADMAP_E2E.md`
 * (closes UX recommendation R6 from the audit).
 *
 * Four explicit checkpoints in the manuscript factory:
 *
 *   1. **Topic gate**       — after the intake agent extracts a brief.
 *   2. **Plan gate**        — after the outline agent proposes structure.
 *   3. **Bibles gate**      — after character + world bibles are drafted.
 *   4. **Pre-final-polish** — after the polish stack finishes, before the
 *                              scene-critic / final-polish pass.
 *
 * Each gate is either `unset` (not visited yet), `pending` (the prior
 * agent has produced output and is waiting on the user), or `approved`
 * (the user has signed off and the workflow can advance).
 *
 * State is persisted per project in `localStorage` so closing/reopening
 * the project (or quitting the app) doesn't lose the writer's place.
 *
 * Privacy: localStorage is local to the app data directory; gate state
 * is not transmitted anywhere and contains no manuscript content.
 */

export type GateId =
  | "topic"
  | "plan"
  | "bibles"
  | "pre_final_polish";

export type GateStatus = "unset" | "pending" | "approved";

export interface GateState {
  status:    GateStatus;
  /** ISO 8601 timestamp of the last status change. */
  changed_at: string;
  /** Optional free-text note the user left when approving. */
  note?:     string;
}

export interface WorkflowState {
  topic:             GateState;
  plan:              GateState;
  bibles:            GateState;
  pre_final_polish:  GateState;
}

const STORAGE_PREFIX = "bf-workflow-gates";
const SETTINGS_KEY   = "bf-workflow-gates-enabled";

const DEFAULT_GATE: GateState = { status: "unset", changed_at: "" };

const DEFAULT_STATE: WorkflowState = {
  topic:            { ...DEFAULT_GATE },
  plan:             { ...DEFAULT_GATE },
  bibles:           { ...DEFAULT_GATE },
  pre_final_polish: { ...DEFAULT_GATE },
};

/**
 * True if the user has approval gates enabled (the default).
 * False = "advanced mode" — every gate is treated as auto-approved.
 */
export function gatesEnabled(): boolean {
  try {
    const raw = window.localStorage.getItem(SETTINGS_KEY);
    if (raw === null) return true;     // default: gates ON
    return raw === "true";
  } catch {
    return true;
  }
}

export function setGatesEnabled(enabled: boolean): void {
  try {
    window.localStorage.setItem(SETTINGS_KEY, enabled ? "true" : "false");
  } catch {
    /* localStorage unavailable — silently fall back to in-memory only */
  }
}

function storageKey(projectId: string): string {
  return `${STORAGE_PREFIX}:${projectId}`;
}

export function loadWorkflowState(projectId: string): WorkflowState {
  try {
    const raw = window.localStorage.getItem(storageKey(projectId));
    if (!raw) return clone(DEFAULT_STATE);
    const parsed = JSON.parse(raw);
    return mergeWithDefaults(parsed);
  } catch {
    return clone(DEFAULT_STATE);
  }
}

export function saveWorkflowState(projectId: string, state: WorkflowState): void {
  try {
    window.localStorage.setItem(storageKey(projectId), JSON.stringify(state));
  } catch {
    /* swallow — non-critical persistence */
  }
}

export function setGate(
  projectId: string,
  gate:      GateId,
  status:    GateStatus,
  note?:     string,
): WorkflowState {
  const current = loadWorkflowState(projectId);
  current[gate] = {
    status,
    changed_at: new Date().toISOString(),
    note,
  };
  saveWorkflowState(projectId, current);
  return current;
}

export function resetWorkflowState(projectId: string): WorkflowState {
  const fresh = clone(DEFAULT_STATE);
  saveWorkflowState(projectId, fresh);
  return fresh;
}

/**
 * Returns the gate that's blocking the workflow (the next pending one),
 * or `null` if all gates are either approved or unset (= not yet
 * relevant). Used by the WorkflowGuide panel to highlight what the
 * writer needs to action next.
 */
export function nextPendingGate(state: WorkflowState): GateId | null {
  const order: GateId[] = ["topic", "plan", "bibles", "pre_final_polish"];
  for (const g of order) {
    if (state[g].status === "pending") return g;
  }
  return null;
}

/**
 * For a chained workflow that walks past every gate (e.g. the full-scene
 * pipeline command runs draft → critic → polish → tells in one shot),
 * returns the first gate that is not yet `approved`. Caller should
 * refuse to launch the workflow when this returns non-null.
 *
 * Intentionally distinct from `nextPendingGate`: a gate stuck at
 * `unset` blocks a chained workflow (the writer never approved it),
 * even though `nextPendingGate` returns null in that state. Per-agent
 * runs use a softer check; chained workflows use this one.
 *
 * Returns null when gates are disabled (advanced mode) or every gate
 * is approved.
 */
export function firstUnapprovedGate(state: WorkflowState): GateId | null {
  if (!gatesEnabled()) return null;
  const order: GateId[] = ["topic", "plan", "bibles", "pre_final_polish"];
  for (const g of order) {
    if (state[g].status !== "approved") return g;
  }
  return null;
}

// ── helpers ─────────────────────────────────────────────────────────────────

function clone<T>(v: T): T {
  return JSON.parse(JSON.stringify(v));
}

function mergeWithDefaults(parsed: unknown): WorkflowState {
  const out: WorkflowState = clone(DEFAULT_STATE);
  if (typeof parsed !== "object" || parsed === null) return out;
  const obj = parsed as Record<string, unknown>;
  for (const key of ["topic", "plan", "bibles", "pre_final_polish"] as const) {
    const val = obj[key];
    if (typeof val === "object" && val !== null) {
      const v = val as Partial<GateState>;
      if (v.status === "unset" || v.status === "pending" || v.status === "approved") {
        out[key] = {
          status:     v.status,
          changed_at: typeof v.changed_at === "string" ? v.changed_at : "",
          note:       typeof v.note === "string" ? v.note : undefined,
        };
      }
    }
  }
  return out;
}

// ── Display metadata (kept here so panels stay declarative) ────────────────

export const GATE_LABELS: Record<GateId, string> = {
  topic:            "Topic & angle",
  plan:             "Plan / outline",
  bibles:           "Character + world bibles",
  pre_final_polish: "Pre-final-polish review",
};

export const GATE_BLURBS: Record<GateId, string> = {
  topic:            "Has the AI captured what you actually want to write about? This is the cheapest place to course-correct.",
  plan:             "Does the outline match your vision? Add/remove chapters, reorder beats, swap themes before any prose is drafted.",
  bibles:           "Are the characters and world consistent with what you imagined? Bible drift here propagates into every scene.",
  pre_final_polish: "Polish passes are about to apply — confirm the manuscript is structurally where you want it before the heavy stylistic edits.",
};

export const GATE_PRECEDED_BY: Record<GateId, string> = {
  topic:            "Intake agent",
  plan:             "Outline agent",
  bibles:           "Character + world bible agents",
  pre_final_polish: "Polish stack (dialogue / metaphor / voice / scene-tension)",
};
