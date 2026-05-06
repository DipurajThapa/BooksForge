// MVP TipTap extension set — single source of truth for the ProseMirror schema.
// Any editor instance (editor view, preview) must use this array to guarantee
// pm_doc JSON round-trips correctly.

import { StarterKit } from "@tiptap/starter-kit";
import { Underline } from "@tiptap/extension-underline";
import { Link } from "@tiptap/extension-link";
import { Image } from "@tiptap/extension-image";
import { Placeholder } from "@tiptap/extension-placeholder";
import { CharacterCount } from "@tiptap/extension-character-count";

export const editorExtensions = [
  StarterKit.configure({
    // StarterKit includes: Document, Paragraph, Text, Heading (h1–h6),
    // Bold, Italic, Strike, Code, CodeBlock, Blockquote,
    // BulletList, OrderedList, ListItem, HardBreak, HorizontalRule.
    heading: { levels: [1, 2, 3, 4] },
    // Disable Strike and Code in the toolbar for now (still in schema).
  }),
  Underline,
  Link.configure({
    openOnClick: false,
    HTMLAttributes: { rel: "noopener noreferrer", target: null },
  }),
  Image.configure({ inline: false, allowBase64: false }),
  Placeholder.configure({
    placeholder: "Start writing your scene…",
  }),
  CharacterCount,
];
