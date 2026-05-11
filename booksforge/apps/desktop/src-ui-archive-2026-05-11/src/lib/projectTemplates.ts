/**
 * G1 — Project templates.
 *
 * After `project_create` returns the auto-seeded root project node, the
 * wizard can apply one of these templates to seed an opinionated initial
 * tree (chapters / scenes / front+back matter).  Each template is a
 * declarative tree the renderer walks, calling `node_create` per row
 * with a parent reference resolved at apply time.
 *
 * LexoRank positions: we hand out simple `0|i00010:`, `0|i00020:` … strings
 * with a 10-step gap so subsequent UI inserts have room to slot between
 * siblings without renumbering.
 */
import type { NodeInfo } from "@booksforge/shared-types";
import { ipc } from "./ipc";

export type TemplateId = "blank" | "generic-novel" | "romance" | "non-fiction";

export interface TemplateMeta {
  id:          TemplateId;
  label:       string;
  description: string;
}

export const TEMPLATES: TemplateMeta[] = [
  {
    id:          "blank",
    label:       "Blank",
    description: "Just the project shell — start completely from scratch.",
  },
  {
    id:          "generic-novel",
    label:       "Generic Novel",
    description: "10 chapters with 3 scenes each; front- and back-matter stubs.",
  },
  {
    id:          "romance",
    label:       "Romance",
    description: "3-act structure with the standard romance beats pre-named.",
  },
  {
    id:          "non-fiction",
    label:       "Non-Fiction",
    description: "Parts → chapters → sections, with introduction and conclusion.",
  },
];

interface TemplateChild {
  kind:           "part" | "chapter" | "scene" | "front_matter" | "back_matter";
  title:          string;
  status?:        "planned" | "drafting" | "revised" | "final";
  target_words?:  number | null;
  beat?:          string | null;
  children?:      TemplateChild[];
}

/** A template description, applied by walking the children depth-first. */
interface TemplateTree {
  children: TemplateChild[];
}

const GENERIC_NOVEL: TemplateTree = {
  children: [
    {
      kind: "front_matter", title: "Front Matter", children: [
        { kind: "front_matter", title: "Title Page" },
        { kind: "front_matter", title: "Dedication" },
      ],
    },
    ...range(1, 10).map((n): TemplateChild => ({
      kind: "chapter", title: `Chapter ${n}`,
      children: [
        { kind: "scene", title: "Opening",     target_words: 1500, beat: "hook" },
        { kind: "scene", title: "Development", target_words: 2000, beat: "rising" },
        { kind: "scene", title: "Climax",      target_words: 1500, beat: "turn"  },
      ],
    })),
    {
      kind: "back_matter", title: "Back Matter", children: [
        { kind: "back_matter", title: "Epilogue" },
        { kind: "back_matter", title: "Acknowledgements" },
      ],
    },
  ],
};

const ROMANCE: TemplateTree = {
  children: [
    {
      kind: "part", title: "Act I — Setup", children: [
        { kind: "chapter", title: "Meet-Cute", children: [
          { kind: "scene", title: "First Encounter",  target_words: 2000, beat: "meet-cute" },
          { kind: "scene", title: "Spark",            target_words: 1800, beat: "attraction" },
        ]},
        { kind: "chapter", title: "Conflict Established", children: [
          { kind: "scene", title: "Obstacle Revealed", target_words: 1800, beat: "obstacle" },
          { kind: "scene", title: "Reluctant Alliance", target_words: 2200, beat: "alliance" },
        ]},
      ],
    },
    {
      kind: "part", title: "Act II — Falling in Love", children: [
        { kind: "chapter", title: "Forced Proximity", children: [
          { kind: "scene", title: "Proximity",         target_words: 2200, beat: "forced-prox" },
          { kind: "scene", title: "Confession Aside",  target_words: 1500, beat: "internal" },
        ]},
        { kind: "chapter", title: "Vulnerability", children: [
          { kind: "scene", title: "Lowered Guard",     target_words: 1800, beat: "vulnerable" },
          { kind: "scene", title: "First Kiss",        target_words: 1500, beat: "first-kiss" },
        ]},
        { kind: "chapter", title: "Black Moment Setup", children: [
          { kind: "scene", title: "Misunderstanding",  target_words: 2000, beat: "miscommun" },
        ]},
      ],
    },
    {
      kind: "part", title: "Act III — Resolution", children: [
        { kind: "chapter", title: "Black Moment", children: [
          { kind: "scene", title: "All Is Lost",       target_words: 1800, beat: "all-is-lost" },
        ]},
        { kind: "chapter", title: "Grand Gesture", children: [
          { kind: "scene", title: "Realisation",       target_words: 1500, beat: "realisation" },
          { kind: "scene", title: "Grand Gesture",     target_words: 2200, beat: "gesture" },
        ]},
        { kind: "chapter", title: "Happy Ever After", children: [
          { kind: "scene", title: "Reunion",           target_words: 1800, beat: "reunion" },
          { kind: "scene", title: "HEA",               target_words: 1500, beat: "hea"      },
        ]},
      ],
    },
  ],
};

const NON_FICTION: TemplateTree = {
  children: [
    { kind: "front_matter", title: "Introduction", target_words: 2500 },
    {
      kind: "part", title: "Part I — Foundations", children: [
        { kind: "chapter", title: "Chapter 1 — The Problem", children: [
          { kind: "scene", title: "Why this matters",  target_words: 1500 },
          { kind: "scene", title: "What we'll cover",  target_words: 1200 },
        ]},
        { kind: "chapter", title: "Chapter 2 — Background", children: [
          { kind: "scene", title: "History",           target_words: 1800 },
          { kind: "scene", title: "Current state",     target_words: 1800 },
        ]},
      ],
    },
    {
      kind: "part", title: "Part II — Practice", children: [
        { kind: "chapter", title: "Chapter 3 — Approach", children: [
          { kind: "scene", title: "Method",            target_words: 2000 },
          { kind: "scene", title: "Examples",          target_words: 2000 },
        ]},
        { kind: "chapter", title: "Chapter 4 — Pitfalls", children: [
          { kind: "scene", title: "Common mistakes",   target_words: 1800 },
          { kind: "scene", title: "Recovery",          target_words: 1500 },
        ]},
      ],
    },
    { kind: "back_matter", title: "Conclusion", target_words: 2000 },
    { kind: "back_matter", title: "Further Reading" },
  ],
};

const TREES: Record<Exclude<TemplateId, "blank">, TemplateTree> = {
  "generic-novel": GENERIC_NOVEL,
  "romance":       ROMANCE,
  "non-fiction":   NON_FICTION,
};

/**
 * Apply a template by issuing `node_create` IPC calls for every row.
 *
 * Returns the count of nodes created.  The project root must already exist
 * (i.e. `project_create` has been called successfully) and is identified
 * as the only node with `kind === "project"` in `node_list`.
 */
export async function applyTemplate(templateId: TemplateId): Promise<number> {
  if (templateId === "blank") return 0;
  const tree = TREES[templateId];

  const existing = await ipc.nodeList();
  const projectRoot = existing.find((n) => n.kind === "project");
  if (!projectRoot) {
    throw new Error("Cannot apply template: project root node missing.");
  }

  let created = 0;
  await walkAndCreate(tree.children, projectRoot.id);
  return created;

  async function walkAndCreate(children: TemplateChild[], parentId: string): Promise<void> {
    let idx = 1;
    for (const child of children) {
      const node: NodeInfo = await ipc.nodeCreate({
        parent_id:    parentId,
        kind:         child.kind,
        title:        child.title,
        position:     formatRank(idx * 10),
        status:       child.status ?? "planned",
        target_words: child.target_words ?? null,
      });
      created += 1;
      // beat is stored on the node, but `node_create` doesn't currently
      // accept a beat field — falls back to the inspector for editing.
      if (child.children?.length) {
        await walkAndCreate(child.children, node.id);
      }
      idx += 1;
    }
  }
}

function formatRank(n: number): string {
  // LexoRank base-36, 5-digit zero-padded; matches Binder's rank format.
  const padded = n.toString(36).padStart(5, "0");
  return `0|i${padded}:`;
}

function range(lo: number, hi: number): number[] {
  const out: number[] = [];
  for (let i = lo; i <= hi; i += 1) out.push(i);
  return out;
}
