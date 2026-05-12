/**
 * DiffView — word-level inline diff renderer.
 *
 * Wraps `lib/wordDiff` and styles each segment so removed text reads
 * as struck-through red and added text reads as underlined green.
 * Equal segments render plain so the writer can scan the body and
 * see only what changed.
 *
 * Used by the Cmd+K QuickActionBar so the writer sees what the AI
 * actually changed before accepting — spec §5.2: "Suggestions
 * appear in a side panel with a diff view; user accepts/rejects/
 * regenerates."
 */
import type { CSSProperties } from "react";
import { wordDiff, type DiffSegment } from "../lib/wordDiff";

interface Props {
  before: string;
  after:  string;
  /** Tone of the visible "diff body" font. Defaults to the prose serif
   *  so the diff reads like the inserted text will read. */
  fontFamily?: string;
}

export default function DiffView({ before, after, fontFamily }: Props) {
  const segs = wordDiff(before, after);
  if (segs.length === 0) {
    return <p style={s.empty}>No change.</p>;
  }
  return (
    <div style={{ ...s.body, fontFamily: fontFamily ?? "var(--font-prose, serif)" }}>
      {segs.map((seg, i) => (
        <Segment key={i} seg={seg} />
      ))}
    </div>
  );
}

function Segment({ seg }: { seg: DiffSegment }) {
  if (seg.op === "equal") {
    return <span>{seg.text}</span>;
  }
  if (seg.op === "remove") {
    return (
      <span style={s.remove} aria-label="removed">
        {seg.text}
      </span>
    );
  }
  return (
    <span style={s.add} aria-label="added">
      {seg.text}
    </span>
  );
}

const s: Record<string, CSSProperties> = {
  body: {
    fontSize: 14,
    lineHeight: 1.6,
    color: "var(--color-neutral-900)",
    whiteSpace: "pre-wrap",
    wordBreak: "break-word",
  },
  remove: {
    background: "rgba(220,38,38,0.10)",
    color: "var(--color-red-700, #b91c1c)",
    textDecoration: "line-through",
    textDecorationThickness: 1,
    padding: "0 1px",
    borderRadius: 2,
  },
  add: {
    background: "rgba(34,197,94,0.14)",
    color: "var(--color-green-700, #15803d)",
    textDecoration: "underline",
    textDecorationThickness: 1,
    padding: "0 1px",
    borderRadius: 2,
  },
  empty: {
    margin: 0,
    fontSize: 12,
    color: "var(--color-neutral-500)",
    fontStyle: "italic",
  },
};
