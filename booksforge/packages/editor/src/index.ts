// @booksforge/editor — TipTap-based scene editor for BooksForge.
//
// Public surface:
//   SceneEditor — React component (controlled editor with autosave)
//   editorExtensions — TipTap extension array (for schema consistency)
//   countWords — word-count utility used by the editor and IPC layer

export { SceneEditor } from "./SceneEditor";
export type { SceneEditorHandle, SceneEditorProps } from "./SceneEditor";
export { default as EditorToolbar } from "./EditorToolbar";
export { editorExtensions } from "./extensions";
export { countWords } from "./wordcount";
