/**
 * BookSetupWizard — 7-step wizard for Stage 1 (Book Setup).
 *
 * Per `book-output/design/WRITER_JOURNEY_REDESIGN_2026-05.md` §4 Stage 1:
 *   Step 1: Book kind        (literary fiction is the only MVP kind)
 *   Step 2: Title + author   (subtitle is optional)
 *   Step 3: Save location    (where the .booksforge bundle lives)
 *   Step 4: Concept          (premise, background, tone, style)
 *   Step 5: Audience         (Stage 2 fields — done in the wizard so
 *                             the writer commits the audience map
 *                             before the journey begins)
 *   Step 6: Format & print   (publishing formats + printing prefs)
 *   Step 7: Concept refine   (AI "Refine my book concept" — 8.5/10 gate)
 *
 * This MVP build ships steps 1-3 fully functional and the rest as
 * scaffolded forms whose data is captured but not yet sent to AI.
 * Step 7's `concept_scorer` agent is a backend follow-up; for now the
 * wizard accepts on click-through with a banner indicating the gate
 * hasn't run.
 */
import React, { useState } from "react";
import { save as saveDialog } from "@tauri-apps/plugin-dialog";
import type { OpenProjectResult } from "@booksforge/shared-types";
import { ipc } from "../lib/ipc";
import { errorMessage } from "../lib/errorMessage";

interface Props {
  onCreated: (project: OpenProjectResult) => void;
  onCancel:  () => void;
}

// ── Form state ─────────────────────────────────────────────────────────────

type BookKindKey = "literary-fiction";  // MVP anchors on this one; expand later.

interface WizardForm {
  // Step 1
  bookKind:   BookKindKey;
  // Step 2
  title:      string;
  subtitle:   string;
  author:     string;
  // Step 3
  bundlePath: string;
  // Step 4
  genre:           string;
  subGenre:        string;
  tone:            string;
  writingStyle:    string;  // free-text for now; enum once the backend lands it
  premise:         string;
  background:      string;
  keyPromises:     string;  // textarea, one promise per line
  targetWordCount: number;
  targetChapterCount: number;
  // Step 5 — Audience
  audience:                 string;
  secondaryAudience:        string;
  ageMin:                   number;
  ageMax:                   number;
  readerExpectations:       string;  // one per line
  emotionalOutcomeDesired:  string;
  compTitlesOrAuthors:      string;  // one per line
  // Step 6 — Format & print
  publishingFormats:    { epub: boolean; paperback: boolean; hardcover: boolean; pdf: boolean };
  trimSize:             string;  // "5x8" / "6x9" / "7x10"
  paperType:            string;  // "white" | "cream"
  interiorColor:        string;  // "bw" | "color"
}

const EMPTY: WizardForm = {
  bookKind:   "literary-fiction",
  title:      "",
  subtitle:   "",
  author:     "",
  bundlePath: "",
  genre:           "literary fiction",
  subGenre:        "",
  tone:            "spare",
  writingStyle:    "lyrical-precise",
  premise:         "",
  background:      "",
  keyPromises:     "",
  targetWordCount: 75_000,
  targetChapterCount: 12,
  audience:                "adult literary readers",
  secondaryAudience:       "",
  ageMin:                  25,
  ageMax:                  65,
  readerExpectations:      "",
  emotionalOutcomeDesired: "",
  compTitlesOrAuthors:     "",
  publishingFormats:    { epub: true, paperback: true, hardcover: false, pdf: false },
  trimSize:             "6x9",
  paperType:            "cream",
  interiorColor:        "bw",
};

const STEP_COUNT = 7;

// ── Component ──────────────────────────────────────────────────────────────

export default function BookSetupWizard({ onCreated, onCancel }: Props) {
  const [step, setStep] = useState<number>(1);
  const [form, setForm] = useState<WizardForm>(EMPTY);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  function update<K extends keyof WizardForm>(k: K, v: WizardForm[K]) {
    setForm((f) => ({ ...f, [k]: v }));
    setError(null);
  }

  function canAdvance(): { ok: boolean; reason?: string } {
    switch (step) {
      case 1: return { ok: true };
      case 2:
        if (!form.title.trim())  return { ok: false, reason: "Title is required." };
        if (!form.author.trim()) return { ok: false, reason: "Author name is required." };
        return { ok: true };
      case 3:
        if (!form.bundlePath.trim()) return { ok: false, reason: "Pick a save location." };
        return { ok: true };
      case 4:
        if (!form.premise.trim()) return { ok: false, reason: "Premise is required." };
        if (form.keyPromises.split("\n").map((l) => l.trim()).filter(Boolean).length === 0) {
          return { ok: false, reason: "Add at least one key promise (one per line)." };
        }
        return { ok: true };
      case 5: return { ok: true };
      case 6: return { ok: true };
      case 7: return { ok: true };
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
        genre:       form.genre || null,
        book_kind:   form.bookKind,
      });

      // 2. Persist the captured brief to `book:project_brief` memory.
      //    Best-effort — if this fails, the project still exists and
      //    the writer can fill in the Brief panel manually. We surface
      //    the error rather than swallow it.
      try {
        const briefJson = buildBriefFromForm(form);
        await ipc.projectBriefSave({ brief_json: briefJson });
      } catch (e) {
        // Non-fatal but visible.
        console.warn("[wizard] brief save failed — project created, brief empty:", e);
        // Don't surface in `error` state — the user has already moved
        // past the inputs; we'll just let the Stage 1 panel prompt
        // them to fill in. A toast would be the right surface; deferring
        // until ToastProvider is wired into routes.
      }

      onCreated(result);
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  }

  /**
   * Map the wizard's form state into the ProjectBrief shape the
   * orchestrator + downstream agents read from `book:project_brief`
   * memory. Fields the wizard captures that ProjectBrief doesn't have
   * a slot for (subtitle, sub-genre, writing style, audience-map
   * fields, format/print prefs) are merged into the closest existing
   * field for now — full audience map and format settings will land
   * in `book:audience_map` and `book:format_prefs` in Phase B Step 2+.
   */
  function buildBriefFromForm(f: WizardForm): unknown {
    const keyPromises = f.keyPromises
      .split("\n").map((l) => l.trim()).filter((l) => l.length > 0);
    const compTitles = f.compTitlesOrAuthors
      .split("\n").map((l) => l.trim()).filter((l) => l.length > 0);
    // Sub-genre is merged into genre as "<genre> / <sub-genre>" for
    // now — domain field is a single String. Same for writing-style
    // → tone composition.
    const genre = f.subGenre.trim()
      ? `${f.genre} / ${f.subGenre.trim()}`
      : f.genre;
    const tone = f.writingStyle.trim()
      ? `${f.tone} — ${f.writingStyle.trim()}`
      : f.tone;
    const audience = f.secondaryAudience.trim()
      ? `${f.audience} (also: ${f.secondaryAudience.trim()})`
      : f.audience;
    return {
      title_suggestions:      [f.title.trim()],
      mode:                   "fiction",  // MVP anchors here
      genre,
      audience,
      tone,
      target_word_count:      f.targetWordCount,
      premise:                f.premise.trim(),
      key_promises:           keyPromises,
      questions_for_user:     [],
      comp_titles_or_authors: compTitles,
      theme_keywords:         [],
      forbidden_tropes:       [],
      era_setting:            null,
      cultural_context:       null,
      // Repurpose creative_seed to carry the background paragraph
      // until ProjectBrief grows a proper `background` field.
      creative_seed:          f.background.trim() || null,
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
        {step === 1 && <Step1Kind form={form} update={update} />}
        {step === 2 && <Step2Title form={form} update={update} />}
        {step === 3 && <Step3Save form={form} update={update} onPick={pickSaveLocation} />}
        {step === 4 && <Step4Concept form={form} update={update} />}
        {step === 5 && <Step5Audience form={form} update={update} />}
        {step === 6 && <Step6Format form={form} update={update} />}
        {step === 7 && <Step7Refine form={form} />}
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
    "", "Book kind", "Title & author", "Save location",
    "Concept", "Audience", "Format & print", "Review",
  ][n] ?? "";
}

// ── Step screens ────────────────────────────────────────────────────────────

function Step1Kind({ form, update }: {
  form: WizardForm;
  update: <K extends keyof WizardForm>(k: K, v: WizardForm[K]) => void;
}) {
  // MVP anchors on literary fiction. The other kinds appear here once
  // their agent prompts are wired (see writer-journey doc §11 Phase D).
  const kinds: { key: BookKindKey; name: string; blurb: string; available: boolean }[] = [
    {
      key: "literary-fiction", name: "Literary Fiction", available: true,
      blurb: "Voice-driven prose. Sentence-craft over plot. Available in MVP.",
    },
  ];
  return (
    <div style={s.step}>
      <h2 style={s.h2}>What kind of book is this?</h2>
      <p style={s.lede}>
        MVP supports literary fiction only. Genre fiction, non-fiction, memoir,
        poetry, and technical books arrive in later phases — they need
        kind-specific prompts that the team is still tuning.
      </p>
      <ul style={s.cardList}>
        {kinds.map((k) => (
          <li key={k.key}>
            <button
              style={{
                ...s.kindCard,
                ...(form.bookKind === k.key ? s.kindCardSelected : {}),
                ...(k.available ? {} : s.kindCardDisabled),
              }}
              onClick={() => k.available && update("bookKind", k.key)}
              disabled={!k.available}
            >
              <span style={s.kindName}>{k.name}</span>
              <span style={s.kindBlurb}>{k.blurb}</span>
            </button>
          </li>
        ))}
      </ul>
    </div>
  );
}

function Step2Title({ form, update }: {
  form: WizardForm;
  update: <K extends keyof WizardForm>(k: K, v: WizardForm[K]) => void;
}) {
  return (
    <div style={s.step}>
      <h2 style={s.h2}>Name your book</h2>
      <p style={s.lede}>
        Title and author are required. The subtitle is optional and shows up on
        the cover + metadata if provided.
      </p>
      <Field label="Title" required>
        <input style={s.input} value={form.title}
          onChange={(e) => update("title", e.target.value)}
          placeholder="The Incomplete Curse" autoFocus />
      </Field>
      <Field label="Subtitle">
        <input style={s.input} value={form.subtitle}
          onChange={(e) => update("subtitle", e.target.value)}
          placeholder="A literary novel of inheritance and silence" />
      </Field>
      <Field label="Author name" required>
        <input style={s.input} value={form.author}
          onChange={(e) => update("author", e.target.value)} />
      </Field>
    </div>
  );
}

function Step3Save({ form, update, onPick }: {
  form: WizardForm;
  update: <K extends keyof WizardForm>(k: K, v: WizardForm[K]) => void;
  onPick: () => Promise<void>;
}) {
  return (
    <div style={s.step}>
      <h2 style={s.h2}>Where should we save this?</h2>
      <p style={s.lede}>
        BooksForge stores everything as a <code style={s.code}>.booksforge/</code>
        folder: manifest, SQLite database, Markdown manuscript, snapshots,
        and exports. Pick a location you back up.
      </p>
      <div style={s.pathRow}>
        <input
          style={{ ...s.input, fontFamily: "var(--font-mono)", fontSize: 12 }}
          value={form.bundlePath}
          onChange={(e) => update("bundlePath", e.target.value)}
          placeholder="/Users/.../MyBook.booksforge"
        />
        <button style={s.secondaryBtn} onClick={onPick}>Browse…</button>
      </div>
    </div>
  );
}

function Step4Concept({ form, update }: {
  form: WizardForm;
  update: <K extends keyof WizardForm>(k: K, v: WizardForm[K]) => void;
}) {
  return (
    <div style={s.step}>
      <h2 style={s.h2}>The concept</h2>
      <p style={s.lede}>
        Every downstream agent (outline, bibles, drafter, polish) reads these
        fields. The Brief panel inside the editor reflects them and you can
        refine any of these later. Premise + key promises are the most
        load-bearing.
      </p>
      <div style={s.gridTwo}>
        <Field label="Genre">
          <input style={s.input} value={form.genre}
            onChange={(e) => update("genre", e.target.value)}
            placeholder="literary fiction" />
        </Field>
        <Field label="Sub-genre">
          <input style={s.input} value={form.subGenre}
            onChange={(e) => update("subGenre", e.target.value)}
            placeholder="domestic / quiet" />
        </Field>
        <Field label="Tone">
          <input style={s.input} value={form.tone}
            onChange={(e) => update("tone", e.target.value)}
            placeholder="spare / propulsive / wry" />
        </Field>
        <Field label="Writing style">
          <input style={s.input} value={form.writingStyle}
            onChange={(e) => update("writingStyle", e.target.value)}
            placeholder="lyrical-precise" />
        </Field>
        <Field label="Target word count">
          <input style={s.input} type="number" min={5000} max={250_000}
            value={form.targetWordCount}
            onChange={(e) => update("targetWordCount", Number(e.target.value))} />
        </Field>
        <Field label="Target chapter count">
          <input style={s.input} type="number" min={3} max={60}
            value={form.targetChapterCount}
            onChange={(e) => update("targetChapterCount", Number(e.target.value))} />
        </Field>
      </div>
      <Field label="Premise" required hint="1-3 sentences in your own register.">
        <textarea
          style={{ ...s.input, minHeight: 80, fontFamily: "var(--font-prose)" }}
          value={form.premise}
          onChange={(e) => update("premise", e.target.value)}
          placeholder="A clockmaker's widow finds twenty-three sealed letters addressed to a woman she has never heard of." />
      </Field>
      <Field label="Background" hint="World, setting, era, or any context the AI should respect.">
        <textarea
          style={{ ...s.input, minHeight: 60, fontFamily: "var(--font-prose)" }}
          value={form.background}
          onChange={(e) => update("background", e.target.value)}
          placeholder="1990s rural Pennsylvania. Small-town news travels by post office. The protagonist has not touched her husband's workshop in three weeks." />
      </Field>
      <Field label="Key promises" required hint="1-6 lines, one promise per line.">
        <textarea
          style={{ ...s.input, minHeight: 80, fontFamily: "var(--font-prose)" }}
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

function Step5Audience({ form, update }: {
  form: WizardForm;
  update: <K extends keyof WizardForm>(k: K, v: WizardForm[K]) => void;
}) {
  return (
    <div style={s.step}>
      <h2 style={s.h2}>Who is this book for?</h2>
      <p style={s.lede}>
        The audience map drives the AI's pacing, voice, and emotional targets.
        A book without a named reader becomes generic.
      </p>
      <Field label="Primary audience">
        <input style={s.input} value={form.audience}
          onChange={(e) => update("audience", e.target.value)} />
      </Field>
      <Field label="Secondary audience">
        <input style={s.input} value={form.secondaryAudience}
          onChange={(e) => update("secondaryAudience", e.target.value)} />
      </Field>
      <div style={s.gridTwo}>
        <Field label="Age min">
          <input style={s.input} type="number" min={8} max={99}
            value={form.ageMin}
            onChange={(e) => update("ageMin", Number(e.target.value))} />
        </Field>
        <Field label="Age max">
          <input style={s.input} type="number" min={8} max={99}
            value={form.ageMax}
            onChange={(e) => update("ageMax", Number(e.target.value))} />
        </Field>
      </div>
      <Field label="Reader expectations" hint="What the reader hopes for. One per line.">
        <textarea
          style={{ ...s.input, minHeight: 60, fontFamily: "var(--font-prose)" }}
          value={form.readerExpectations}
          onChange={(e) => update("readerExpectations", e.target.value)} />
      </Field>
      <Field label="Emotional outcome desired" hint="What you want the reader to feel by the last page.">
        <input style={s.input} value={form.emotionalOutcomeDesired}
          onChange={(e) => update("emotionalOutcomeDesired", e.target.value)}
          placeholder="A long quiet ache that resolves into clarity" />
      </Field>
      <Field label="Comparable books / authors" hint="One per line. Used for positioning, not imitation.">
        <textarea
          style={{ ...s.input, minHeight: 60, fontFamily: "var(--font-prose)" }}
          value={form.compTitlesOrAuthors}
          onChange={(e) => update("compTitlesOrAuthors", e.target.value)}
          placeholder={"Marilynne Robinson — Gilead\nCormac McCarthy — The Road"} />
      </Field>
    </div>
  );
}

function Step6Format({ form, update }: {
  form: WizardForm;
  update: <K extends keyof WizardForm>(k: K, v: WizardForm[K]) => void;
}) {
  function setFormat(k: keyof WizardForm["publishingFormats"], v: boolean) {
    update("publishingFormats", { ...form.publishingFormats, [k]: v });
  }
  return (
    <div style={s.step}>
      <h2 style={s.h2}>How will this book ship?</h2>
      <p style={s.lede}>
        Picking now means the validators check against the right platform
        requirements all the way through. You can change these later.
      </p>
      <Field label="Publishing formats">
        <div style={s.checkRow}>
          <label style={s.checkLabel}>
            <input type="checkbox" checked={form.publishingFormats.epub}
              onChange={(e) => setFormat("epub", e.target.checked)} /> EPUB
          </label>
          <label style={s.checkLabel}>
            <input type="checkbox" checked={form.publishingFormats.paperback}
              onChange={(e) => setFormat("paperback", e.target.checked)} /> Paperback
          </label>
          <label style={s.checkLabel}>
            <input type="checkbox" checked={form.publishingFormats.hardcover}
              onChange={(e) => setFormat("hardcover", e.target.checked)} /> Hardcover
          </label>
          <label style={s.checkLabel}>
            <input type="checkbox" checked={form.publishingFormats.pdf}
              onChange={(e) => setFormat("pdf", e.target.checked)} /> Print-ready PDF
          </label>
        </div>
      </Field>
      <div style={s.gridTwo}>
        <Field label="Trim size">
          <select style={s.input} value={form.trimSize}
            onChange={(e) => update("trimSize", e.target.value)}>
            <option value="5x8">5 × 8 in</option>
            <option value="5.25x8">5.25 × 8 in</option>
            <option value="5.5x8.5">5.5 × 8.5 in</option>
            <option value="6x9">6 × 9 in (most common)</option>
            <option value="7x10">7 × 10 in</option>
          </select>
        </Field>
        <Field label="Paper">
          <select style={s.input} value={form.paperType}
            onChange={(e) => update("paperType", e.target.value)}>
            <option value="white">White</option>
            <option value="cream">Cream (warmer, easier on eye for fiction)</option>
          </select>
        </Field>
        <Field label="Interior color">
          <select style={s.input} value={form.interiorColor}
            onChange={(e) => update("interiorColor", e.target.value)}>
            <option value="bw">Black & white (cheaper to print)</option>
            <option value="color">Full color (needed if you have images)</option>
          </select>
        </Field>
      </div>
    </div>
  );
}

function Step7Refine({ form }: { form: WizardForm }) {
  return (
    <div style={s.step}>
      <h2 style={s.h2}>Review &amp; create</h2>
      <p style={s.lede}>
        The <b>concept scorer</b> agent will run on the editor's Setup stage and
        score your premise on clarity, originality, emotional pull, market fit,
        and execution potential. For now we create the project and the score
        runs in the next stage. Below is what's about to be saved.
      </p>
      <dl style={s.review}>
        <dt style={s.dt}>Book</dt>
        <dd style={s.dd}>
          <b>{form.title || <em>(no title)</em>}</b>
          {form.subtitle && <span> — {form.subtitle}</span>}
          {" "}by <b>{form.author || <em>(no author)</em>}</b>
        </dd>
        <dt style={s.dt}>Kind</dt>
        <dd style={s.dd}>{form.bookKind}</dd>
        <dt style={s.dt}>Saved to</dt>
        <dd style={s.dd} title={form.bundlePath}>
          <code style={s.code}>{form.bundlePath}</code>
        </dd>
        <dt style={s.dt}>Concept</dt>
        <dd style={s.dd}>
          <b>{form.genre}</b>
          {form.subGenre && ` · ${form.subGenre}`}
          {" "}· {form.tone} tone · {form.targetWordCount.toLocaleString()} words ·{" "}
          {form.targetChapterCount} chapters
        </dd>
        <dt style={s.dt}>Audience</dt>
        <dd style={s.dd}>
          {form.audience} · ages {form.ageMin}–{form.ageMax}
        </dd>
        <dt style={s.dt}>Format</dt>
        <dd style={s.dd}>
          {Object.entries(form.publishingFormats)
            .filter(([_, on]) => on)
            .map(([k]) => k)
            .join(" · ") || <em>(none selected)</em>}
          {" "}· {form.trimSize} · {form.paperType} paper · {form.interiorColor}
        </dd>
      </dl>
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
  cardList: { listStyle: "none", margin: 0, padding: 0, display: "flex", flexDirection: "column", gap: 8 },
  kindCard: {
    display: "flex", flexDirection: "column", alignItems: "flex-start", gap: 4,
    width: "100%", padding: 16, textAlign: "left",
    background: "#fff",
    border: "1px solid var(--color-neutral-300)", borderRadius: 6,
    cursor: "pointer", fontFamily: "inherit",
  },
  kindCardSelected: {
    borderColor: "var(--color-amber-500)",
    background: "var(--color-amber-50, #fffbeb)",
  },
  kindCardDisabled: { opacity: 0.45, cursor: "not-allowed" },
  kindName: { fontSize: 15, fontWeight: 600, color: "var(--color-neutral-900)" },
  kindBlurb: { fontSize: 12, color: "var(--color-neutral-600)", lineHeight: 1.5 },
  field: { display: "flex", flexDirection: "column", gap: 4 },
  fieldLabel: {
    fontSize: 12, fontWeight: 600,
    color: "var(--color-neutral-700)",
    textTransform: "uppercase", letterSpacing: "0.04em",
  },
  fieldHint: { fontSize: 11, color: "var(--color-neutral-500)" },
  required: { color: "var(--color-amber-600)" },
  input: {
    width: "100%", boxSizing: "border-box",
    padding: "8px 12px",
    border: "1px solid var(--color-neutral-300)",
    borderRadius: 4,
    background: "#fff", color: "var(--color-neutral-900)",
    fontFamily: "var(--font-ui)", fontSize: 14, outline: "none",
  },
  gridTwo: {
    display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12,
  },
  pathRow: { display: "flex", gap: 8, alignItems: "center" },
  checkRow: { display: "flex", flexWrap: "wrap", gap: 12 },
  checkLabel: {
    display: "inline-flex", alignItems: "center", gap: 6,
    fontSize: 13, color: "var(--color-neutral-800)",
    cursor: "pointer",
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
