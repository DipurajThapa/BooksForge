/**
 * New Project Wizard — MVP steps:
 *   Step 1: Project name + author
 *   Step 2: Save location (folder picker)
 *   Step 4: Confirm + create  (Step 3 = AI consent is deferred)
 */
import React, { useState } from "react";
import { save as saveDialog } from "@tauri-apps/plugin-dialog";
import type { OpenProjectResult } from "@booksforge/shared-types";
import { ipc } from "../lib/ipc";

interface Props {
  onCreated: (result: OpenProjectResult) => void;
  onCancel: () => void;
}

type Step = 1 | 2 | 4;

interface FormState {
  title: string;
  author: string;
  bundlePath: string;
}

const EMPTY: FormState = { title: "", author: "", bundlePath: "" };

export default function NewProjectWizard({ onCreated, onCancel }: Props) {
  const [step, setStep] = useState<Step>(1);
  const [form, setForm] = useState<FormState>(EMPTY);
  const [creating, setCreating] = useState(false);
  const [error, setError] = useState<string | null>(null);

  function set<K extends keyof FormState>(key: K, value: FormState[K]) {
    setForm((f) => ({ ...f, [key]: value }));
    setError(null);
  }

  async function pickSaveLocation() {
    const defaultName = form.title
      ? `${form.title.replace(/[^a-zA-Z0-9_\- ]/g, "")}.booksforge`
      : "MyBook.booksforge";
    const selected = await saveDialog({
      title: "Choose project save location",
      defaultPath: defaultName,
    }).catch(() => null);
    if (selected) set("bundlePath", selected);
  }

  async function handleCreate() {
    if (!form.title.trim()) { setError("Title is required."); return; }
    if (!form.author.trim()) { setError("Author is required."); return; }
    if (!form.bundlePath.trim()) { setError("Save location is required."); return; }

    setCreating(true);
    setError(null);
    try {
      const result = await ipc.projectCreate({
        title: form.title.trim(),
        author: form.author.trim(),
        bundle_path: form.bundlePath,
        genre: null,
      });
      onCreated(result);
    } catch (e) {
      setError(String(e));
      setCreating(false);
    }
  }

  return (
    <div style={s.overlay}>
      <div style={s.panel}>
        <header style={s.header}>
          <span style={s.stepLabel}>
            Step {step === 4 ? 3 : step} of 3
          </span>
          <button style={s.closeBtn} onClick={onCancel} disabled={creating}>
            ✕
          </button>
        </header>

        {step === 1 && (
          <Step1
            form={form}
            onChange={set}
            onNext={() => setStep(2)}
            onCancel={onCancel}
          />
        )}
        {step === 2 && (
          <Step2
            bundlePath={form.bundlePath}
            onPick={pickSaveLocation}
            onBack={() => setStep(1)}
            onNext={() => setStep(4)}
          />
        )}
        {step === 4 && (
          <Step4
            form={form}
            creating={creating}
            error={error}
            onBack={() => setStep(2)}
            onCreate={handleCreate}
          />
        )}
      </div>
    </div>
  );
}

// ── Step 1: Name + Author ─────────────────────────────────────────────────────

function Step1({
  form,
  onChange,
  onNext,
  onCancel,
}: {
  form: FormState;
  onChange: <K extends keyof FormState>(k: K, v: FormState[K]) => void;
  onNext: () => void;
  onCancel: () => void;
}) {
  const valid = form.title.trim().length > 0 && form.author.trim().length > 0;
  return (
    <div style={s.body}>
      <h2 style={s.title}>Name your project</h2>
      <label style={s.label}>
        Book title
        <input
          style={s.input}
          value={form.title}
          onChange={(e) => onChange("title", e.target.value)}
          placeholder="e.g. The Midnight Archive"
          autoFocus
        />
      </label>
      <label style={s.label}>
        Author
        <input
          style={s.input}
          value={form.author}
          onChange={(e) => onChange("author", e.target.value)}
          placeholder="e.g. Jane Smith"
        />
      </label>
      <div style={s.footer}>
        <button style={s.ghostBtn} onClick={onCancel}>Cancel</button>
        <button style={s.primaryBtn} onClick={onNext} disabled={!valid}>
          Next
        </button>
      </div>
    </div>
  );
}

// ── Step 2: Save location ─────────────────────────────────────────────────────

function Step2({
  bundlePath,
  onPick,
  onBack,
  onNext,
}: {
  bundlePath: string;
  onPick: () => void;
  onBack: () => void;
  onNext: () => void;
}) {
  return (
    <div style={s.body}>
      <h2 style={s.title}>Choose save location</h2>
      <p style={s.hint}>
        A <code>.booksforge</code> folder will be created at the location you choose.
        You can move it later — BooksForge stores everything inside that folder.
      </p>
      <div style={s.pathRow}>
        <span style={s.pathDisplay}>
          {bundlePath || <span style={{ color: "var(--color-text-tertiary)" }}>No location selected</span>}
        </span>
        <button style={s.secondaryBtn} onClick={onPick}>
          Browse…
        </button>
      </div>
      <div style={s.footer}>
        <button style={s.ghostBtn} onClick={onBack}>Back</button>
        <button style={s.primaryBtn} onClick={onNext} disabled={!bundlePath}>
          Next
        </button>
      </div>
    </div>
  );
}

// ── Step 4: Confirm + Create ──────────────────────────────────────────────────

function Step4({
  form,
  creating,
  error,
  onBack,
  onCreate,
}: {
  form: FormState;
  creating: boolean;
  error: string | null;
  onBack: () => void;
  onCreate: () => void;
}) {
  return (
    <div style={s.body}>
      <h2 style={s.title}>Ready to create</h2>
      <dl style={s.summary}>
        <dt style={s.dt}>Title</dt>
        <dd style={s.dd}>{form.title}</dd>
        <dt style={s.dt}>Author</dt>
        <dd style={s.dd}>{form.author}</dd>
        <dt style={s.dt}>Location</dt>
        <dd style={{ ...s.dd, fontFamily: "var(--font-mono)", fontSize: 11 }}>
          {form.bundlePath}
        </dd>
      </dl>
      {error && <p style={s.error}>{error}</p>}
      <div style={s.footer}>
        <button style={s.ghostBtn} onClick={onBack} disabled={creating}>Back</button>
        <button style={s.primaryBtn} onClick={onCreate} disabled={creating}>
          {creating ? "Creating…" : "Create Project"}
        </button>
      </div>
    </div>
  );
}

// ── Styles ────────────────────────────────────────────────────────────────────

const s: Record<string, React.CSSProperties> = {
  overlay: {
    position: "fixed",
    inset: 0,
    background: "rgba(0,0,0,0.4)",
    display: "flex",
    alignItems: "center",
    justifyContent: "center",
    zIndex: 100,
  },
  panel: {
    width: 520,
    background: "var(--color-surface)",
    borderRadius: 10,
    boxShadow: "0 20px 60px rgba(0,0,0,0.3)",
    display: "flex",
    flexDirection: "column",
    overflow: "hidden",
  },
  header: {
    display: "flex",
    alignItems: "center",
    justifyContent: "space-between",
    padding: "var(--space-4) var(--space-6)",
    borderBottom: "1px solid var(--color-border)",
  },
  stepLabel: {
    fontSize: 12,
    color: "var(--color-text-tertiary)",
    fontWeight: 500,
    letterSpacing: "0.04em",
  },
  closeBtn: {
    background: "none",
    border: "none",
    color: "var(--color-text-tertiary)",
    fontSize: 16,
    cursor: "pointer",
    padding: 4,
    lineHeight: 1,
  },
  body: {
    padding: "var(--space-6)",
    display: "flex",
    flexDirection: "column",
    gap: "var(--space-4)",
  },
  title: {
    margin: 0,
    fontSize: 18,
    fontWeight: 600,
    color: "var(--color-text-primary)",
    fontFamily: "var(--font-prose)",
  },
  hint: {
    margin: 0,
    fontSize: 13,
    color: "var(--color-text-secondary)",
    lineHeight: 1.6,
  },
  label: {
    display: "flex",
    flexDirection: "column",
    gap: "var(--space-1)",
    fontSize: 13,
    fontWeight: 500,
    color: "var(--color-text-secondary)",
  },
  input: {
    padding: "var(--space-2) var(--space-3)",
    border: "1px solid var(--color-border)",
    borderRadius: 5,
    fontSize: 14,
    background: "var(--color-surface)",
    color: "var(--color-text-primary)",
    fontFamily: "var(--font-ui)",
    outline: "none",
  },
  pathRow: {
    display: "flex",
    gap: "var(--space-2)",
    alignItems: "center",
  },
  pathDisplay: {
    flex: 1,
    fontSize: 12,
    fontFamily: "var(--font-mono)",
    color: "var(--color-text-primary)",
    overflow: "hidden",
    textOverflow: "ellipsis",
    whiteSpace: "nowrap",
    padding: "var(--space-2) var(--space-3)",
    border: "1px solid var(--color-border)",
    borderRadius: 5,
    minHeight: 36,
    display: "flex",
    alignItems: "center",
  },
  summary: {
    margin: 0,
    display: "grid",
    gridTemplateColumns: "80px 1fr",
    rowGap: "var(--space-2)",
    columnGap: "var(--space-3)",
    padding: "var(--space-4)",
    background: "var(--color-neutral-50)",
    borderRadius: 6,
    border: "1px solid var(--color-border)",
  },
  dt: {
    fontSize: 12,
    fontWeight: 600,
    color: "var(--color-text-tertiary)",
    textTransform: "uppercase",
    letterSpacing: "0.06em",
    margin: 0,
    alignSelf: "start",
  },
  dd: {
    fontSize: 14,
    color: "var(--color-text-primary)",
    margin: 0,
    wordBreak: "break-all",
  },
  footer: {
    display: "flex",
    justifyContent: "flex-end",
    gap: "var(--space-2)",
    marginTop: "var(--space-2)",
  },
  primaryBtn: {
    padding: "var(--space-2) var(--space-5)",
    background: "var(--color-amber-600)",
    color: "#fff",
    border: "none",
    borderRadius: 5,
    fontSize: 14,
    fontWeight: 600,
    cursor: "pointer",
    fontFamily: "var(--font-ui)",
  },
  secondaryBtn: {
    padding: "var(--space-2) var(--space-4)",
    background: "transparent",
    color: "var(--color-text-primary)",
    border: "1px solid var(--color-border)",
    borderRadius: 5,
    fontSize: 13,
    cursor: "pointer",
    fontFamily: "var(--font-ui)",
  },
  ghostBtn: {
    padding: "var(--space-2) var(--space-4)",
    background: "transparent",
    color: "var(--color-text-secondary)",
    border: "none",
    borderRadius: 5,
    fontSize: 14,
    cursor: "pointer",
    fontFamily: "var(--font-ui)",
  },
  error: {
    color: "var(--color-error)",
    fontFamily: "var(--font-mono)",
    fontSize: 12,
    margin: 0,
  },
};
