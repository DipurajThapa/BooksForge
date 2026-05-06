//! Typed wrappers around Tauri IPC `invoke` calls.

import { invoke } from "@tauri-apps/api/core";
import type {
  CreateProjectInput,
  OpenProjectInput,
  OpenProjectResult,
  RecentProjectEntry,
} from "@booksforge/shared-types";

export const ipc = {
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
};
