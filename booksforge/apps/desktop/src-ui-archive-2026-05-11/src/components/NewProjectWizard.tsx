/**
 * New Project Wizard.
 *
 * MVP path:
 *   Step 1: Project name + author
 *   Step 2: Save location (folder picker)
 *   Step 3: Confirm + create   (Step 3 of 3 in the chrome; internally Step 4)
 *
 * Optional MZ-07 AI branch (entered from Step 3 via the "Generate outline
 * with AI" toggle):
 *   Phase "ai-brief"    — collect a ProjectBrief, run the outline-architect
 *   Phase "ai-preview"  — show the proposal; Accept materialises the tree
 */
import React, { useEffect, useRef, useState } from "react";
import { save as saveDialog } from "@tauri-apps/plugin-dialog";
import type {
  AgentRunCompletedEvent,
  AgentRunProgressEvent,
  AgentRunStartedEvent,
  OpenProjectResult,
} from "@booksforge/shared-types";
import { useDialogA11y } from "../lib/useDialogA11y";
import OutlinePreview, { type OutlineProposal } from "./OutlinePreview";
import { ipc } from "../lib/ipc";
import { errorMessage } from "../lib/errorMessage";
import { applyTemplate, TEMPLATES, type TemplateId } from "../lib/projectTemplates";

interface Props {
  onCreated: (result: OpenProjectResult) => void;
  onCancel: () => void;
}

// Phase 5A — added Step 0 ("BookKind picker") in front of the existing
// title/author/save flow. Renumbering the others would touch every
// rendering branch; instead, "0" is the new entry step and 1/2/4 stay
// what they were.
type Step  = 0 | 1 | 2 | 4;
type Phase = "form" | "ai-brief" | "ai-running" | "ai-preview" | "ai-applying";

type BookKindKey =
  | "literary-fiction"
  | "genre-fiction"
  | "non-fiction"
  | "memoir"
  | "childrens-book";

interface BookKindOption {
  key:        BookKindKey;
  name:       string;
  blurb:      string;
  supported:  boolean;
}

const BOOK_KIND_OPTIONS: BookKindOption[] = [
  {
    key:       "literary-fiction",
    name:      "Literary Fiction",
    blurb:     "Voice-driven prose. Sentence-craft over plot. Polish stack: voice → metaphor → dialogue → tension.",
    supported: true,
  },
  {
    key:       "genre-fiction",
    name:      "Genre Fiction",
    blurb:     "Cozy fantasy / thriller / romance / mystery / YA. Pacing first. Polish stack: tension → dialogue → metaphor → voice.",
    supported: true,
  },
  {
    key:       "non-fiction",
    name:      "Non-Fiction",
    blurb:     "Strategy / popular science / long-form essay / business. Argument + evidence weighted highest. Never fabricates stats.",
    supported: true,
  },
  {
    key:       "memoir",
    name:      "Memoir",
    blurb:     "Prose-craft + interiority weighted like literary, with non-fiction's no-fabrication discipline.",
    supported: true,
  },
  {
    key:       "childrens-book",
    name:      "Children's Book (coming soon)",
    blurb:     "Different word counts, layout, reading-level constraints. Out of MVP scope; pick another kind for now.",
    supported: false,
  },
];

interface FormState {
  bookKind:   BookKindKey | null;
  title:      string;
  author:     string;
  bundlePath: string;
  template:   TemplateId;
  useAi:      boolean;
  // AI brief fields (only used if useAi).
  genre:             string;
  audience:          string;
  tone:              string;
  premise:           string;
  // The 1-6 promises the book makes to the reader. Required by the
  // domain validator (`ProjectBrief::validate`); the wizard surfaces
  // it as a textarea where each non-empty line is one promise.
  keyPromises:       string;
  targetWordCount:   number;
  targetChapterCount: number;
}

const EMPTY: FormState = {
  bookKind: null,
  title: "", author: "", bundlePath: "", template: "blank", useAi: false,
  genre: "fantasy", audience: "adult", tone: "adventurous",
  premise: "", keyPromises: "", targetWordCount: 80000, targetChapterCount: 12,
};

export default function NewProjectWizard({ onCreated, onCancel }: Props) {
  const { dialogProps, titleId } = useDialogA11y(onCancel);
  // Phase 5A — wizard now opens on the BookKind picker.
  const [step,  setStep]  = useState<Step>(0);
  const [phase, setPhase] = useState<Phase>("form");
  const [form,  setForm]  = useState<FormState>(EMPTY);
  const [busy,  setBusy]  = useState(false);
  const [error, setError] = useState<string | null>(null);

  // AI flow state.
  const [createdProject, setCreatedProject] = useState<OpenProjectResult | null>(null);
  const [taskId, setTaskId]                 = useState<string | null>(null);
  const [proposal, setProposal]             = useState<OutlineProposal | null>(null);
  const [rawOutput, setRawOutput]           = useState<string | null>(null);
  // Live progress state for the ai-running phase. The backend emits
  // `agent-run-started` (with run_id) → `agent-run-progress` (every 250ms
  // with token count + elapsed_ms) → `agent-run-completed`.
  const [runId, setRunId]                   = useState<string | null>(null);
  const [tokens, setTokens]                 = useState<number>(0);
  const [elapsedMs, setElapsedMs]           = useState<number>(0);
  const [cancelling, setCancelling]         = useState<boolean>(false);
  const startedAtRef = useRef<number | null>(null);

  // Subscribe to outline-architect progress events while in ai-running.
  // Filters by `agent_id === "outline-architect"` so a stray run from
  // elsewhere can't pollute the wizard's counter.
  useEffect(() => {
    if (phase !== "ai-running") return;
    let cancelled = false;
    let unStarted: (() => void) | undefined;
    let unProgress: (() => void) | undefined;
    let unCompleted: (() => void) | undefined;
    (async () => {
      unStarted = await ipc.onAgentRunStarted((e: AgentRunStartedEvent) => {
        if (cancelled) return;
        if (e.agent_id !== "outline-architect") return;
        setRunId(e.run_id);
        startedAtRef.current = Date.now();
      });
      unProgress = await ipc.onAgentRunProgress((e: AgentRunProgressEvent) => {
        if (cancelled) return;
        setTokens(e.tokens);
        setElapsedMs(e.elapsed_ms);
      });
      unCompleted = await ipc.onAgentRunCompleted((e: AgentRunCompletedEvent) => {
        if (cancelled) return;
        if (e.agent_id !== "outline-architect") return;
        // Reset counters once the run lands — the awaiting `handleRunOutline`
        // promise will move the phase to ai-preview / ai-brief.
        setRunId(null);
        setCancelling(false);
      });
    })();
    return () => {
      cancelled = true;
      unStarted?.();
      unProgress?.();
      unCompleted?.();
    };
  }, [phase]);

  // Tick a soft elapsed-time clock at 500ms even when the backend is
  // silent (e.g. between retry attempts or while waiting for the model
  // to start streaming). Falls back to startedAtRef when no progress
  // event has arrived yet, so the user always sees motion.
  const [softTick, setSoftTick] = useState(0);
  useEffect(() => {
    if (phase !== "ai-running") return;
    const id = window.setInterval(() => setSoftTick((t) => t + 1), 500);
    return () => window.clearInterval(id);
  }, [phase]);
  const displayElapsedMs =
    elapsedMs > 0
      ? elapsedMs
      : startedAtRef.current
      ? Date.now() - startedAtRef.current
      : softTick * 500;

  async function handleCancelOutline() {
    if (!runId) {
      // Backend hasn't started yet — just bounce back to the brief.
      setPhase("ai-brief");
      return;
    }
    setCancelling(true);
    try {
      await ipc.agentCancel({ run_id: runId });
    } catch {
      /* idempotent — phase will reset when the awaited IPC resolves */
    }
  }

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

  // Step 4: create the project.  If useAi, advance to the AI-brief phase
  // instead of finishing.
  async function handleCreate() {
    if (!form.title.trim())  { setError("Title is required."); return; }
    if (!form.author.trim()) { setError("Author is required."); return; }
    if (!form.bundlePath.trim()) { setError("Save location is required."); return; }

    setBusy(true);
    setError(null);
    try {
      const result = await ipc.projectCreate({
        title: form.title.trim(),
        author: form.author.trim(),
        bundle_path: form.bundlePath,
        genre: form.useAi ? form.genre : null,
        // Phase 4 — set the project's book_kind from Step 0.
        book_kind: form.bookKind,
      });
      setCreatedProject(result);
      if (form.template !== "blank") {
        try {
          await applyTemplate(form.template);
        } catch (e) {
          // Non-fatal: project was created, but seeding failed.  Surface
          // and let the user proceed with a blank tree.
          setError(`Project created, but template seeding failed: ${errorMessage(e)}`);
        }
      }
      if (form.useAi) {
        setPhase("ai-brief");
      } else {
        onCreated(result);
      }
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  }

  // Phase ai-brief → run agent.
  async function handleRunOutline() {
    if (!createdProject) return;
    if (!form.premise.trim()) { setError("Premise is required."); return; }

    // Domain validator (`ProjectBrief::validate`) requires 1-6 entries.
    // Parse the textarea (one promise per line, blanks dropped).
    const keyPromises = form.keyPromises
      .split("\n")
      .map((s) => s.trim())
      .filter((s) => s.length > 0);
    if (keyPromises.length === 0) {
      setError("Add at least one Key Promise — what the reader will get from the book (one per line, up to 6).");
      return;
    }
    if (keyPromises.length > 6) {
      setError(`Too many Key Promises (${keyPromises.length}). Keep it to 6 or fewer — pick the load-bearing ones.`);
      return;
    }

    setPhase("ai-running");
    setError(null);
    // Reset live counters so a re-run after a failure shows fresh progress.
    setRunId(null);
    setTokens(0);
    setElapsedMs(0);
    setCancelling(false);
    startedAtRef.current = Date.now();
    try {
      const briefJson = JSON.stringify({
        title_suggestions:   [form.title],
        mode:                "fiction",
        genre:               form.genre,
        audience:            form.audience,
        tone:                form.tone,
        target_word_count:   form.targetWordCount,
        premise:             form.premise.trim(),
        key_promises:        keyPromises,
        questions_for_user:  [],
      });
      const result = await ipc.agentRunOutline({
        project_id:          createdProject.project_id,
        brief_json:          briefJson,
        target_chapter_count: form.targetChapterCount,
        genre_overlay:        null,
        // null → backend resolves the right tier per agent (Light for
        // outline-architect → qwen3.5:9b on a fully-installed box).
        // The wizard intentionally no longer asks the user to pick a
        // model — the multi-tier strategy is the orchestrator's job,
        // not the writer's.
        model:                null,
      });
      if (result.status !== "completed" || !result.proposal_json) {
        setError(result.error ?? `Agent returned status: ${result.status}`);
        setRawOutput(result.raw_output ?? null);
        setPhase("ai-brief");
        return;
      }
      setTaskId(result.task_id);
      setProposal(JSON.parse(result.proposal_json) as OutlineProposal);
      setPhase("ai-preview");
    } catch (e) {
      setError(errorMessage(e));
      setPhase("ai-brief");
    }
  }

  // Phase ai-preview → apply.
  async function handleAcceptOutline() {
    if (!createdProject || !taskId) return;
    setPhase("ai-applying");
    setError(null);
    try {
      await ipc.agentApplyOutline({
        project_id:    createdProject.project_id,
        task_id:       taskId,
        project_title: form.title.trim(),
      });
      onCreated(createdProject);
    } catch (e) {
      setError(errorMessage(e));
      setPhase("ai-preview");
    }
  }

  function handleSkipOutline() {
    if (createdProject) onCreated(createdProject);
  }

  // ── Render ─────────────────────────────────────────────────────────────
  return (
    <div style={s.overlay} role="presentation">
      <div {...dialogProps} style={phase === "ai-preview" ? s.panelWide : s.panel}>
        <header style={s.header}>
          <span id={titleId} style={s.stepLabel}>{phaseLabel(step, phase)}</span>
          <button style={s.closeBtn} onClick={onCancel} disabled={busy} aria-label="Cancel new-project wizard">✕</button>
        </header>

        {phase === "form" && step === 0 && (
          <Step0BookKind
            value={form.bookKind}
            onChange={(k) => set("bookKind", k)}
            onNext={() => setStep(1)}
            onCancel={onCancel}
          />
        )}
        {phase === "form" && step === 1 && (
          <Step1 form={form} onChange={set} onBack={() => setStep(0)} onNext={() => setStep(2)} onCancel={onCancel} />
        )}
        {phase === "form" && step === 2 && (
          <Step2
            bundlePath={form.bundlePath}
            onPick={pickSaveLocation}
            onBack={() => setStep(1)}
            onNext={() => setStep(4)}
          />
        )}
        {phase === "form" && step === 4 && (
          <Step4
            form={form}
            onChange={set}
            creating={busy}
            error={error}
            onBack={() => setStep(2)}
            onCreate={handleCreate}
          />
        )}

        {phase === "ai-brief" && (
          <AiBrief
            form={form}
            onChange={set}
            error={error}
            rawOutput={rawOutput}
            onSkip={handleSkipOutline}
            onRun={handleRunOutline}
          />
        )}

        {phase === "ai-running" && (
          <div style={s.body}>
            <h2 style={s.title}>Generating outline…</h2>
            <p style={s.hint}>
              The outline-architect agent is running locally on your machine.
              Typical wall-clock: <b>30–120 s</b> on <code style={s.codeInline}>qwen3.5:9b</code>,{" "}
              <b>2–5 min</b> on <code style={s.codeInline}>qwen3.6:latest</code>.
            </p>

            {/* Live progress block — spinner + token counter + elapsed +
                derived tokens/sec rate. Updates every 250ms via
                `agent-run-progress` events; falls back to a soft 500ms
                wall clock so the user sees motion even before the model
                streams its first token. */}
            <div style={s.runStatus} role="status" aria-live="polite">
              <span style={s.spinner} aria-hidden="true" />
              <div style={s.runStatusText}>
                <span style={s.runStatusLine}>
                  {runId ? (
                    <>
                      <b>{tokens.toLocaleString()}</b> tokens generated ·{" "}
                      <b>{formatElapsed(displayElapsedMs)}</b> elapsed
                      {tokens > 0 && elapsedMs > 500 && (
                        <>
                          {" "}· <b>{((tokens / (elapsedMs / 1000)) || 0).toFixed(1)}</b> tok/s
                        </>
                      )}
                    </>
                  ) : (
                    <>
                      <b>{formatElapsed(displayElapsedMs)}</b> elapsed · waiting for
                      first token from Ollama…
                    </>
                  )}
                </span>
                <span style={s.runStatusHint}>
                  {displayElapsedMs > 180_000
                    ? "Taking longer than usual — model may be retrying after a parse failure. You can cancel and try again."
                    : "Outline drafts run as a single agent call. JSON parse failures retry up to 3× automatically."}
                </span>
              </div>
            </div>

            <div style={s.footer}>
              <button
                style={s.ghostBtn}
                onClick={handleCancelOutline}
                disabled={cancelling}
                aria-label="Cancel outline generation"
              >
                {cancelling ? "Cancelling…" : "Cancel"}
              </button>
            </div>
          </div>
        )}

        {phase === "ai-preview" && proposal && (
          <div style={s.bodyTall}>
            <h2 style={s.title}>Review the proposed outline</h2>
            <p style={s.hint}>
              A pre-edit snapshot will be taken automatically before any changes
              are written, so this is fully reversible.
            </p>
            {error && <p style={s.error}>{error}</p>}
            <div style={s.previewScroll}>
              <OutlinePreview proposal={proposal} />
            </div>
            <div style={s.footer}>
              <button style={s.ghostBtn} onClick={handleSkipOutline}>
                Skip — start blank
              </button>
              <button style={s.primaryBtn} onClick={handleAcceptOutline}>
                Accept and create tree
              </button>
            </div>
          </div>
        )}

        {phase === "ai-applying" && (
          <div style={s.body}>
            <h2 style={s.title}>Building your document tree…</h2>
            <p style={s.hint}>Snapshotting and inserting nodes atomically.</p>
          </div>
        )}
      </div>
    </div>
  );
}

function phaseLabel(step: Step, phase: Phase): string {
  if (phase === "form") {
    // Phase 5A: 4 steps now (0 = book kind, 1 = title/author, 2 = save
    // location, 4 = template + AI toggle).
    const visible = step === 4 ? 4 : step + 1;
    return `Step ${visible} of 4`;
  }
  if (phase === "ai-brief")    return "Outline · brief";
  if (phase === "ai-running")  return "Outline · running";
  if (phase === "ai-preview")  return "Outline · review";
  if (phase === "ai-applying") return "Outline · applying";
  return "";
}

// ── Step 0 — Book kind picker ────────────────────────────────────────────────

function Step0BookKind({
  value, onChange, onNext, onCancel,
}: {
  value:    BookKindKey | null;
  onChange: (k: BookKindKey) => void;
  onNext:   () => void;
  onCancel: () => void;
}) {
  const valid = value !== null
    && BOOK_KIND_OPTIONS.find(o => o.key === value)?.supported === true;
  return (
    <div style={s.body}>
      <h2 style={s.title}>What kind of book is this?</h2>
      <p style={s.hint}>
        BooksForge tunes its drafter prompts, polish-stack ordering, and
        rubric weights to your book&apos;s genre. You can change this later
        in Settings.
      </p>
      <div style={s.kindGrid}>
        {BOOK_KIND_OPTIONS.map(opt => {
          const selected = value === opt.key;
          return (
            <button
              key={opt.key}
              style={{
                ...s.kindCard,
                ...(selected ? s.kindCardSelected : {}),
                ...(opt.supported ? {} : s.kindCardDisabled),
              }}
              onClick={() => opt.supported && onChange(opt.key)}
              disabled={!opt.supported}
              title={opt.supported ? opt.blurb : "Coming soon — pick another kind for now."}
            >
              <strong style={s.kindName}>{opt.name}</strong>
              <div style={s.kindBlurb}>{opt.blurb}</div>
            </button>
          );
        })}
      </div>
      <div style={s.footer}>
        <button style={s.ghostBtn} onClick={onCancel}>Cancel</button>
        <button style={s.primaryBtn} onClick={onNext} disabled={!valid}>
          Next
        </button>
      </div>
    </div>
  );
}

// ── Step 1 ───────────────────────────────────────────────────────────────────
function Step1({
  form, onChange, onBack, onNext, onCancel,
}: {
  form: FormState;
  onChange: <K extends keyof FormState>(k: K, v: FormState[K]) => void;
  onBack: () => void;
  onNext: () => void;
  onCancel: () => void;
}) {
  const valid = form.title.trim().length > 0 && form.author.trim().length > 0;
  return (
    <div style={s.body}>
      <h2 style={s.title}>Name your project</h2>
      <label style={s.label}>
        Book title
        <input style={s.input} value={form.title}
          onChange={(e) => onChange("title", e.target.value)}
          placeholder="e.g. The Midnight Archive" autoFocus />
      </label>
      <label style={s.label}>
        Author
        <input style={s.input} value={form.author}
          onChange={(e) => onChange("author", e.target.value)}
          placeholder="e.g. Jane Smith" />
      </label>
      <div style={s.footer}>
        <button style={s.ghostBtn} onClick={onBack}>Back</button>
        <button style={s.ghostBtn} onClick={onCancel}>Cancel</button>
        <button style={s.primaryBtn} onClick={onNext} disabled={!valid}>Next</button>
      </div>
    </div>
  );
}

// ── Step 2 ───────────────────────────────────────────────────────────────────
function Step2({
  bundlePath, onPick, onBack, onNext,
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
      </p>
      <div style={s.pathRow}>
        <span style={s.pathDisplay}>
          {bundlePath || <span style={{ color: "var(--color-text-tertiary)" }}>No location selected</span>}
        </span>
        <button style={s.secondaryBtn} onClick={onPick}>Browse…</button>
      </div>
      <div style={s.footer}>
        <button style={s.ghostBtn} onClick={onBack}>Back</button>
        <button style={s.primaryBtn} onClick={onNext} disabled={!bundlePath}>Next</button>
      </div>
    </div>
  );
}

// ── Step 4 (the final form step) ─────────────────────────────────────────────
function Step4({
  form, onChange, creating, error, onBack, onCreate,
}: {
  form: FormState;
  onChange: <K extends keyof FormState>(k: K, v: FormState[K]) => void;
  creating: boolean;
  error: string | null;
  onBack: () => void;
  onCreate: () => void;
}) {
  return (
    <div style={s.body}>
      <h2 style={s.title}>Ready to create</h2>
      <dl style={s.summary}>
        <dt style={s.dt}>Title</dt><dd style={s.dd}>{form.title}</dd>
        <dt style={s.dt}>Author</dt><dd style={s.dd}>{form.author}</dd>
        <dt style={s.dt}>Location</dt>
        <dd style={{ ...s.dd, fontFamily: "var(--font-mono)", fontSize: 11 }}>{form.bundlePath}</dd>
      </dl>
      <label style={s.label}>
        Starting template
        <select
          style={s.input}
          value={form.template}
          onChange={(e) => onChange("template", e.target.value as TemplateId)}
          disabled={creating}
        >
          {TEMPLATES.map((t) => (
            <option key={t.id} value={t.id}>{t.label}</option>
          ))}
        </select>
        <span style={{ ...s.hint, fontSize: 12 }}>
          {TEMPLATES.find((t) => t.id === form.template)?.description}
        </span>
      </label>
      <label style={{ ...s.label, flexDirection: "row", alignItems: "center", gap: 8 }}>
        <input
          type="checkbox"
          checked={form.useAi}
          onChange={(e) => onChange("useAi", e.target.checked)}
          disabled={creating}
        />
        Generate an outline with AI after creation (optional)
      </label>
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

// ── AI brief ─────────────────────────────────────────────────────────────────
function AiBrief({
  form, onChange, error, rawOutput, onSkip, onRun,
}: {
  form: FormState;
  onChange: <K extends keyof FormState>(k: K, v: FormState[K]) => void;
  error: string | null;
  rawOutput: string | null;
  onSkip: () => void;
  onRun: () => void;
}) {
  return (
    <div style={s.body}>
      <h2 style={s.title}>Tell the agent about your book</h2>
      <p style={s.hint}>
        The outline-architect runs locally on Ollama. BooksForge picks the right model
        per agent automatically (Light tier for outline, Heavy for prose drafting) — no
        model selection needed.
      </p>

      <div style={s.gridTwo}>
        <label style={s.label}>
          Genre
          <input style={s.input} value={form.genre}
            onChange={(e) => onChange("genre", e.target.value)} />
        </label>
        <label style={s.label}>
          Audience
          <input style={s.input} value={form.audience}
            onChange={(e) => onChange("audience", e.target.value)} />
        </label>
        <label style={s.label}>
          Tone
          <input style={s.input} value={form.tone}
            onChange={(e) => onChange("tone", e.target.value)} />
        </label>
        <label style={s.label}>
          Target word count
          <input style={s.input} type="number" min={20000} max={300000}
            value={form.targetWordCount}
            onChange={(e) => onChange("targetWordCount", Number(e.target.value))} />
        </label>
        <label style={s.label}>
          Target chapter count
          <input style={s.input} type="number" min={6} max={60}
            value={form.targetChapterCount}
            onChange={(e) => onChange("targetChapterCount", Number(e.target.value))} />
        </label>
      </div>

      <label style={s.label}>
        Premise
        <textarea
          style={{ ...s.input, minHeight: 80, fontFamily: "var(--font-prose)" }}
          value={form.premise}
          onChange={(e) => onChange("premise", e.target.value)}
          placeholder="One paragraph describing the core story."
        />
      </label>

      <label style={s.label}>
        Key promises (1–6, one per line)
        <textarea
          style={{ ...s.input, minHeight: 80, fontFamily: "var(--font-prose)" }}
          value={form.keyPromises}
          onChange={(e) => onChange("keyPromises", e.target.value)}
          placeholder={
            "What will the reader actually get from this book?\n" +
            "One promise per line. Examples:\n" +
            "  Sustained dread anchored in everyday objects\n" +
            "  A village whose hierarchy is the real horror\n" +
            "  An ending that doesn't resolve cleanly"
          }
        />
        <span style={{ fontSize: 11, color: "var(--text-muted)" }}>
          Required by the validator (1–6 entries). These shape every chapter — pick the
          load-bearing ones.
        </span>
      </label>

      {error && <p style={s.error}>{error}</p>}
      {rawOutput && (
        <details style={{ fontSize: 11 }}>
          <summary>Raw model output (debug)</summary>
          <pre style={{ whiteSpace: "pre-wrap", fontFamily: "var(--font-mono)" }}>
            {rawOutput}
          </pre>
        </details>
      )}

      <div style={s.footer}>
        <button style={s.ghostBtn} onClick={onSkip}>Skip — start blank</button>
        <button style={s.primaryBtn} onClick={onRun}>Generate outline</button>
      </div>
    </div>
  );
}

// ── Styles ───────────────────────────────────────────────────────────────────
const s: Record<string, React.CSSProperties> = {
  overlay: {
    position: "fixed", inset: 0, background: "rgba(0,0,0,0.4)",
    display: "flex", alignItems: "center", justifyContent: "center", zIndex: 100,
  },
  panel: {
    width: 520, background: "var(--color-surface)", borderRadius: 10,
    boxShadow: "0 20px 60px rgba(0,0,0,0.3)",
    display: "flex", flexDirection: "column", overflow: "hidden",
    maxHeight: "calc(100vh - 80px)",
  },
  panelWide: {
    width: "min(94vw, 880px)", background: "var(--color-surface)", borderRadius: 10,
    boxShadow: "0 20px 60px rgba(0,0,0,0.3)",
    display: "flex", flexDirection: "column", overflow: "hidden",
    maxHeight: "calc(100vh - 80px)",
  },
  header: {
    display: "flex", alignItems: "center", justifyContent: "space-between",
    padding: "var(--space-4) var(--space-6)", borderBottom: "1px solid var(--color-border)",
  },
  stepLabel: {
    fontSize: 12, color: "var(--color-text-tertiary)", fontWeight: 500,
    letterSpacing: "0.04em",
  },
  closeBtn: {
    background: "none", border: "none", color: "var(--color-text-tertiary)",
    fontSize: 16, cursor: "pointer", padding: 4, lineHeight: 1,
  },
  // ── Phase 5A — book-kind picker grid ─────────────────────────────────
  kindGrid: {
    display: "grid", gridTemplateColumns: "1fr 1fr", gap: 10, marginTop: 8,
  },
  kindCard: {
    textAlign: "left", padding: 12,
    background: "var(--color-bg)", color: "inherit",
    border: "2px solid var(--color-border)", borderRadius: 6,
    cursor: "pointer", display: "flex", flexDirection: "column", gap: 4,
  },
  kindCardSelected: {
    borderColor: "var(--color-success, #2e7d32)",
    background: "var(--color-success-bg, rgba(46,125,50,0.08))",
  },
  kindCardDisabled: {
    opacity: 0.5, cursor: "not-allowed",
  },
  kindName:  { fontSize: 14 },
  kindBlurb: { fontSize: 12, opacity: 0.85, lineHeight: 1.4 },
  body: {
    padding: "var(--space-6)", display: "flex", flexDirection: "column",
    gap: "var(--space-4)", overflowY: "auto",
  },
  bodyTall: {
    padding: "var(--space-6)", display: "flex", flexDirection: "column",
    gap: "var(--space-3)", overflowY: "auto", flex: 1, minHeight: 0,
  },
  previewScroll: { flex: 1, minHeight: 0, overflowY: "auto" },
  title: {
    margin: 0, fontSize: 18, fontWeight: 600,
    color: "var(--color-text-primary)", fontFamily: "var(--font-prose)",
  },
  hint: { margin: 0, fontSize: 13, color: "var(--color-text-secondary)", lineHeight: 1.6 },
  label: {
    display: "flex", flexDirection: "column", gap: "var(--space-1)",
    fontSize: 13, fontWeight: 500, color: "var(--color-text-secondary)",
  },
  input: {
    padding: "var(--space-2) var(--space-3)", border: "1px solid var(--color-border)",
    borderRadius: 5, fontSize: 14, background: "var(--color-surface)",
    color: "var(--color-text-primary)", fontFamily: "var(--font-ui)", outline: "none",
    width: "100%", boxSizing: "border-box",
  },
  gridTwo: {
    display: "grid", gridTemplateColumns: "1fr 1fr", gap: "var(--space-3)",
  },
  pathRow:     { display: "flex", gap: "var(--space-2)", alignItems: "center" },
  pathDisplay: {
    flex: 1, fontSize: 12, fontFamily: "var(--font-mono)",
    color: "var(--color-text-primary)", overflow: "hidden",
    textOverflow: "ellipsis", whiteSpace: "nowrap",
    padding: "var(--space-2) var(--space-3)",
    border: "1px solid var(--color-border)", borderRadius: 5,
    minHeight: 36, display: "flex", alignItems: "center",
  },
  summary: {
    margin: 0, display: "grid", gridTemplateColumns: "80px 1fr",
    rowGap: "var(--space-2)", columnGap: "var(--space-3)",
    padding: "var(--space-4)",
    background: "var(--color-neutral-50)", borderRadius: 6,
    border: "1px solid var(--color-border)",
  },
  dt: {
    fontSize: 12, fontWeight: 600, color: "var(--color-text-tertiary)",
    textTransform: "uppercase", letterSpacing: "0.06em", margin: 0, alignSelf: "start",
  },
  dd: { fontSize: 14, color: "var(--color-text-primary)", margin: 0, wordBreak: "break-all" },
  footer: {
    display: "flex", justifyContent: "flex-end",
    gap: "var(--space-2)", marginTop: "var(--space-2)",
  },
  primaryBtn: {
    padding: "var(--space-2) var(--space-5)", background: "var(--color-amber-600)",
    color: "#fff", border: "none", borderRadius: 5,
    fontSize: 14, fontWeight: 600, cursor: "pointer", fontFamily: "var(--font-ui)",
  },
  secondaryBtn: {
    padding: "var(--space-2) var(--space-4)", background: "transparent",
    color: "var(--color-text-primary)", border: "1px solid var(--color-border)",
    borderRadius: 5, fontSize: 13, cursor: "pointer", fontFamily: "var(--font-ui)",
  },
  ghostBtn: {
    padding: "var(--space-2) var(--space-4)", background: "transparent",
    color: "var(--color-text-secondary)", border: "1px solid var(--color-border)",
    borderRadius: 5,
    fontSize: 14, cursor: "pointer", fontFamily: "var(--font-ui)",
  },
  error: { color: "var(--color-error)", fontFamily: "var(--font-mono)", fontSize: 12, margin: 0 },
  // ── Live progress styles for ai-running phase ──────────────────────────
  runStatus: {
    display: "flex", alignItems: "flex-start", gap: "var(--space-3)",
    padding: "var(--space-4)",
    background: "var(--color-neutral-50, rgba(0,0,0,0.03))",
    border: "1px solid var(--color-border)", borderRadius: 6,
  },
  runStatusText: {
    display: "flex", flexDirection: "column", gap: "var(--space-1)", flex: 1,
  },
  runStatusLine: {
    fontSize: 13, color: "var(--color-text-primary)",
    fontVariantNumeric: "tabular-nums", fontFamily: "var(--font-ui)",
  },
  runStatusHint: {
    fontSize: 11, color: "var(--color-text-tertiary)", lineHeight: 1.5,
  },
  spinner: {
    width: 18, height: 18, flexShrink: 0,
    borderRadius: "50%",
    border: "2px solid var(--color-border)",
    borderTopColor: "var(--color-amber-600)",
    animation: "bf-wizard-spin 0.9s linear infinite",
    marginTop: 1,
  },
  codeInline: {
    fontFamily: "var(--font-mono)", fontSize: 12,
    padding: "1px 5px",
    background: "var(--color-surface-raised, rgba(0,0,0,0.05))",
    borderRadius: 3,
  },
};

// Inject the spinner keyframes once. Inline `<style>` would re-inject on
// every render; this runs at module load and is a no-op on subsequent
// imports (HMR-safe via the data-bf-anim guard).
if (typeof document !== "undefined" && !document.getElementById("bf-wizard-anim")) {
  const styleEl = document.createElement("style");
  styleEl.id = "bf-wizard-anim";
  styleEl.textContent = `@keyframes bf-wizard-spin {
    from { transform: rotate(0deg); }
    to   { transform: rotate(360deg); }
  }`;
  document.head.appendChild(styleEl);
}

/// Format milliseconds as a short human elapsed string ("12s", "2m 14s").
function formatElapsed(ms: number): string {
  const total = Math.floor(ms / 1000);
  if (total < 60) return `${total}s`;
  const m = Math.floor(total / 60);
  const s = total % 60;
  return `${m}m ${s.toString().padStart(2, "0")}s`;
}
