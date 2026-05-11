import { describe, it, expect, vi } from "vitest";
import { render } from "@testing-library/react";
import type { OpenProjectResult } from "@booksforge/shared-types";
import { mockIpcModule } from "../test-utils/mockIpc";

vi.mock("../lib/ipc", () => mockIpcModule());

// EditorShell pulls in the @booksforge/editor TipTap bundle which
// instantiates ProseMirror. ProseMirror's view layer wants a real
// browser context and crashes under jsdom unless its DOM-touching
// methods are stubbed. The smoke test only needs the shell itself
// to mount, so we replace the editor exports with inert stand-ins.
vi.mock("@booksforge/editor", () => ({
  EditorToolbar: () => null,
  SceneEditor: () => null,
}));

import EditorShell from "./EditorShell";

/**
 * Smoke test for EditorShell.
 *
 * EditorShell is the three-pane workspace. It owns far too much state
 * to test exhaustively here — covering it in unit tests would require
 * mocking every IPC call, the TipTap editor handle, and the OS save
 * dialog. This smoke test just confirms it mounts without throwing
 * given a valid project + the shared IPC mock.
 */
describe("EditorShell", () => {
  const project: OpenProjectResult = {
    project_id: "01TESTPROJECT00000000000000",
    title: "Test Project",
    author: "Test Author",
    bundle_path: "/tmp/test.booksforge",
    book_kind: null,
  };

  it("render-without-crash", () => {
    const { container } = render(
      <EditorShell project={project} onClose={() => undefined} />,
    );
    expect(container.firstChild).toBeTruthy();
  });
});
