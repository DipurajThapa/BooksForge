/**
 * OutlineView — flat scene list with inline metadata editing.
 *
 * Per `outputs/UI_UX_SPEC.md §5.1`: "A second tab in the sidebar:
 * Outline view — flat list with synopsis, POV, beat, target word
 * count, status. Editing inline updates the same `nodes` rows."
 *
 * Scope of this PR:
 *   - Renders one card per scene, grouped by chapter, sorted by
 *     position.
 *   - Inline-editable per scene: title (header is also the click
 *     target to select), status (dropdown), POV (text), beat (text),
 *     target words (number).
 *   - Saves on blur via the parent's `onUpdateNode` handler. Synopsis
 *     is in the spec but isn't on the current `NodeInfo` payload —
 *     omitted until the data model exposes it.
 *
 * Backend boundary: this is purely presentational. The parent
 * (Manuscript) owns the `nodeUpdate` IPC; OutlineView never invokes
 * IPC directly.
 */
import { useMemo, useState } from "react";
import type { CSSProperties } from "react";
import type { NodeInfo } from "@booksforge/shared-types";

/** Subset of NodeInfo fields the writer can edit from the outline. */
export interface OutlineUpdate {
  title?:        string;
  status?:       string;
  pov?:          string | null;
  beat?:         string | null;
  target_words?: number | null;
}

interface Props {
  nodes:           NodeInfo[];
  selectedSceneId: string | null;
  onSelectScene:   (id: string) => void;
  onUpdateNode:    (id: string, patch: OutlineUpdate) => void;
}

const STATUS_VALUES = ["planned", "drafting", "revised", "final"] as const;

export default function OutlineView({ nodes, selectedSceneId, onSelectScene, onUpdateNode }: Props) {
  const grouped = useMemo(() => groupByChapter(nodes), [nodes]);

  if (grouped.length === 0) {
    return (
      <nav style={s.root} aria-label="Outline view">
        <h2 style={s.heading}>Outline</h2>
        <p style={s.empty}>
          No scenes yet. Generate an outline in Stage 4 and accept
          it — the scenes show up here for bulk metadata editing.
        </p>
      </nav>
    );
  }

  return (
    <nav style={s.root} aria-label="Outline view">
      <h2 style={s.heading}>Outline</h2>
      <div style={s.body}>
        {grouped.map((group) => (
          <section key={group.chapterId ?? "__orphan"} style={s.group}>
            <header style={s.groupHeader}>
              <span style={s.groupTitle}>{group.chapterTitle}</span>
              <span style={s.groupCount}>
                {group.scenes.length} scene{group.scenes.length === 1 ? "" : "s"}
              </span>
            </header>
            <ul style={s.cardList}>
              {group.scenes.map((scene) => (
                <SceneCard
                  key={scene.id}
                  scene={scene}
                  selected={scene.id === selectedSceneId}
                  onSelect={() => onSelectScene(scene.id)}
                  onUpdate={(patch) => onUpdateNode(scene.id, patch)}
                />
              ))}
            </ul>
          </section>
        ))}
      </div>
    </nav>
  );
}

// ── Grouping ────────────────────────────────────────────────────────

interface ChapterGroup {
  chapterId:    string | null;
  chapterTitle: string;
  scenes:       NodeInfo[];
}

function groupByChapter(nodes: NodeInfo[]): ChapterGroup[] {
  const scenes = nodes
    .filter((n) => n.kind === "scene")
    .sort((a, b) => a.position.localeCompare(b.position));
  const chapters = new Map<string, NodeInfo>();
  for (const n of nodes) {
    if (n.kind === "chapter") chapters.set(n.id, n);
  }
  const groups = new Map<string | null, ChapterGroup>();
  for (const scene of scenes) {
    const chId    = scene.parent_id;
    const chapter = chId ? chapters.get(chId) : undefined;
    if (!groups.has(chId)) {
      groups.set(chId, {
        chapterId:    chId,
        chapterTitle: chapter?.title || (chId ? "Untitled chapter" : "Orphan scenes"),
        scenes:       [],
      });
    }
    groups.get(chId)!.scenes.push(scene);
  }
  // Sort groups by their chapter's position (orphans last).
  const list = Array.from(groups.values());
  list.sort((a, b) => {
    const aCh = a.chapterId ? chapters.get(a.chapterId)?.position ?? "zzz" : "zzz";
    const bCh = b.chapterId ? chapters.get(b.chapterId)?.position ?? "zzz" : "zzz";
    return aCh.localeCompare(bCh);
  });
  return list;
}

// ── SceneCard ───────────────────────────────────────────────────────

function SceneCard({
  scene, selected, onSelect, onUpdate,
}: {
  scene:    NodeInfo;
  selected: boolean;
  onSelect: () => void;
  onUpdate: (patch: OutlineUpdate) => void;
}) {
  // Track local copies so the writer can edit without re-render
  // ping-pong; we push the change up only on blur.
  const [title,       setTitle]       = useState<string>(scene.title);
  const [status,      setStatus]      = useState<string>(scene.status);
  const [pov,         setPov]         = useState<string>(scene.pov ?? "");
  const [beat,        setBeat]        = useState<string>(scene.beat ?? "");
  const [targetWords, setTargetWords] = useState<string>(
    scene.target_words != null ? String(scene.target_words) : ""
  );

  function commitTitle() {
    const next = title.trim();
    if (next.length > 0 && next !== scene.title) onUpdate({ title: next });
  }
  function commitStatus(next: string) {
    setStatus(next);
    if (next !== scene.status) onUpdate({ status: next });
  }
  function commitPov() {
    const next = pov.trim();
    if (next !== (scene.pov ?? "")) onUpdate({ pov: next.length === 0 ? null : next });
  }
  function commitBeat() {
    const next = beat.trim();
    if (next !== (scene.beat ?? "")) onUpdate({ beat: next.length === 0 ? null : next });
  }
  function commitTargetWords() {
    const raw = targetWords.trim();
    if (raw === "") {
      if (scene.target_words != null) onUpdate({ target_words: null });
      return;
    }
    const n = Number(raw);
    if (!Number.isFinite(n) || n < 0) {
      // Reset the local input — invalid value.
      setTargetWords(scene.target_words != null ? String(scene.target_words) : "");
      return;
    }
    const rounded = Math.round(n);
    if (rounded !== scene.target_words) onUpdate({ target_words: rounded });
  }

  return (
    <li style={{ ...s.card, ...(selected ? s.cardSelected : {}) }}>
      <header style={s.cardHeader}>
        <span style={statusDotStyle(scene.status)} aria-hidden="true" />
        <button
          style={s.cardTitleBtn}
          onClick={onSelect}
          title="Open this scene in the editor"
        >
          <input
            style={s.cardTitleInput}
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            onBlur={commitTitle}
            onKeyDown={(e) => {
              if (e.key === "Enter")  { e.preventDefault(); (e.target as HTMLInputElement).blur(); }
              if (e.key === "Escape") { setTitle(scene.title); (e.target as HTMLInputElement).blur(); }
            }}
            onClick={(e) => e.stopPropagation()}
          />
        </button>
      </header>
      <div style={s.cardMeta}>
        <label style={s.metaRow}>
          <span style={s.metaLabel}>Status</span>
          <select
            style={s.metaInput}
            value={status}
            onChange={(e) => commitStatus(e.target.value)}
          >
            {STATUS_VALUES.map((v) => (
              <option key={v} value={v}>{v}</option>
            ))}
          </select>
        </label>
        <label style={s.metaRow}>
          <span style={s.metaLabel}>POV</span>
          <input
            style={s.metaInput}
            value={pov}
            onChange={(e) => setPov(e.target.value)}
            onBlur={commitPov}
            placeholder="—"
          />
        </label>
        <label style={s.metaRow}>
          <span style={s.metaLabel}>Beat</span>
          <input
            style={s.metaInput}
            value={beat}
            onChange={(e) => setBeat(e.target.value)}
            onBlur={commitBeat}
            placeholder="—"
          />
        </label>
        <label style={s.metaRow}>
          <span style={s.metaLabel}>Target</span>
          <input
            style={s.metaInput}
            type="number"
            min={0}
            value={targetWords}
            onChange={(e) => setTargetWords(e.target.value)}
            onBlur={commitTargetWords}
            placeholder="—"
          />
        </label>
        <div style={s.wordCountRow}>
          <span style={s.metaLabel}>Words</span>
          <span style={s.wordCountValue}>
            {scene.word_count.toLocaleString()}
            {scene.target_words ? (
              <span style={s.wordCountTarget}>
                {" "}/ {scene.target_words.toLocaleString()}
              </span>
            ) : null}
          </span>
        </div>
      </div>
    </li>
  );
}

function statusDotStyle(status: string): CSSProperties {
  const colour =
    status === "final"    ? "var(--color-green-500, #22c55e)" :
    status === "revised"  ? "var(--color-amber-500, #f59e0b)" :
    status === "drafting" ? "var(--color-amber-400, #fbbf24)" :
    /* planned */           "var(--color-neutral-300)";
  return {
    width: 6, height: 6, borderRadius: "50%",
    background: colour, flexShrink: 0,
  };
}

const s: Record<string, CSSProperties> = {
  root: {
    // Width + border owned by the parent `.leftPane` wrapper in
    // Manuscript so the tab bar above can sit at the pane edge.
    flex: 1,
    minHeight: 0,
    padding: "8px 0 16px",
    background: "var(--color-neutral-50)",
    display: "flex", flexDirection: "column", gap: 8,
    fontFamily: "var(--font-ui)",
    overflowY: "auto",
  },
  heading: {
    fontSize: 11, fontWeight: 600,
    letterSpacing: "0.08em", textTransform: "uppercase",
    color: "var(--color-neutral-500)",
    margin: "0 0 4px 16px",
  },
  body: {
    display: "flex", flexDirection: "column", gap: 12,
    padding: "0 8px 16px",
  },
  empty: {
    margin: "4px 16px 0",
    fontSize: 12, color: "var(--color-neutral-500)",
    lineHeight: 1.55,
  },
  group: {
    display: "flex", flexDirection: "column", gap: 6,
  },
  groupHeader: {
    display: "flex", justifyContent: "space-between", alignItems: "baseline",
    padding: "4px 6px",
  },
  groupTitle: {
    fontFamily: "var(--font-prose, serif)",
    fontSize: 13, fontWeight: 600,
    color: "var(--color-neutral-900)",
  },
  groupCount: {
    fontFamily: "var(--font-mono)",
    fontSize: 10, color: "var(--color-neutral-500)",
  },
  cardList: {
    listStyle: "none", margin: 0, padding: 0,
    display: "flex", flexDirection: "column", gap: 6,
  },
  card: {
    padding: "6px 8px 8px",
    background: "#fff",
    border: "1px solid var(--color-neutral-200)",
    borderRadius: 5,
    display: "flex", flexDirection: "column", gap: 6,
  },
  cardSelected: {
    borderColor: "var(--color-amber-500, #f59e0b)",
    background: "var(--color-amber-50, #fffbeb)",
  },
  cardHeader: {
    display: "flex", alignItems: "center", gap: 6,
  },
  cardTitleBtn: {
    flex: 1,
    background: "transparent",
    border: "none",
    padding: 0,
    cursor: "pointer",
    textAlign: "left",
  },
  cardTitleInput: {
    width: "100%",
    background: "transparent",
    border: "none",
    padding: "2px 0",
    fontFamily: "var(--font-ui)",
    fontSize: 13, fontWeight: 600,
    color: "var(--color-neutral-900)",
    outline: "none",
  },
  cardMeta: {
    display: "grid",
    gridTemplateColumns: "auto 1fr",
    columnGap: 8, rowGap: 3,
    fontSize: 11,
  },
  metaRow: {
    display: "contents",
  },
  metaLabel: {
    fontSize: 10, fontWeight: 600,
    color: "var(--color-neutral-500)",
    textTransform: "uppercase", letterSpacing: "0.04em",
    alignSelf: "center",
  },
  metaInput: {
    width: "100%",
    boxSizing: "border-box",
    padding: "2px 6px",
    border: "1px solid transparent",
    borderRadius: 3,
    background: "transparent",
    color: "var(--color-neutral-900)",
    fontFamily: "var(--font-ui)",
    fontSize: 11,
    outline: "none",
  },
  wordCountRow: {
    display: "contents",
  },
  wordCountValue: {
    fontFamily: "var(--font-mono)",
    fontSize: 11,
    color: "var(--color-neutral-700)",
    fontVariantNumeric: "tabular-nums",
    padding: "2px 6px",
  },
  wordCountTarget: {
    color: "var(--color-neutral-500)",
  },
};
