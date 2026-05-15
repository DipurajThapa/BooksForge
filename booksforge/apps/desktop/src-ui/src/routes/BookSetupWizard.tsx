/**
 * BookSetupWizard — 4-step wizard for first-run project creation (F2 redesign).
 *
 * Per `book-output/design/WRITER_JOURNEY_REDESIGN_2026-05.md` §4 Stage 1
 * AND the 2026-05-12 UX audit (F2): the wizard captures only the
 * fields the orchestrator needs to *create* a project. Everything
 * else (sub-genre, tone, writing style, audience map, format & print
 * preferences, concept-scorer gate) is editable in the in-editor
 * Stage 1 / Stage 2 panels. That removes ~3 minutes of duplicate
 * data entry on first run.
 *
 *   Step 1: Title + author + save location  (everything mandatory in one screen)
 *   Step 2: Premise + 1–6 key promises     (the spine the orchestrator reads)
 *   Step 3: Primary audience               (one line; refine later)
 *   Step 4: Confirm & create
 *
 * Wire to the backend writing pipeline is unchanged: we still call
 * `projectCreate` then `projectBriefSave` with the same `ProjectBrief`
 * shape — the locked drafter / orchestrator / agents read the same
 * memory key `book:project_brief`. Defaults fill the optional fields
 * so the brief schema matches its existing Rust validator.
 */
import React, { useState } from "react";
import { save as saveDialog } from "@tauri-apps/plugin-dialog";
import type { OpenProjectResult } from "@booksforge/shared-types";
import { ipc } from "../lib/ipc";
import { errorMessage } from "../lib/errorMessage";
import { useToast } from "../components/ToastProvider";

interface Props {
  onCreated: (project: OpenProjectResult) => void;
  onCancel:  () => void;
}

// ── Form state ─────────────────────────────────────────────────────────────

// MVP anchors on literary fiction; the other kinds appear in later
// phases. We keep the constant rather than exposing a chooser because
// the wizard is supposed to be near-zero friction.
const BOOK_KIND = "literary-fiction" as const;

interface WizardForm {
  // Step 1
  title:       string;
  author:      string;
  bundlePath:  string;
  // Step 2
  premise:     string;
  keyPromises: string;  // textarea, one promise per line
  // Step 3
  audience:    string;
}

const EMPTY: WizardForm = {
  title:       "",
  author:      "",
  bundlePath:  "",
  premise:     "",
  keyPromises: "",
  audience:    "adult literary readers",
};

const STEP_COUNT = 4;

// ── Component ──────────────────────────────────────────────────────────────

export default function BookSetupWizard({ onCreated, onCancel }: Props) {
  const [step, setStep] = useState<number>(1);
  const [form, setForm] = useState<WizardForm>(EMPTY);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const toast = useToast();

  function update<K extends keyof WizardForm>(k: K, v: WizardForm[K]) {
    setForm((f) => ({ ...f, [k]: v }));
    setError(null);
  }

  function canAdvance(): { ok: boolean; reason?: string } {
    switch (step) {
      case 1:
        if (!form.title.trim())      return { ok: false, reason: "Title is required." };
        if (!form.author.trim())     return { ok: false, reason: "Author name is required." };
        if (!form.bundlePath.trim()) return { ok: false, reason: "Pick a save location." };
        return { ok: true };
      case 2:
        if (!form.premise.trim()) return { ok: false, reason: "Premise is required." };
        if (form.keyPromises.split("\n").map((l) => l.trim()).filter(Boolean).length === 0) {
          return { ok: false, reason: "Add at least one key promise (one per line)." };
        }
        return { ok: true };
      case 3:
        if (!form.audience.trim()) return { ok: false, reason: "Audience is required (you can refine it later)." };
        return { ok: true };
      case 4:  return { ok: true };
      default: return { ok: true };
    }
  }

  async function pickSaveLocation() {
    const safe = form.title.trim().replace(/[^a-zA-Z0-9_\- ]/g, "") || "MyBook";
    const selected = await saveDialog({
      title: "Choose project save location",
      defaultPath: `${safe}.booksforge`,
    }).catch(() => null);
    if (selected) update("bundlePath", selected);
  }

  async function handleCreate() {
    const guard = canAdvance();
    if (!guard.ok) { setError(guard.reason ?? "Form incomplete."); return; }
    setBusy(true); setError(null);
    try {
      // 1. Create the bundle (manifest + db + project root node).
      const result = await ipc.projectCreate({
        title:       form.title.trim(),
        author:      form.author.trim(),
        bundle_path: form.bundlePath,
        genre:       "literary fiction",
        book_kind:   BOOK_KIND,
      });

      // 2. Persist a minimal-but-valid ProjectBrief to `book:project_brief`.
      //    The Rust validator's constraints (premise non-empty, 1-6
      //    key_promises, 5000-250000 target_word_count) are all met
      //    by these defaults; downstream agents read this verbatim.
      try {
        const briefJson = buildBriefFromForm(form);
        await ipc.projectBriefSave({ brief_json: briefJson });
      } catch (e) {
        // The project itself succeeded. Tell the writer the brief is
        // missing so they don't land in an empty Stage 1 and wonder
        // why. They can re-save from Stage 1 once they're inside.
        toast.push({
          severity: "warning",
          title: "Brief not saved",
          body: `Project created, but the brief save failed: ${errorMessage(e)}. Open Stage 1 to re-enter and save.`,
        });
      }

      toast.push({
        severity: "success",
        title: "Project created",
        body: `${result.title} is ready. The Audience map and Format preferences can be filled in from the editor.`,
      });
      onCreated(result);
    } catch (e) {
      const msg = errorMessage(e);
      setError(msg);
      toast.push({
        severity: "error",
        title: "Could not create project",
        body: msg,
      });
    } finally {
      setBusy(false);
    }
  }

  /**
   * Builds the ProjectBrief payload the orchestrator + downstream
   * agents read from `book:project_brief`. Fields the wizard no
   * longer asks for use sensible defaults so the validator passes
   * and the agent prompts don't see undefined values. The writer
   * fills these in via Stage 1 / Stage 2 — same memory key, same
   * IPC, no backend change.
   */
  function buildBriefFromForm(f: WizardForm): unknown {
    const keyPromises = f.keyPromises
      .split("\n").map((l) => l.trim()).filter((l) => l.length > 0);
    return {
      title_suggestions:      [f.title.trim()],
      mode:                   "fiction",
      genre:                  "literary fiction",
      audience:               f.audience.trim(),
      tone:                   "spare",
      target_word_count:      75000,
      premise:                f.premise.trim(),
      key_promises:           keyPromises,
      questions_for_user:     [],
      comp_titles_or_authors: [],
      theme_keywords:         [],
      forbidden_tropes:       [],
      era_setting:            null,
      cultural_context:       null,
      creative_seed:          null,
    };
  }

  function next() {
    const guard = canAdvance();
    if (!guard.ok) { setError(guard.reason ?? "Form incomplete."); return; }
    setError(null);
    if (step < STEP_COUNT) setStep(step + 1);
  }
  function back() {
    setError(null);
    if (step > 1) setStep(step - 1);
  }

  return (
    <div style={s.shell}>
      <header style={s.header}>
        <span style={s.wordmark}>BooksForge</span>
        <span style={s.crumbs}>
          Step <b>{step}</b> of {STEP_COUNT} — {stepTitle(step)}
        </span>
        <button style={s.closeBtn} onClick={onCancel} disabled={busy}>Cancel</button>
      </header>

      <ProgressBar step={step} total={STEP_COUNT} />

      <div style={s.body}>
        {step === 1 && <Step1Project form={form} update={update} onPick={pickSaveLocation} />}
        {step === 2 && <Step2Concept form={form} update={update} />}
        {step === 3 && <Step3Audience form={form} update={update} />}
        {step === 4 && <Step4Confirm form={form} />}
      </div>

      {error && <div style={s.error}>{error}</div>}

      <footer style={s.footer}>
        <button style={s.ghostBtn} onClick={back} disabled={step === 1 || busy}>← Back</button>
        {step < STEP_COUNT && (
          <button style={s.primaryBtn} onClick={next}>Continue →</button>
        )}
        {step === STEP_COUNT && (
          <button style={s.primaryBtn} onClick={handleCreate} disabled={busy}>
            {busy ? "Creating project…" : "Create project"}
          </button>
        )}
      </footer>
    </div>
  );
}

function stepTitle(n: number): string {
  return [
    "",
    "Project basics",
    "Concept",
    "Audience",
    "Review",
  ][n] ?? "";
}

// ── Step screens ────────────────────────────────────────────────────────────

function Step1Project({ form, update, onPick }: {
  form:   WizardForm;
  update: <K extends keyof WizardForm>(k: K, v: WizardForm[K]) => void;
  onPick: () => Promise<void>;
}) {
  return (
    <div style={s.step}>
      <h2 style={s.h2}>Start a new book</h2>
      <p style={s.lede}>
        Three fields. Title and author land on the cover and metadata;
        the save location is the <code style={s.code}>.booksforge/</code>
        bundle that holds your manuscript, database, snapshots, and
        exports. Pick a folder you back up.
      </p>
      <Field label="Title" required>
        <input style={s.input} value={form.title}
          onChange={(e) => update("title", e.target.value)}
          placeholder="The Incomplete Curse" autoFocus />
      </Field>
      <Field label="Author name" required>
        <input style={s.input} value={form.author}
          onChange={(e) => update("author", e.target.value)} />
      </Field>
      <Field label="Save location" required>
        <div style={s.pathRow}>
          <input
            style={{ ...s.input, fontFamily: "var(--font-mono)", fontSize: 12 }}
            value={form.bundlePath}
            onChange={(e) => update("bundlePath", e.target.value)}
            placeholder="/Users/.../MyBook.booksforge"
          />
          <button style={s.secondaryBtn} onClick={onPick}>Browse…</button>
        </div>
      </Field>
    </div>
  );
}

function Step2Concept({ form, update }: {
  form:   WizardForm;
  update: <K extends keyof WizardForm>(k: K, v: WizardForm[K]) => void;
}) {
  return (
    <div style={s.step}>
      <h2 style={s.h2}>The concept</h2>
      <p style={s.lede}>
        Every downstream agent (outline, bibles, drafter, polish) reads
        these two fields. Genre, tone, word-count target, sub-genre and
        background are editable from Stage 1 in the editor once you're
        inside — you don't need them here.
      </p>
      <Field label="Premise" required hint="1–3 sentences in your own register.">
        <textarea
          style={{ ...s.input, minHeight: 96, fontFamily: "var(--font-prose)" }}
          value={form.premise}
          onChange={(e) => update("premise", e.target.value)}
          placeholder="A clockmaker's widow finds twenty-three sealed letters addressed to a woman she has never heard of." />
      </Field>
      <Field label="Key promises" required hint="1–6 lines, one promise per line.">
        <textarea
          style={{ ...s.input, minHeight: 110, fontFamily: "var(--font-prose)" }}
          value={form.keyPromises}
          onChange={(e) => update("keyPromises", e.target.value)}
          placeholder={
            "Sustained dread anchored in everyday objects\n" +
            "A village whose hierarchy is the real horror\n" +
            "An ending that does not resolve cleanly"
          } />
      </Field>
    </div>
  );
}

function Step3Audience({ form, update }: {
  form:   WizardForm;
  update: <K extends keyof WizardForm>(k: K, v: WizardForm[K]) => void;
}) {
  return (
    <div style={s.step}>
      <h2 style={s.h2}>Who is this book for?</h2>
      <p style={s.lede}>
        A book without a named reader becomes generic. One line is enough
        here. Secondary audience, age range, reader expectations, and
        comparable titles all live in Stage 2 — they're useful but not
        load-bearing until the drafter runs.
      </p>
      <Field label="Primary audience" required>
        <input style={s.input} value={form.audience}
          onChange={(e) => update("audience", e.target.value)} />
      </Field>
    </div>
  );
}

function Step4Confirm({ form }: { form: WizardForm }) {
  const promiseCount = form.keyPromises
    .split("\n").map((l) => l.trim()).filter(Boolean).length;
  return (
    <div style={s.step}>
      <h2 style={s.h2}>Review &amp; create</h2>
      <p style={s.lede}>
        We'll create the <code style={s.code}>.booksforge/</code> bundle
        on disk and write your brief into project memory. After this you
        land in Stage 1 — Book Setup, where you can fine-tune everything.
      </p>
      <dl style={s.review}>
        <dt style={s.dt}>Book</dt>
        <dd style={s.dd}>
          <b>{form.title || <em>(no title)</em>}</b>
          {" "}by <b>{form.author || <em>(no author)</em>}</b>
        </dd>
        <dt style={s.dt}>Saved to</dt>
        <dd style={s.dd} title={form.bundlePath}>
          <code style={s.code}>{form.bundlePath}</code>
        </dd>
        <dt style={s.dt}>Premise</dt>
        <dd style={s.dd}>{form.premise || <em>(none)</em>}</dd>
        <dt style={s.dt}>Promises</dt>
        <dd style={s.dd}>
          {promiseCount} promise{promiseCount === 1 ? "" : "s"}
        </dd>
        <dt style={s.dt}>Audience</dt>
        <dd style={s.dd}>{form.audience || <em>(unset)</em>}</dd>
      </dl>
      <p style={{ ...s.lede, marginTop: 4 }}>
        <b>Next steps after create:</b> tone, sub-genre, target word count,
        audience map, comp titles, format &amp; print preferences, cover,
        and boilerplate are all editable from the editor's stages. The
        drafting pipeline reads the brief at run-time, so any change you
        make before pressing <i>Run pipeline</i> takes effect.
      </p>
    </div>
  );
}

// ── Subcomponents ───────────────────────────────────────────────────────────

function ProgressBar({ step, total }: { step: number; total: number }) {
  return (
    <div style={s.progress}>
      {Array.from({ length: total }).map((_, i) => (
        <span key={i} style={{
          ...s.progressTick,
          background: i < step
            ? "var(--color-amber-500, #f59e0b)"
            : "var(--color-neutral-200)",
        }} />
      ))}
    </div>
  );
}

function Field({ label, hint, required, children }: {
  label: string;
  hint?: string;
  required?: boolean;
  children: React.ReactNode;
}) {
  return (
    <label style={s.field}>
      <span style={s.fieldLabel}>
        {label}{required && <span style={s.required}> *</span>}
      </span>
      {children}
      {hint && <span style={s.fieldHint}>{hint}</span>}
    </label>
  );
}

const s: Record<string, React.CSSProperties> = {
  shell: {
    minHeight: "100vh",
    display: "flex", flexDirection: "column",
    background: "var(--color-neutral-50)",
    fontFamily: "var(--font-ui)",
  },
  header: {
    height: 48, padding: "0 16px",
    display: "flex", alignItems: "center", gap: 16,
    borderBottom: "1px solid var(--color-neutral-200)",
    background: "#fff",
  },
  wordmark: {
    fontFamily: "var(--font-prose)", fontSize: 18, fontWeight: 700,
    color: "var(--color-amber-600)",
  },
  crumbs: { flex: 1, fontSize: 13, color: "var(--color-neutral-600)" },
  closeBtn: {
    background: "none", border: "1px solid var(--color-neutral-300)",
    borderRadius: 4, padding: "4px 12px", fontSize: 12,
    color: "var(--color-neutral-700)", cursor: "pointer",
  },
  progress: {
    display: "flex", gap: 4, padding: "8px 16px",
    background: "#fff",
    borderBottom: "1px solid var(--color-neutral-200)",
  },
  progressTick: {
    height: 4, flex: 1, borderRadius: 2,
    transition: "background 200ms ease",
  },
  body: {
    flex: 1, overflowY: "auto",
    padding: "32px 16px",
    display: "flex", justifyContent: "center",
  },
  step: {
    width: "min(640px, 100%)",
    display: "flex", flexDirection: "column", gap: 16,
  },
  h2: {
    fontFamily: "var(--font-prose)",
    fontSize: 24, fontWeight: 600,
    color: "var(--color-neutral-900)",
    margin: 0,
  },
  lede: {
    fontSize: 13, color: "var(--color-neutral-600)",
    lineHeight: 1.6, margin: 0,
  },
  field: {
    // Explicit width so the field doesn't collapse to min-content
    // inside the flex column step layout.
    display: "flex", flexDirection: "column", gap: 4,
    width: "100%",
  },
  fieldLabel: {
    fontSize: 12, fontWeight: 600,
    color: "var(--color-neutral-700)",
    textTransform: "uppercase", letterSpacing: "0.04em",
  },
  fieldHint: { fontSize: 11, color: "var(--color-neutral-500)" },
  required: { color: "var(--color-amber-600)" },
  input: {
    // Visible-by-default form control — display:block + min-height
    // keeps the input rendering as a full-width block inside the
    // flex column. Border one step darker for contrast.
    display: "block",
    width: "100%", boxSizing: "border-box",
    padding: "8px 12px",
    border: "1px solid var(--color-neutral-400, #9ca3af)",
    borderRadius: 4,
    background: "#fff", color: "var(--color-neutral-900)",
    fontFamily: "var(--font-ui)", fontSize: 14,
    lineHeight: 1.4,
    minHeight: 40,
    outline: "none",
  },
  pathRow: { display: "flex", gap: 8, alignItems: "center" },
  secondaryBtn: {
    flex: "0 0 auto", padding: "8px 16px",
    background: "transparent", color: "var(--color-neutral-800)",
    border: "1px solid var(--color-neutral-300)", borderRadius: 4,
    fontSize: 13, fontWeight: 500, cursor: "pointer",
    fontFamily: "var(--font-ui)",
  },
  code: {
    fontFamily: "var(--font-mono)", fontSize: 11,
    padding: "1px 4px",
    background: "var(--color-neutral-100)", borderRadius: 3,
  },
  review: {
    display: "grid", gridTemplateColumns: "120px 1fr",
    rowGap: 8, columnGap: 12,
    padding: 16,
    background: "#fff",
    border: "1px solid var(--color-neutral-200)", borderRadius: 6,
  },
  dt: {
    fontSize: 11, fontWeight: 600,
    color: "var(--color-neutral-500)",
    textTransform: "uppercase", letterSpacing: "0.06em",
    margin: 0,
  },
  dd: {
    fontSize: 13, color: "var(--color-neutral-900)",
    margin: 0, wordBreak: "break-word",
  },
  error: {
    margin: "0 16px",
    padding: "8px 12px",
    background: "var(--color-red-50, rgba(220,38,38,0.05))",
    color: "var(--color-red-700, #b91c1c)",
    border: "1px solid var(--color-red-200, rgba(220,38,38,0.25))",
    borderRadius: 4,
    fontFamily: "var(--font-mono)", fontSize: 12,
  },
  footer: {
    height: 56, padding: "0 16px",
    display: "flex", alignItems: "center", justifyContent: "space-between",
    borderTop: "1px solid var(--color-neutral-200)",
    background: "#fff",
  },
  ghostBtn: {
    background: "transparent", color: "var(--color-neutral-700)",
    border: "1px solid var(--color-neutral-300)", borderRadius: 4,
    padding: "8px 16px", fontSize: 13, cursor: "pointer",
    fontFamily: "var(--font-ui)",
  },
  primaryBtn: {
    background: "var(--color-amber-600)", color: "#fff",
    border: "none", borderRadius: 4,
    padding: "8px 20px", fontSize: 14, fontWeight: 600, cursor: "pointer",
    fontFamily: "var(--font-ui)",
  },
};
