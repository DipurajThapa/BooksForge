/**
 * Inline formatting toolbar for the SceneEditor.
 *
 * Mirrors the editor's TipTap extension set so writers can reach every
 * supported inline mark and block type without leaving the keyboard.
 * Rendered above the EditorContent — the parent decides when to mount it
 * (typically when a scene is selected).
 *
 * The component subscribes to the editor's transaction stream so the
 * "active" state of each button stays in sync as the caret moves.
 */
import React, { useEffect, useState } from "react";
import type { Editor } from "@tiptap/react";

interface Props {
  editor: Editor | null;
}

interface ButtonSpec {
  label:    string;
  title:    string;
  isActive: (e: Editor) => boolean;
  command:  (e: Editor) => void;
  enabled?: (e: Editor) => boolean;
}

const BUTTONS: ButtonSpec[] = [
  { label: "B",      title: "Bold (⌘B)",       isActive: e => e.isActive("bold"),
    command: e => e.chain().focus().toggleBold().run() },
  { label: "I",      title: "Italic (⌘I)",     isActive: e => e.isActive("italic"),
    command: e => e.chain().focus().toggleItalic().run() },
  { label: "U",      title: "Underline (⌘U)",  isActive: e => e.isActive("underline"),
    command: e => e.chain().focus().toggleUnderline().run() },
  { label: "‹code›", title: "Inline code",     isActive: e => e.isActive("code"),
    command: e => e.chain().focus().toggleCode().run() },
];

const HEADINGS: ButtonSpec[] = [
  { label: "H1", title: "Heading 1",
    isActive: e => e.isActive("heading", { level: 1 }),
    command:  e => e.chain().focus().toggleHeading({ level: 1 }).run() },
  { label: "H2", title: "Heading 2",
    isActive: e => e.isActive("heading", { level: 2 }),
    command:  e => e.chain().focus().toggleHeading({ level: 2 }).run() },
  { label: "H3", title: "Heading 3",
    isActive: e => e.isActive("heading", { level: 3 }),
    command:  e => e.chain().focus().toggleHeading({ level: 3 }).run() },
  { label: "¶",  title: "Paragraph",
    isActive: e => e.isActive("paragraph") && !e.isActive("heading"),
    command:  e => e.chain().focus().setParagraph().run() },
];

const BLOCKS: ButtonSpec[] = [
  { label: "• List",  title: "Bullet list",
    isActive: e => e.isActive("bulletList"),
    command:  e => e.chain().focus().toggleBulletList().run() },
  { label: "1. List", title: "Numbered list",
    isActive: e => e.isActive("orderedList"),
    command:  e => e.chain().focus().toggleOrderedList().run() },
  { label: "❝",       title: "Blockquote",
    isActive: e => e.isActive("blockquote"),
    command:  e => e.chain().focus().toggleBlockquote().run() },
  { label: "⌨",       title: "Code block",
    isActive: e => e.isActive("codeBlock"),
    command:  e => e.chain().focus().toggleCodeBlock().run() },
  { label: "—",       title: "Horizontal rule",
    isActive: () => false,
    command:  e => e.chain().focus().setHorizontalRule().run() },
];

export default function EditorToolbar({ editor }: Props) {
  // Re-render on every transaction so the active highlights stay current.
  const [, setTick] = useState(0);
  useEffect(() => {
    if (!editor) return;
    const onUpdate = () => setTick((n) => n + 1);
    editor.on("transaction", onUpdate);
    editor.on("selectionUpdate", onUpdate);
    return () => {
      editor.off("transaction", onUpdate);
      editor.off("selectionUpdate", onUpdate);
    };
  }, [editor]);

  if (!editor) {
    return <div style={s.bar} aria-hidden />;
  }

  function renderGroup(group: ButtonSpec[], key: string) {
    return (
      <div style={s.group} key={key}>
        {group.map((b) => {
          const active = b.isActive(editor!);
          const enabled = b.enabled ? b.enabled(editor!) : true;
          return (
            <button
              key={b.label}
              type="button"
              style={{
                ...s.btn,
                ...(active ? s.btnActive : null),
                ...(enabled ? null : s.btnDisabled),
              }}
              title={b.title}
              onMouseDown={(ev) => ev.preventDefault()} // keep editor focus
              onClick={() => enabled && b.command(editor!)}
              disabled={!enabled}
            >
              {b.label}
            </button>
          );
        })}
      </div>
    );
  }

  function handleLink() {
    const previous = editor!.getAttributes("link").href as string | undefined;
    const url = window.prompt("Link URL", previous ?? "https://");
    if (url === null) return; // cancelled
    if (url === "") {
      editor!.chain().focus().extendMarkRange("link").unsetLink().run();
      return;
    }
    editor!
      .chain()
      .focus()
      .extendMarkRange("link")
      .setLink({ href: url })
      .run();
  }

  return (
    <div style={s.bar} role="toolbar" aria-label="Formatting">
      {renderGroup(HEADINGS, "headings")}
      {renderGroup(BUTTONS, "marks")}
      {renderGroup(BLOCKS, "blocks")}
      <div style={s.group}>
        <button
          type="button"
          style={{ ...s.btn, ...(editor.isActive("link") ? s.btnActive : null) }}
          title="Link (⌘K not bound — use this button)"
          onMouseDown={(ev) => ev.preventDefault()}
          onClick={handleLink}
        >
          🔗
        </button>
      </div>
      <div style={s.spacer} />
      <div style={s.group}>
        <button
          type="button"
          style={s.btn}
          title="Undo (⌘Z)"
          onMouseDown={(ev) => ev.preventDefault()}
          onClick={() => editor.chain().focus().undo().run()}
          disabled={!editor.can().undo()}
        >
          ↶
        </button>
        <button
          type="button"
          style={s.btn}
          title="Redo (⌘⇧Z)"
          onMouseDown={(ev) => ev.preventDefault()}
          onClick={() => editor.chain().focus().redo().run()}
          disabled={!editor.can().redo()}
        >
          ↷
        </button>
      </div>
    </div>
  );
}

const s: Record<string, React.CSSProperties> = {
  bar: {
    display: "flex",
    alignItems: "center",
    gap: 6,
    padding: "6px 10px",
    borderBottom: "1px solid var(--color-border)",
    background: "var(--color-surface)",
    flexShrink: 0,
    flexWrap: "wrap",
  },
  group: { display: "flex", gap: 2 },
  spacer: { flex: 1 },
  btn: {
    background: "transparent",
    border: "1px solid transparent",
    borderRadius: 4,
    padding: "3px 8px",
    fontSize: 12,
    fontFamily: "var(--font-ui)",
    color: "var(--color-text-secondary)",
    cursor: "pointer",
    minWidth: 26,
    height: 26,
    display: "inline-flex",
    alignItems: "center",
    justifyContent: "center",
  },
  btnActive: {
    background: "var(--color-amber-600, #d97706)",
    borderColor: "var(--color-amber-600, #d97706)",
    color: "#fff",
    fontWeight: 600,
  },
  btnDisabled: {
    opacity: 0.4,
    cursor: "not-allowed",
  },
};
