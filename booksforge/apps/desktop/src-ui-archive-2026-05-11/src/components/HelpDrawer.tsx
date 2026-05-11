/**
 * In-app help drawer (BACKLOG §I4) — fully offline content.
 *
 * Three tabs:
 *   - Quickstart — what BooksForge is, how to start writing.
 *   - Shortcuts — keyboard map.
 *   - Agents    — what each of the 11 agents does + when to invoke it.
 *
 * No remote fetches; all copy is bundled with the app.
 */
import React, { useState } from "react";
import { useDialogA11y } from "../lib/useDialogA11y";
import { GLOSSARY } from "../lib/glossary";

interface Props { onClose: () => void; }

type Tab = "quickstart" | "shortcuts" | "agents" | "glossary";

export default function HelpDrawer({ onClose }: Props) {
  const { dialogProps, titleId } = useDialogA11y(onClose);
  const [tab, setTab] = useState<Tab>("quickstart");

  return (
    <div style={s.overlay} role="presentation">
      <div {...dialogProps} style={s.dialog}>
        <header style={s.header}>
          <strong id={titleId}>Help</strong>
          <button style={s.close} onClick={onClose} aria-label="Close panel">✕</button>
        </header>

        <div style={s.tabs}>
          <TabBtn active={tab === "quickstart"} onClick={() => setTab("quickstart")}>Quickstart</TabBtn>
          <TabBtn active={tab === "shortcuts"}  onClick={() => setTab("shortcuts")}>Shortcuts</TabBtn>
          <TabBtn active={tab === "agents"}     onClick={() => setTab("agents")}>Agents</TabBtn>
          <TabBtn active={tab === "glossary"}   onClick={() => setTab("glossary")}>Glossary</TabBtn>
        </div>

        <div style={s.body}>
          {tab === "quickstart" && <Quickstart />}
          {tab === "shortcuts"  && <Shortcuts />}
          {tab === "agents"     && <AgentsHelp />}
          {tab === "glossary"   && <Glossary />}
        </div>
      </div>
    </div>
  );
}

function TabBtn({ children, active, onClick }: {
  children: React.ReactNode; active: boolean; onClick: () => void;
}) {
  return (
    <button
      style={{
        ...s.tab,
        borderBottomColor: active ? "var(--color-accent, #2e7d32)" : "transparent",
        fontWeight: active ? 600 : 400,
      }}
      onClick={onClick}
    >
      {children}
    </button>
  );
}

function Quickstart() {
  return (
    <div style={s.section}>
      <h3 style={s.h3}>BooksForge — local-first manuscript workspace</h3>
      <p style={s.p}>
        BooksForge is a desktop app for writing books with strong privacy,
        deep snapshots, and a bounded swarm of local-LLM agents.
        Everything stays on your device by default.
      </p>

      <h4 style={s.h4}>1.  Make a project</h4>
      <p style={s.p}>
        From the project picker, pick a template (fiction novel, non-fiction,
        academic) and a destination folder.  BooksForge creates a
        <code style={s.code}>.booksforge</code> bundle there with a
        manuscript, a SQLite database, and the snapshot store.
      </p>

      <h4 style={s.h4}>2.  Outline + draft</h4>
      <p style={s.p}>
        Use the Binder (left pane) to add Parts → Chapters → Scenes.
        Click a Scene to edit its prose in the centre pane.  Word counts
        update live; status colours show drafting → revising → done.
      </p>

      <h4 style={s.h4}>3.  Use the agents (optional)</h4>
      <p style={s.p}>
        With Ollama running locally on <code style={s.code}>127.0.0.1:11434</code>,
        click <strong>Agents</strong> in the toolbar and pick a workflow.
        Copyedit, Humanize, and Continuity have inline accept buttons.
      </p>

      <h4 style={s.h4}>4.  Snapshot + export</h4>
      <p style={s.p}>
        Click <strong>Snapshots</strong> to take a manual snapshot any time
        (an automatic one fires hourly during active sessions).  Click
        <strong>Export</strong> to ship a Markdown / EPUB / DOCX / PDF.
      </p>

      <h4 style={s.h4}>Privacy</h4>
      <p style={s.p}>
        BooksForge does not contact remote servers by default.  The only
        outbound network calls in MVP are local Ollama traffic
        (<code style={s.code}>127.0.0.1</code>) and one-time Ollama
        installer downloads.  See <strong>Settings → Telemetry</strong> to
        confirm.
      </p>
    </div>
  );
}

function Shortcuts() {
  const groups: Array<{ name: string; rows: Array<[string, string]> }> = [
    {
      name: "Navigation",
      rows: [
        ["⌘ / Ctrl + .", "Toggle distraction-free mode"],
        ["⌘ / Ctrl + F", "Open find / replace"],
        ["⌘ / Ctrl + S", "Save current scene"],
      ],
    },
    {
      name: "Editor (TipTap)",
      rows: [
        ["⌘ / Ctrl + B", "Bold"],
        ["⌘ / Ctrl + I", "Italic"],
        ["⌘ / Ctrl + Z", "Undo"],
        ["⌘ / Ctrl + Shift + Z", "Redo"],
      ],
    },
    {
      name: "Quick actions",
      rows: [
        ["⌘ / Ctrl + Shift + S", "Sharpen prose"],
        ["⌘ / Ctrl + Shift + R", "Rephrase selection"],
        ["⌘ / Ctrl + Shift + K", "Continue paragraph"],
      ],
    },
  ];
  return (
    <div style={s.section}>
      {groups.map(g => (
        <div key={g.name}>
          <h4 style={s.h4}>{g.name}</h4>
          <table style={s.table}>
            <tbody>
              {g.rows.map(([key, action]) => (
                <tr key={key}>
                  <td style={s.kbd}><kbd>{key}</kbd></td>
                  <td style={s.action}>{action}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      ))}
    </div>
  );
}

function AgentsHelp() {
  const agents: Array<[string, string, string]> = [
    ["Outline Architect", "generating", "Brief → full outline with chapters + scenes."],
    ["Chapter Drafter",   "generating", "Synopsis → scene draft in the project's voice."],
    ["Copyeditor",        "prose",      "Mechanical fixes only — punctuation, spacing, casing.  Per-edit accept."],
    ["Humanization",      "prose",      "Detect AI-tells; propose human-sounding rewrites grounded in the project's voice fingerprint."],
    ["Continuity",        "prose",      "Find name / POV / tense / timeline drift.  Apply renames or annotate findings."],
    ["Developmental Editor", "generating", "Per-chapter structural notes (pacing, stakes, arcs)."],
    ["Memory Curator",    "memory",     "Refresh book / chapter / entity memory from accepted prose."],
    ["Vocabulary Dictionary", "memory", "Propose avoid / prefer rules from edit history.  User picks which to promote."],
    ["Intake",            "generating", "Free-text idea → typed ProjectBrief."],
    ["Proposal Validator (Tier 2)", "meta", "LLM-backed validation of another agent's output.  Auto-invoked alongside primary runs."],
    ["Peer Review",       "meta",       "Cross-agent verification on a focus axis (fact fidelity, voice, AI-tells, etc.).  Auto-invoked."],
  ];
  return (
    <div style={s.section}>
      <p style={s.p}>
        Agents run on your local Ollama instance.  No prose ever leaves
        your device.  Each prose-emitting agent's output passes through
        Tier-1 cross-cutting validators (schema, redaction, length,
        originality) before reaching the proposal review surface.
      </p>
      <table style={s.table}>
        <thead>
          <tr>
            <th style={s.th}>Agent</th>
            <th style={s.th}>Category</th>
            <th style={s.th}>What it does</th>
          </tr>
        </thead>
        <tbody>
          {agents.map(([name, cat, blurb]) => (
            <tr key={name}>
              <td style={s.cellName}>{name}</td>
              <td style={s.cellCat}>{cat}</td>
              <td style={s.cellBlurb}>{blurb}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

/**
 * Phase 8 of `PRODUCT_ROADMAP_E2E.md` — plain-English glossary.
 * Source of truth lives in `lib/glossary.ts`; this view just renders it.
 */
function Glossary() {
  const entries = Object.entries(GLOSSARY).sort(([, ea], [, eb]) =>
    ea.label.localeCompare(eb.label)
  );
  return (
    <div style={s.section}>
      <p style={s.p}>
        Quick reference for the publishing / agent jargon that appears in
        BooksForge. Hover over any underlined term in the UI for the same
        definition.
      </p>
      <dl style={glStyles.list}>
        {entries.map(([key, e]) => (
          <div key={key} style={glStyles.row}>
            <dt style={glStyles.term}>
              {e.label}
              {e.link && (
                <a href={e.link.href} target="_blank" rel="noreferrer" style={glStyles.link}>
                  {" "}↗ {e.link.label}
                </a>
              )}
            </dt>
            <dd style={glStyles.def}>
              {e.short}
              {e.long && <div style={glStyles.long}>{e.long}</div>}
            </dd>
          </div>
        ))}
      </dl>
    </div>
  );
}

const glStyles: Record<string, React.CSSProperties> = {
  list: { display: "flex", flexDirection: "column", gap: 10, margin: 0 },
  row:  { borderBottom: "1px solid var(--color-border)", paddingBottom: 8 },
  term: { fontWeight: 600, fontSize: 13, marginBottom: 2 },
  link: { fontSize: 11, opacity: 0.75 },
  def:  { margin: 0, fontSize: 12, opacity: 0.9, lineHeight: 1.5 },
  long: { marginTop: 4, opacity: 0.8 },
};

const s: Record<string, React.CSSProperties> = {
  overlay:  { position: "fixed", inset: 0, background: "rgba(0,0,0,0.4)", zIndex: 50, display: "flex", alignItems: "center", justifyContent: "center" },
  dialog:   { width: "min(720px, 92vw)", maxHeight: "90vh", display: "flex", flexDirection: "column", background: "var(--color-surface)", border: "1px solid var(--color-border)", borderRadius: 6, overflow: "hidden" },
  header:   { display: "flex", alignItems: "center", justifyContent: "space-between", padding: "10px 14px", borderBottom: "1px solid var(--color-border)" },
  close:    { background: "transparent", border: "none", fontSize: 18, cursor: "pointer", color: "inherit" },
  tabs:     { display: "flex", gap: 0, borderBottom: "1px solid var(--color-border)" },
  tab:      { background: "transparent", border: "none", borderBottom: "2px solid transparent", color: "inherit", padding: "8px 14px", cursor: "pointer", fontSize: 13 },
  body:     { padding: "14px 16px", overflowY: "auto", flex: 1 },
  section:  { display: "flex", flexDirection: "column", gap: 12 },
  h3:       { fontSize: 16, fontWeight: 600, margin: 0 },
  h4:       { fontSize: 13, fontWeight: 600, margin: "8px 0 4px 0" },
  p:        { fontSize: 13, margin: 0, lineHeight: 1.5 },
  code:     { fontFamily: "ui-monospace, SFMono-Regular, monospace", fontSize: 12, padding: "0 4px", background: "var(--color-bg)", borderRadius: 3 },
  table:    { width: "100%", borderCollapse: "collapse", fontSize: 12 },
  th:       { textAlign: "left", padding: 6, borderBottom: "1px solid var(--color-border)", fontWeight: 600 },
  kbd:      { padding: 6, fontFamily: "ui-monospace, SFMono-Regular, monospace", whiteSpace: "nowrap" },
  action:   { padding: 6 },
  cellName: { padding: 6, fontWeight: 600 },
  cellCat:  { padding: 6, opacity: 0.7, fontStyle: "italic" },
  cellBlurb:{ padding: 6 },
};
