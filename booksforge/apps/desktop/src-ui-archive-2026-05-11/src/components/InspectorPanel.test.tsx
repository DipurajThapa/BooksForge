import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import type { NodeInfo } from "@booksforge/shared-types";
import { mockIpcModule } from "../test-utils/mockIpc";

vi.mock("../lib/ipc", () => mockIpcModule());

import InspectorPanel from "./InspectorPanel";

/**
 * Smoke tests for InspectorPanel.
 *
 * Right-pane outline-metadata editor. With `node={null}` it shows an
 * empty-state hint; with a node it renders the title + status + POV
 * inputs. These tests cover both branches at the smoke level.
 */
describe("InspectorPanel", () => {
  it("render-without-crash with no node selected", () => {
    render(<InspectorPanel node={null} onSaved={() => undefined} />);
    expect(screen.getByText(/Select a node/i)).toBeTruthy();
  });

  it("renders editable fields when a node is supplied", () => {
    const node: NodeInfo = {
      id: "01NODE00000000000000000000",
      parent_id: null,
      kind: "scene",
      title: "Opening Scene",
      position: "0|i00000:",
      status: "drafting",
      pov: "Alex",
      beat: "Inciting incident",
      target_words: 1500,
      word_count: 0,
      created_at: "2026-01-01T00:00:00Z",
      updated_at: "2026-01-01T00:00:00Z",
    };
    const { container } = render(
      <InspectorPanel node={node} onSaved={() => undefined} />,
    );
    // The title input mirrors the node title on mount.
    const inputs = container.querySelectorAll("input");
    expect(inputs.length).toBeGreaterThan(0);
  });
});
