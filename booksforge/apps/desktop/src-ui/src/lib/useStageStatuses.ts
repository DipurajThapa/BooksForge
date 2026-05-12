/**
 * useStageStatuses — derives the per-stage status from existing project
 * data (brief, bibles, node tree). Drives the StageRail's coloured dots.
 *
 * No new backend IPC required: the status is *derived*, not stored.
 * That's the right call for MVP because:
 *   1. The data we'd persist (e.g. "Stage 1 passed at <ts>") is
 *      strictly less informative than the underlying brief / bibles /
 *      nodes — so we'd be storing a stale derivative.
 *   2. Re-derivation is cheap (three IPC calls, all already cached).
 *   3. Audit-trail can be added later by writing pass/fail timestamps
 *      into project memory; the rail's colour stays derived.
 *
 * Status rules per stage (anchored to the 6-stage MVP — see
 * `book-output/design/WRITER_JOURNEY_REDESIGN_2026-05.md`):
 *
 *   1. setup
 *      passed       → brief.premise non-empty AND key_promises ≥ 1
 *      in_progress  → brief loaded but missing required fields
 *      available    → no brief loaded
 *
 *   2. audience
 *      passed       → ≥ 2 of {non-default audience, theme_keywords,
 *                            comp_titles_or_authors} non-empty
 *      in_progress  → at least 1 of those non-empty
 *      available    → setup passed but audience untouched
 *      locked       → setup hasn't even reached in_progress
 *
 *   3. characters (Bibles — characters OR world)
 *      passed       → has_character_bible OR has_world_bible
 *      available    → otherwise (this stage is OPTIONAL; auto-gen
 *                     happens in Stage 5 if skipped, so it never locks)
 *
 *   4. outline
 *      passed       → scene nodes exist (outline applied)
 *      available    → setup passed
 *      locked       → setup not even in_progress
 *
 *   5. drafting
 *      passed       → all scene nodes have word_count > 0
 *      in_progress  → some scenes drafted, some empty
 *      available    → scenes exist but none drafted
 *      locked       → no scenes exist yet
 *
 *   6. export
 *      available    → at least one scene has prose
 *      locked       → nothing to export yet
 *      (passed is not auto-detected; user explicitly clicks export)
 */
import { useCallback, useEffect, useState } from "react";
import type { OpenProjectResult } from "@booksforge/shared-types";
import { ipc } from "./ipc";
import type { StageId, StageStatus } from "../components/StageRail";

export type StageStatuses = Record<StageId, StageStatus>;

const DEFAULT_STATUSES: StageStatuses = {
  setup:      "available",
  audience:   "locked",
  characters: "available",
  outline:    "locked",
  drafting:   "locked",
  export:     "locked",
};

interface BriefLike {
  premise?:                string;
  audience?:               string;
  key_promises?:           string[];
  theme_keywords?:         string[];
  comp_titles_or_authors?: string[];
  forbidden_tropes?:       string[];
}

/**
 * Decide whether an `audience` string is the wizard's stock default vs
 * something the writer actually edited. The list intentionally captures
 * the strings the wizard emits at creation time — anything else
 * counts as customised.
 */
function isDefaultAudience(s: string | undefined): boolean {
  if (!s) return true;
  const t = s.trim().toLowerCase();
  return !t
    || t === "general readers"
    || t === "adult literary readers";
}

function computeStatuses(input: {
  brief:               BriefLike | null;
  briefLoaded:         boolean;
  hasCharacterBible:   boolean;
  hasWorldBible:       boolean;
  sceneCount:          number;
  draftedSceneCount:   number;
}): StageStatuses {
  const {
    brief, briefLoaded, hasCharacterBible, hasWorldBible,
    sceneCount, draftedSceneCount,
  } = input;

  // ── Stage 1: setup ──────────────────────────────────────────────
  let setup: StageStatus = "available";
  if (briefLoaded && brief) {
    const hasPremise  = !!brief.premise?.trim();
    const hasPromises = (brief.key_promises ?? []).length >= 1;
    setup = hasPremise && hasPromises ? "passed" : "in_progress";
  }

  // ── Stage 2: audience ───────────────────────────────────────────
  let audience: StageStatus = setup === "passed" ? "available" : "locked";
  if (briefLoaded && brief) {
    const filled = [
      !isDefaultAudience(brief.audience),
      (brief.theme_keywords ?? []).length > 0,
      (brief.comp_titles_or_authors ?? []).length > 0,
    ].filter(Boolean).length;
    if (filled >= 2)      audience = "passed";
    else if (filled >= 1) audience = "in_progress";
    // Otherwise leave as available/locked from the setup-gate above.
  }

  // ── Stage 3: bibles (optional) ──────────────────────────────────
  const characters: StageStatus =
    hasCharacterBible || hasWorldBible ? "passed" : "available";

  // ── Stage 4: outline ────────────────────────────────────────────
  let outline: StageStatus = setup === "passed" ? "available" : "locked";
  if (sceneCount > 0) outline = "passed";

  // ── Stage 5: drafting ───────────────────────────────────────────
  let drafting: StageStatus = "locked";
  if (sceneCount > 0) {
    if (draftedSceneCount === sceneCount) drafting = "passed";
    else if (draftedSceneCount > 0)        drafting = "in_progress";
    else                                   drafting = "available";
  }

  // ── Stage 6: export ─────────────────────────────────────────────
  const exportStage: StageStatus =
    draftedSceneCount > 0 ? "available" : "locked";

  return { setup, audience, characters, outline, drafting, export: exportStage };
}

interface UseStageStatusesReturn {
  statuses: StageStatuses;
  /** True on first load only — subsequent refreshes don't block render. */
  loading:  boolean;
  /** Re-fetch + recompute. Call after any cross-stage action. */
  refresh:  () => Promise<void>;
}

/**
 * Compute stage statuses for the active project. Refreshes on mount and
 * whenever `refreshKey` changes (the EditorShell bumps it on every stage
 * switch so navigating the rail picks up cross-stage changes).
 */
export function useStageStatuses(
  project: OpenProjectResult,
  refreshKey: number,
): UseStageStatusesReturn {
  void project; // future-proofing — currently every IPC reads from the open project state on the backend
  const [statuses, setStatuses] = useState<StageStatuses>(DEFAULT_STATUSES);
  const [loading,  setLoading]  = useState<boolean>(true);

  const refresh = useCallback(async () => {
    try {
      const [brief, bibles, nodes] = await Promise.all([
        ipc.projectBriefLoad(),
        ipc.biblesLoad(),
        ipc.nodeList(),
      ]);
      const sceneNodes = nodes.filter((n) => n.kind === "scene");
      const drafted   = sceneNodes.filter((n) => (n.word_count ?? 0) > 0);
      const next = computeStatuses({
        brief:              (brief.brief_json as BriefLike) ?? null,
        briefLoaded:        brief.loaded,
        hasCharacterBible:  bibles.has_character_bible,
        hasWorldBible:      bibles.has_world_bible,
        sceneCount:         sceneNodes.length,
        draftedSceneCount:  drafted.length,
      });
      setStatuses(next);
    } catch {
      // Best-effort: leave whatever we last computed. The rail simply
      // doesn't update; nothing breaks.
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh, refreshKey]);

  return { statuses, loading, refresh };
}

// ── Exports for tests ─────────────────────────────────────────────────

export const __test = { computeStatuses, isDefaultAudience };
