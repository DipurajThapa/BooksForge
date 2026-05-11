/**
 * <Term> — inline tooltip for jargon (KDP / EPUB / BISAC / trim / bleed / …).
 *
 * Phase 8 of `PRODUCT_ROADMAP_E2E.md` (closes UX recommendation R5).
 *
 * Usage:
 *
 *   <Term k="BISAC" />              // renders the canonical label
 *   <Term k="EPUB">EPUB-3</Term>    // overrides the visible text
 *   <Term k="KDP" inline />         // inline span (default behaviour anyway)
 *
 * The visible text gets a dotted underline; on hover/focus a small
 * tooltip appears with the plain-English `short` description.
 * Definitions live in `lib/glossary.ts` so we never duplicate copy
 * between tooltips and the in-app help drawer.
 *
 * Accessibility: native `title` attribute (screen-reader-friendly) plus
 * an `aria-describedby` link to a hidden description block.
 */
import React, { useId } from "react";
import { GLOSSARY, isGlossaryKey } from "../lib/glossary";

interface Props {
  /** Glossary key (e.g. "KDP", "EPUB", "BISAC", "trim"). */
  k:        string;
  /** Optional override of the visible text. Defaults to entry.label. */
  children?: React.ReactNode;
  /** Render in-flow (default) or as a separate block. */
  inline?:   boolean;
  /** Emphasis style — defaults to dotted underline. */
  variant?:  "underline" | "muted";
}

export default function Term({ k, children, inline = true, variant = "underline" }: Props) {
  const descId = useId();

  if (!isGlossaryKey(k)) {
    // Fail safely: unknown key just renders the raw children / key.
    return <span>{children ?? k}</span>;
  }
  const entry = GLOSSARY[k];
  const label = children ?? entry.label;

  const baseStyle: React.CSSProperties = {
    cursor: "help",
    color:  "inherit",
  };
  const styleByVariant: React.CSSProperties =
    variant === "underline"
      ? { borderBottom: "1px dotted currentColor", textDecoration: "none" }
      : { opacity: 0.85 };

  const Tag = inline ? "span" : "div";

  return (
    <>
      <Tag
        title={entry.short}
        aria-describedby={descId}
        style={{ ...baseStyle, ...styleByVariant }}
      >
        {label}
      </Tag>
      <span id={descId} style={srOnly}>{entry.short}</span>
    </>
  );
}

const srOnly: React.CSSProperties = {
  position:    "absolute",
  width:       1,
  height:      1,
  padding:     0,
  margin:      -1,
  overflow:    "hidden",
  clip:        "rect(0, 0, 0, 0)",
  whiteSpace:  "nowrap",
  border:      0,
};
