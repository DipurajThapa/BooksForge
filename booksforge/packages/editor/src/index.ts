// @booksforge/editor — TipTap-based scene editor for BooksForge.
//
// Public surface:
//   SceneEditor — React component (controlled editor with autosave)
//   editorExtensions — TipTap extension array (for schema consistency)
//   countWords — word-count utility used by the editor and IPC layer
//
// TipTap type re-exports — consumers (src-ui, etc.) should import these from
// here rather than from `@tiptap/core` / `@tiptap/react` directly. Keeps
// the TipTap dep declared in exactly one place (this package).

export { SceneEditor } from "./SceneEditor";
export type { SceneEditorHandle, SceneEditorProps } from "./SceneEditor";
export { default as EditorToolbar } from "./EditorToolbar";
export { editorExtensions } from "./extensions";
export { countWords } from "./wordcount";

export type { JSONContent } from "@tiptap/core";
export type { Editor } from "@tiptap/react";
