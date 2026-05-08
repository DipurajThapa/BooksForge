import React, { useCallback, useEffect, useImperativeHandle, useRef, forwardRef } from "react";
import { useEditor, EditorContent, type Editor } from "@tiptap/react";
import type { JSONContent } from "@tiptap/core";
import { editorExtensions } from "./extensions";
import { countChars, countWords } from "./wordcount";

/**
 * Imperative handle exposed to wrapper components.  MZ-08 quick-actions use
 * `getSelectionText` to read the highlighted passage (or the current
 * paragraph when nothing is highlighted) and `replaceSelection` to swap in
 * an accepted suggestion without round-tripping through `initialDoc`.
 */
export interface SceneEditorHandle {
  /** Returns highlighted text, or the surrounding paragraph if no selection. */
  getSelectionText(): string;
  /** Returns the full plaintext of the document. */
  getPlainText(): string;
  /** Returns the underlying TipTap editor (for advanced use). */
  getEditor(): Editor | null;
}

export interface SceneEditorProps {
  /** ProseMirror JSON document loaded from storage.  Pass `null` for a new scene. */
  initialDoc: JSONContent | null;
  /** Called after every debounced change with the new pm_doc JSON + word/char counts. */
  onSave: (doc: JSONContent, wordCount: number, charCount: number) => void;
  /** Debounce delay in milliseconds (default: 5000). */
  saveDelay?: number;
  /** Whether the editor is read-only. */
  readOnly?: boolean;
  /** Fired once when the underlying TipTap editor instance is ready, and
   *  again with `null` when it is being torn down.  Lets the parent
   *  surface a formatting toolbar without owning the editor. */
  onEditorReady?: (editor: import("@tiptap/react").Editor | null) => void;
}

const EMPTY_DOC: JSONContent = { type: "doc", content: [{ type: "paragraph" }] };

export const SceneEditor = forwardRef<SceneEditorHandle, SceneEditorProps>(function SceneEditor({
  initialDoc,
  onSave,
  saveDelay = 5000,
  readOnly = false,
  onEditorReady,
}, ref) {
  const saveTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const onSaveRef = useRef(onSave);
  onSaveRef.current = onSave;
  const onReadyRef = useRef(onEditorReady);
  onReadyRef.current = onEditorReady;

  const editor = useEditor({
    extensions: editorExtensions,
    content: initialDoc ?? EMPTY_DOC,
    editable: !readOnly,
    onUpdate({ editor }) {
      // Debounce autosave.
      if (saveTimer.current) clearTimeout(saveTimer.current);
      saveTimer.current = setTimeout(() => {
        const doc = editor.getJSON();
        const text = editor.getText();
        onSaveRef.current(doc, countWords(text), countChars(text));
      }, saveDelay);
    },
    onBlur({ editor }) {
      // Flush immediately on blur.
      if (saveTimer.current) {
        clearTimeout(saveTimer.current);
        saveTimer.current = null;
      }
      const doc = editor.getJSON();
      const text = editor.getText();
      onSaveRef.current(doc, countWords(text), countChars(text));
    },
  });

  // Update content when the node selection changes (switching scenes).
  useEffect(() => {
    if (!editor) return;
    const nextDoc = initialDoc ?? EMPTY_DOC;
    // Only update if the content actually changed (avoid caret reset).
    const current = JSON.stringify(editor.getJSON());
    if (JSON.stringify(nextDoc) !== current) {
      editor.commands.setContent(nextDoc, false);
    }
  }, [editor, initialDoc]);

  // Flush any pending autosave on unmount (e.g. project close, scene switch).
  const editorRef = useRef(editor);
  editorRef.current = editor;

  // Notify the parent once the editor instance is ready (and again with
  // `null` on teardown) so it can render a formatting toolbar.
  useEffect(() => {
    onReadyRef.current?.(editor ?? null);
    return () => {
      onReadyRef.current?.(null);
    };
  }, [editor]);

  // Imperative handle: lets the wrapper read the current selection without
  // owning the TipTap editor instance directly.
  useImperativeHandle(ref, () => ({
    getSelectionText() {
      const ed = editorRef.current;
      if (!ed) return "";
      const { from, to, $from } = ed.state.selection;
      if (from !== to) {
        return ed.state.doc.textBetween(from, to, "\n");
      }
      // No selection — fall back to the surrounding paragraph.
      const start = $from.before($from.depth);
      const end   = $from.after($from.depth);
      return ed.state.doc.textBetween(start, end, "\n").trim();
    },
    getPlainText() {
      return editorRef.current?.getText() ?? "";
    },
    getEditor() {
      return editorRef.current;
    },
  }), []);

  useEffect(() => {
    return () => {
      if (saveTimer.current) {
        clearTimeout(saveTimer.current);
        saveTimer.current = null;
        const ed = editorRef.current;
        if (ed) {
          const doc = ed.getJSON();
          const text = ed.getText();
          onSaveRef.current(doc, countWords(text), countChars(text));
        }
      }
    };
  }, []);

  return (
    <div style={s.root}>
      <EditorContent editor={editor} style={s.content} />
    </div>
  );
});

const s: Record<string, React.CSSProperties> = {
  root: {
    flex: 1,
    display: "flex",
    flexDirection: "column",
    overflow: "auto",
  },
  content: {
    flex: 1,
    padding: "var(--space-8) var(--space-10)",
    maxWidth: 720,
    margin: "0 auto",
    width: "100%",
    fontFamily: "var(--font-prose)",
    fontSize: 17,
    lineHeight: 1.75,
    color: "var(--color-text-primary)",
  },
};
