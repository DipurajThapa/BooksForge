/**
 * Plain-English glossary for the publishing / typography / agent jargon
 * the UI inevitably leaks to the writer.  Phase 8 of `PRODUCT_ROADMAP_E2E.md`
 * (closes UX recommendation R5).
 *
 * The `<Term>` component (see `components/Term.tsx`) renders a glossary
 * key as a dotted-underlined span with an accessible tooltip showing
 * the `short` definition, plus a longer `long` paragraph available via
 * the `<HelpDrawer>` glossary section.
 *
 * Add entries here, not in the markup — the index becomes the
 * single source of truth for the in-app help drawer's glossary tab.
 */

export interface GlossaryEntry {
  /** Display label (capitalised as the writer sees it). */
  label: string;
  /** ≤ 120 chars — fits inside a tooltip without scrolling. */
  short: string;
  /** Optional longer explanation rendered in the help drawer. */
  long?: string;
  /** Optional retailer / spec link the writer can chase down. */
  link?:  { href: string; label: string };
}

export const GLOSSARY: Record<string, GlossaryEntry> = {
  // ── Publishing platforms ──────────────────────────────────────────────────
  KDP: {
    label: "KDP",
    short: "Amazon Kindle Direct Publishing — Amazon's self-serve book-publishing portal.",
    long:  "KDP handles both Kindle eBooks and print-on-demand paperbacks/hardcovers. Free to use; Amazon takes a royalty cut.",
    link:  { href: "https://kdp.amazon.com", label: "kdp.amazon.com" },
  },
  google_play: {
    label: "Google Play Books",
    short: "Google's eBook + audiobook marketplace; reaches Android users worldwide.",
    long:  "Accepts EPUB and PDF. You pick how much of the book Google can show as a free preview (often 10–20%).",
  },
  apple_books: {
    label: "Apple Books",
    short: "Apple's eBook store; gates uploads on EPUBCheck PASS and requires age-range / category metadata.",
  },

  // ── File formats + validation ─────────────────────────────────────────────
  EPUB: {
    label: "EPUB-3",
    short: "Open eBook file format. Supported by every major reader except Kindle (which converts EPUB on upload).",
    long:  "EPUB-3 is the modern eBook standard. BooksForge produces EPUB-3 with embedded fonts and validates against EPUBCheck.",
  },
  EPUBCheck: {
    label: "EPUBCheck",
    short: "Official EPUB validator. Apple Books rejects files with EPUBCheck errors.",
    long:  "We bundle EPUBCheck 5.x. PASS = no errors, WARN = some warnings (usually safe to publish), FAIL = will be rejected.",
    link:  { href: "https://www.w3.org/publishing/epubcheck/", label: "EPUBCheck home" },
  },
  DOCX: {
    label: "DOCX",
    short: "Microsoft Word format — handy if you want to mark up the manuscript with an editor before publishing.",
  },
  PDF: {
    label: "PDF",
    short: "Print-ready file. KDP uses it for the paperback interior.",
  },

  // ── Print typography + design ─────────────────────────────────────────────
  trim: {
    label: "Trim size",
    short: "The physical dimensions of the book (e.g. 6×9 inches).",
    long:  "Trim affects how the interior typesets. 6×9 is the most common for trade paperbacks; 5×8 is common for novels; 8.5×11 for workbooks.",
  },
  bleed: {
    label: "Bleed",
    short: "Extra margin (≈ 0.125 in) on the cover where ink runs off the page edge.",
    long:  "Print covers need bleed so the cover trim doesn't show white edges. The interior almost never needs bleed.",
  },
  spine: {
    label: "Spine",
    short: "The bound edge of the book where the title runs vertically. Eligible at ≥ 100 pages.",
  },
  BISAC: {
    label: "BISAC code",
    short: "Industry subject classifier (e.g. FIC009000 for Fantasy / General).",
    long:  "BISAC codes drive what shelves your book lands on at retailers. Most books use 1–3 codes.",
    link:  { href: "https://www.bisg.org/complete-bisac-subject-headings-list", label: "Complete BISAC list" },
  },
  ISBN: {
    label: "ISBN",
    short: "13-digit book identifier. KDP can mint a free Amazon-only ISBN at upload.",
    long:  "Buy your own ISBN if you plan to distribute outside Amazon (Bowker in the US; free in Canada). Amazon ISBNs lock you to Amazon.",
  },
  ONIX: {
    label: "ONIX",
    short: "XML metadata standard publishers use to send book info to retailers. Used internally by Apple Books.",
  },

  // ── Manuscript / authoring concepts ───────────────────────────────────────
  manuscript: {
    label: "Manuscript",
    short: "The full text of your book — every chapter, every scene.",
  },
  chapter: {
    label: "Chapter",
    short: "A top-level section of the book, usually 2 000–6 000 words.",
  },
  scene: {
    label: "Scene",
    short: "A self-contained moment within a chapter. BooksForge edits and validates one scene at a time.",
  },
  outline: {
    label: "Outline",
    short: "The book's structural plan — themes, beats, chapter list — produced before drafting.",
  },
  pre_edit_snapshot: {
    label: "Pre-edit snapshot",
    short: "A frozen copy of the book taken before any AI agent makes a change. Lets you roll back instantly.",
    long:  "Snapshots are mandatory before any agent applies an edit, so nothing the AI does is destructive — you can always restore the prior version.",
  },

  // ── Agent / quality concepts ──────────────────────────────────────────────
  voice_fingerprint: {
    label: "Voice fingerprint",
    short: "A 16-number profile of your writing style (sentence length, vocabulary, punctuation rhythm, …).",
    long:  "Used by the polish agents so AI edits stay close to your voice. The closer the stylometric distance to your anchor, the safer the edit.",
  },
  stylometric_distance: {
    label: "Stylometric distance",
    short: "How far an edited passage drifts from your voice fingerprint. Lower is better.",
  },
  AI_tells: {
    label: "AI tells",
    short: "Phrases LLMs over-use (e.g. 'navigate the complexities of', 'tapestry of'). The tells scanner flags them.",
  },
  rubric: {
    label: "Rubric",
    short: "12-axis quality scoring (voice, dialogue, tension, …). Weights vary by book kind.",
  },
  genre_pack: {
    label: "Genre pack",
    short: "Preset of system prompts, polish ordering, and rubric weights tuned for one book kind (literary, genre, non-fiction, memoir).",
  },
  HUMAN_REQUIRED: {
    label: "Human-required",
    short: "Step BooksForge cannot do for you (e.g. commission cover art, accept a publisher's terms, disclose AI use).",
  },

  // ── Workflow concepts ─────────────────────────────────────────────────────
  approval_gate: {
    label: "Approval gate",
    short: "A check-in where the workflow pauses for you to review the AI's output before continuing.",
    long:  "Gates exist at four points: topic confirmation, outline approval, character/world bibles, and pre-final-polish.",
  },
  Ollama: {
    label: "Ollama",
    short: "Local LLM runtime BooksForge talks to over 127.0.0.1:11434. Nothing leaves your machine.",
    link:  { href: "https://ollama.com", label: "ollama.com" },
  },
  pm_doc: {
    label: "Scene document",
    short: "The internal JSON representation of one scene's prose (used by the editor, never shown to readers).",
  },
};

export type GlossaryKey = keyof typeof GLOSSARY;

/** Returns true if the given key has a glossary entry. */
export function isGlossaryKey(key: string): key is GlossaryKey {
  return Object.prototype.hasOwnProperty.call(GLOSSARY, key);
}
