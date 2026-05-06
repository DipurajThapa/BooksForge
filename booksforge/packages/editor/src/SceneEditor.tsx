import React, { useCallback, useEffect, useRef } from "react";
import { useEditor, EditorContent } from "@tiptap/react";
import type { JSONContent } from "@tiptap/core";
import { editorExtensions } from "./extensions";
import { countChars, countWords } from "./wordcount";

export interface SceneEditorProps {
  /** ProseMirror JSON document loaded from storage.  Pass `null` for a new scene. */
  initialDoc: JSONContent | null;
  /** Called after every debounced change with the new pm_doc JSON + word/char counts. */
  onSave: (doc: JSONContent, wordCount: number, charCount: number) => void;
  /** Debounce delay in milliseconds (default: 5000). */
  saveDelay?: number;
  /** Whether the editor is read-only. */
  readOnly?: boolean;
}

const EMPTY_DOC: JSONContent = { type: "doc", content: [{ type: "paragraph" }] };

export function SceneEditor({
  initialDoc,
  onSave,
  saveDelay = 5000,
  readOnly = false,
}: SceneEditorProps) {
  const saveTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const onSaveRef = useRef(onSave);
  onSaveRef.current = onSave;

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

  // Flush on unmount.
  useEffect(() => {
    return () => {
      if (saveTimer.current) clearTimeout(saveTimer.current);
    };
  }, []);

  return (
    <div style={s.root}>
      <EditorContent editor={editor} style={s.content} />
    </div>
  );
}

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
