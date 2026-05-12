/**
 * EditorShell — minimal foundation for the writer's editor (2026-05 redesign).
 *
 * Three regions:
 *   ┌─────────────┬───────────────────────────────────────────────────┐
 *   │ StageRail   │ Stage panel (Setup / Audience / … / Export)       │
 *   │   (220px)   │   — content depends on the active stage           │
 *   ├─────────────┴───────────────────────────────────────────────────┤
 *   │ (footer / status — currently empty)                             │
 *   └────────────────────────────────────────────────────────────────┘
 *
 * Stage status:
 *   The rail's coloured dots are derived live from project data via
 *   `useStageStatuses`. We refresh:
 *     1. On mount (initial render).
 *     2. Whenever the user switches to a new stage in the rail —
 *        re-fetching catches changes the previous stage just made.
 *   That covers the common writer-flow patterns ("edit Stage 1, click
 *   Stage 4 to see it unlocked") without per-panel refresh hooks.
 */
import { useCallback, useState } from "react";
import type { OpenProjectResult } from "@booksforge/shared-types";
import StageRail, { MVP_STAGES, type StageId, type StageInfo } from "../components/StageRail";
import { useStageStatuses } from "../lib/useStageStatuses";

import Stage1_Setup       from "../stages/Stage1_Setup";
import Stage2_Audience    from "../stages/Stage2_Audience";
import Stage5_Characters  from "../stages/Stage5_Characters";
import Stage7_Outline     from "../stages/Stage7_Outline";
import Stage8_Drafting    from "../stages/Stage8_Drafting";
import Stage13_14_Export  from "../stages/Stage13_14_Export";

interface Props {
  project: OpenProjectResult;
  onClose: () => void;
}

export default function EditorShell({ project, onClose }: Props) {
  const [active,     setActive]     = useState<StageId>("setup");
  // refreshKey is bumped on stage switch so useStageStatuses re-fetches
  // and the rail's dots reflect any cross-stage change.
  const [refreshKey, setRefreshKey] = useState<number>(0);
  const { statuses, refresh } = useStageStatuses(project, refreshKey);

  const handleSelect = useCallback((next: StageId) => {
    setActive(next);
    setRefreshKey((k) => k + 1);
  }, []);

  // Manual refresh trigger we hand to stage panels that need to push
  // an update mid-stage (e.g. after a successful save the panel can
  // call this so the rail dot turns green without a tab switch).
  const handleStageProgress = useCallback(() => {
    void refresh();
  }, [refresh]);

  // F5 — "Save & continue" advances the rail one step forward. The
  // panel calls this after a successful save so the writer doesn't
  // have to click the rail manually. We refresh statuses along the
  // way so the dot for the just-finished stage flips green before
  // the next one mounts.
  const handleStageAdvance = useCallback(() => {
    const order = MVP_STAGES.map((s) => s.id);
    setActive((current) => {
      const idx = order.indexOf(current);
      if (idx < 0 || idx >= order.length - 1) return current;
      const next = order[idx + 1];
      return next ?? current;
    });
    setRefreshKey((k) => k + 1);
  }, []);

  // The rail merges the static stage definition with the live status.
  // Active stage always shows "in_progress" if it isn't already "passed"
  // — that's the dot the writer sees beside the stage they're typing in.
  const stages: StageInfo[] = MVP_STAGES.map((s) => {
    const computed = statuses[s.id];
    const status =
      s.id === active && computed !== "passed" && computed !== "failed"
        ? "in_progress"
        : computed;
    return { ...s, status };
  });

  return (
    <div style={s.shell}>
      <header style={s.header}>
        <span style={s.wordmark}>BooksForge</span>
        <span style={s.projectTitle}>{project.title}</span>
        <div style={s.headerRight}>
          <span style={s.refreshHint} title="Statuses refresh on every stage switch">
            ↻ live
          </span>
          <button style={s.closeBtn} onClick={onClose}>Close</button>
        </div>
      </header>

      <div style={s.body}>
        <StageRail stages={stages} active={active} onSelect={handleSelect} />
        <main style={s.main}>
          {active === "setup"      && <Stage1_Setup      project={project} onChanged={handleStageProgress} onAdvance={handleStageAdvance} />}
          {active === "audience"   && <Stage2_Audience   project={project} onChanged={handleStageProgress} onAdvance={handleStageAdvance} />}
          {active === "characters" && <Stage5_Characters project={project} onChanged={handleStageProgress} />}
          {active === "outline"    && <Stage7_Outline    project={project} onChanged={handleStageProgress} />}
          {active === "drafting"   && <Stage8_Drafting   project={project} onChanged={handleStageProgress} />}
          {active === "export"     && <Stage13_14_Export project={project} onChanged={handleStageProgress} />}
        </main>
      </div>
    </div>
  );
}

const s: Record<string, React.CSSProperties> = {
  shell: {
    minHeight: "100vh",
    display: "flex", flexDirection: "column",
    background: "var(--color-neutral-50)",
    fontFamily: "var(--font-ui)",
  },
  header: {
    height: 48,
    padding: "0 16px",
    display: "flex", alignItems: "center", gap: 16,
    background: "#fff",
    borderBottom: "1px solid var(--color-neutral-200)",
    flexShrink: 0,
  },
  wordmark: {
    fontFamily: "var(--font-prose, serif)", fontSize: 18, fontWeight: 700,
    color: "var(--color-amber-600)",
  },
  projectTitle: {
    flex: 1, fontSize: 14, fontWeight: 500,
    color: "var(--color-neutral-900)",
    overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap",
  },
  headerRight: { display: "flex", gap: 8, alignItems: "center" },
  refreshHint: {
    fontSize: 11,
    color: "var(--color-neutral-400)",
    fontFamily: "var(--font-mono)",
    cursor: "default",
  },
  closeBtn: {
    background: "none", border: "1px solid var(--color-neutral-300)",
    borderRadius: 4, padding: "4px 12px", fontSize: 12,
    color: "var(--color-neutral-700)", cursor: "pointer",
  },
  body: { flex: 1, display: "flex", overflow: "hidden" },
  main: {
    flex: 1, overflow: "auto",
    background: "#fff",
  },
};
