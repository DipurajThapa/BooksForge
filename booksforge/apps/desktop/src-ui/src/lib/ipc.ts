//! Typed wrappers around Tauri IPC `invoke` calls.

import { invoke } from "@tauri-apps/api/core";
import type {
  CreateProjectInput,
  NodeCreateInput,
  NodeInfo,
  NodeUpdateInput,
  OpenProjectInput,
  OpenProjectResult,
  RecentProjectEntry,
  RecoveryStatus,
  SceneLoadResult,
  SceneSaveInput,
} from "@booksforge/shared-types";

export const ipc = {
  // ── Project lifecycle ─────────────────────────────────────────────────────
  projectCreate(input: CreateProjectInput): Promise<OpenProjectResult> {
    return invoke("project_create", { input });
  },
  projectOpen(input: OpenProjectInput): Promise<OpenProjectResult> {
    return invoke("project_open", { input });
  },
  projectClose(): Promise<void> {
    return invoke("project_close");
  },
  projectRecent(): Promise<RecentProjectEntry[]> {
    return invoke("project_recent");
  },

  // ── Document tree (nodes) ─────────────────────────────────────────────────
  nodeList(): Promise<NodeInfo[]> {
    return invoke("node_list");
  },
  nodeCreate(input: NodeCreateInput): Promise<NodeInfo> {
    return invoke("node_create", { input });
  },
  nodeUpdate(input: NodeUpdateInput): Promise<NodeInfo> {
    return invoke("node_update", { input });
  },
  nodeDelete(id: string): Promise<void> {
    return invoke("node_delete", { id });
  },

  // ── Scene content ─────────────────────────────────────────────────────────
  sceneSave(input: SceneSaveInput): Promise<void> {
    return invoke("scene_save", { input });
  },
  sceneLoad(nodeId: string): Promise<SceneLoadResult | null> {
    return invoke("scene_load", { nodeId });
  },

  // ── Crash recovery ────────────────────────────────────────────────────────
  recoveryCheck(): Promise<RecoveryStatus> {
    return invoke("recovery_check");
  },
  recoveryClear(): Promise<void> {
    return invoke("recovery_clear");
  },
};
