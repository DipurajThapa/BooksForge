/**
 * Dialog/modal a11y hook (BACKLOG §I1).
 *
 * Drop-in keyboard + screen-reader plumbing for any of the
 * `position:fixed` overlay panels in the app:
 *
 *   - **ESC closes** the dialog.
 *   - **Focus is captured on mount** so screen readers announce the
 *     dialog title and keyboard users can navigate immediately
 *     without first tabbing through the underlying app.
 *   - **Focus is restored on unmount** so the user lands back on the
 *     control they came from (e.g. the toolbar button that opened
 *     the panel).
 *
 * Returns the props to spread on the dialog root element + the
 * generated `aria-labelledby` id you should set on the dialog title
 * heading.
 *
 * ## Usage
 *
 * ```tsx
 * const { dialogProps, titleId } = useDialogA11y(onClose);
 * return (
 *   <div role="presentation" style={s.overlay}>
 *     <div {...dialogProps} style={s.dialog}>
 *       <header><strong id={titleId}>Export</strong></header>
 *       …
 *     </div>
 *   </div>
 * );
 * ```
 *
 * Focus trap (cycling Tab through the dialog only) is intentionally
 * *not* in this hook — most browsers and screen readers already
 * implement reasonable defaults for `aria-modal="true"`, and adding
 * a custom trap risks fighting native AT behaviour.  If a specific
 * panel needs a hard trap, layer it on top.
 */
import { useEffect, useId, useRef } from "react";

interface DialogA11yProps {
  role:           "dialog";
  "aria-modal":   "true";
  "aria-labelledby": string;
  tabIndex:       -1;
  ref:            React.RefObject<HTMLDivElement>;
  onKeyDown:      (e: React.KeyboardEvent) => void;
}

export function useDialogA11y(onClose: () => void): {
  dialogProps: DialogA11yProps;
  titleId:     string;
} {
  const ref       = useRef<HTMLDivElement>(null);
  const titleId   = useId();
  const opener    = useRef<HTMLElement | null>(null);

  useEffect(() => {
    // Remember the element that had focus when the dialog opened so
    // we can return focus there when it closes.
    opener.current = (document.activeElement as HTMLElement | null) ?? null;
    // Pull focus into the dialog so AT users hear its title and
    // keyboard users can Tab within it immediately.
    ref.current?.focus();
    return () => {
      // Best-effort focus return.  Wrap in a try/catch because
      // `opener` may have been removed from the DOM in the meantime
      // (e.g. the parent panel itself was unmounted).
      try { opener.current?.focus(); } catch { /* noop */ }
    };
  }, []);

  const onKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Escape") {
      e.stopPropagation();
      onClose();
    }
  };

  return {
    dialogProps: {
      role:              "dialog",
      "aria-modal":      "true",
      "aria-labelledby": titleId,
      tabIndex:          -1,
      ref,
      onKeyDown,
    },
    titleId,
  };
}
