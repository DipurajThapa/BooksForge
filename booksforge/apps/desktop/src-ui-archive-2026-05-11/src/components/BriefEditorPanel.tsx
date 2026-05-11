/**
 * BriefEditorPanel — Round 5 of `PRODUCT_ROADMAP_E2E.md`.
 *
 * Lets the writer edit the persisted `ProjectBrief` after intake. The
 * 6 uniqueness fields it surfaces (`comp_titles_or_authors`,
 * `theme_keywords`, `forbidden_tropes`, `era_setting`,
 * `cultural_context`, `creative_seed`) drive the orchestrator's
 * `creative_profile` block — without this panel they can only be
 * populated by the intake agent's auto-extraction from the writer's
 * idea text.
 *
 * The panel also exposes the structural fields (`title_suggestions`,
 * `genre`, `audience`, `tone`, `target_word_count`, `premise`,
 * `key_promises`) so a writer can correct intake mistakes without
 * re-running intake. `mode` and `questions_for_user` stay read-only —
 * they're outputs of intake's reasoning that the user shouldn't be
 * patching by hand.
 *
 * Backend round-trip: `project_brief_load` → edit in this form →
 * `project_brief_save` (validates against `ProjectBrief::validate`
 * before persisting to book-scope memory).
 */
import React, { useEffect, useState } from "react";
import { useDialogA11y } from "../lib/useDialogA11y";
import { ipc } from "../lib/ipc";
import { errorMessage } from "../lib/errorMessage";

interface Props {
  onClose: () => void;
}

/**
 * Working copy of the brief held in form state. Matches
 * `crates/booksforge-domain/src/brief.rs::ProjectBrief` field-by-field.
 * `mode` and `questions_for_user` are intentionally absent from the
 * form (read-only on the backend). Numeric word count is held as a
 * string so partial typing doesn't fight the input.
 */
interface BriefForm {
  title_suggestions:        string[];
  genre:                    string;
  audience:                 string;
  tone:                     string;
  target_word_count:        string;
  premise:                  string;
  key_promises:             string[];
  comp_titles_or_authors:   string[];
  theme_keywords:           string[];
  forbidden_tropes:         string[];
  era_setting:              string;
  cultural_context:         string;
  creative_seed:            string;
}

const EMPTY: BriefForm = {
  title_suggestions:      [],
  genre:                  "",
  audience:               "",
  tone:                   "",
  target_word_count:      "50000",
  premise:                "",
  key_promises:           [],
  comp_titles_or_authors: [],
  theme_keywords:         [],
  forbidden_tropes:       [],
  era_setting:            "",
  cultural_context:       "",
  creative_seed:          "",
};

export default function BriefEditorPanel({ onClose }: Props) {
  const { dialogProps, titleId } = useDialogA11y(onClose);
  const [form,    setForm]    = useState<BriefForm>(EMPTY);
  const [origMode, setOrigMode] = useState<string>("fiction");
  const [loading, setLoading] = useState(true);
  const [saving,  setSaving]  = useState(false);
  const [loaded,  setLoaded]  = useState<boolean>(false);
  const [error,   setError]   = useState<string | null>(null);
  const [savedHint, setSavedHint] = useState<string | null>(null);
  // Provenance metadata so the writer knows where the loaded brief
  // came from. `source` is one of "wizard" / "intake" / "user-edit".
  const [briefSource,   setBriefSource]   = useState<string>("");
  const [briefSavedAt,  setBriefSavedAt]  = useState<string>("");

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const r = await ipc.projectBriefLoad();
        if (cancelled) return;
        setLoaded(r.loaded);
        // Provenance metadata (source, updated_at) added 2026-05 so the
        // writer can see whether the brief came from the wizard, intake
        // agent, or a manual edit. Lets us surface "loaded from wizard"
        // confidence on this panel.
        setBriefSource((r as { source?: string }).source ?? "");
        setBriefSavedAt((r as { updated_at?: string }).updated_at ?? "");
        if (r.loaded) {
          // brief_json is `unknown` from the IPC binding — safe-coerce.
          const b = r.brief_json as Partial<{ [K in keyof BriefForm]: unknown }> & {
            title_suggestions?: string[];
            mode?: string;
            target_word_count?: number | string;
            key_promises?: string[];
            comp_titles_or_authors?: string[];
            theme_keywords?: string[];
            forbidden_tropes?: string[];
            era_setting?: string | null;
            cultural_context?: string | null;
            creative_seed?: string | null;
          };
          setOrigMode(typeof b.mode === "string" ? b.mode : "fiction");
          setForm({
            title_suggestions:      asStrArr(b.title_suggestions),
            genre:                  asStr(b.genre),
            audience:               asStr(b.audience),
            tone:                   asStr(b.tone),
            target_word_count:      String(b.target_word_count ?? "50000"),
            premise:                asStr(b.premise),
            key_promises:           asStrArr(b.key_promises),
            comp_titles_or_authors: asStrArr(b.comp_titles_or_authors),
            theme_keywords:         asStrArr(b.theme_keywords),
            forbidden_tropes:       asStrArr(b.forbidden_tropes),
            era_setting:            asStr(b.era_setting),
            cultural_context:       asStr(b.cultural_context),
            creative_seed:          asStr(b.creative_seed),
          });
        }
      } catch (e) {
        if (!cancelled) setError(errorMessage(e));
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();
    return () => { cancelled = true; };
  }, []);

  function field<K extends keyof BriefForm>(k: K, v: BriefForm[K]) {
    setForm(prev => ({ ...prev, [k]: v }));
    setSavedHint(null);
  }

  function listField(k: "title_suggestions" | "key_promises" | "comp_titles_or_authors"
                       | "theme_keywords" | "forbidden_tropes", raw: string) {
    const arr = raw.split(/[;\n,]+/).map(s => s.trim()).filter(Boolean);
    field(k, arr);
  }

  async function handleSave() {
    setSaving(true);
    setError(null);
    setSavedHint(null);
    const wordCount = Number.parseInt(form.target_word_count, 10);
    if (Number.isNaN(wordCount) || wordCount < 5_000 || wordCount > 250_000) {
      setError("Target word count must be between 5,000 and 250,000.");
      setSaving(false);
      return;
    }
    const briefJson = {
      title_suggestions:      form.title_suggestions,
      mode:                   origMode,
      genre:                  form.genre,
      audience:               form.audience,
      tone:                   form.tone,
      target_word_count:      wordCount,
      premise:                form.premise,
      key_promises:           form.key_promises.length > 0
                                ? form.key_promises
                                : ["[ADD A KEY PROMISE]"],
      questions_for_user:     [],
      comp_titles_or_authors: form.comp_titles_or_authors,
      theme_keywords:         form.theme_keywords,
      forbidden_tropes:       form.forbidden_tropes,
      era_setting:            form.era_setting.trim() || null,
      cultural_context:       form.cultural_context.trim() || null,
      creative_seed:          form.creative_seed.trim() || null,
    };
    try {
      await ipc.projectBriefSave({ brief_json: briefJson });
      setLoaded(true);
      setSavedHint("Saved. Subsequent agent runs will pick up these signals.");
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setSaving(false);
    }
  }

  return (
    <div style={s.overlay} role="presentation">
      <div {...dialogProps} style={s.dialog}>
        <header style={s.header}>
          <strong id={titleId}>Project brief</strong>
          <button style={s.close} onClick={onClose} aria-label="Close panel">✕</button>
        </header>

        <div style={s.body}>
          {loading && <div style={s.hint}>Loading brief…</div>}

          {!loading && (
            <>
              <p style={s.blurb}>
                Persisted with the project. Every agent reads the uniqueness fields
                below to braid your story's signature through bibles, drafts, and
                polish — without them, two writers picking the same genre get
                near-identical books.
              </p>
              {loaded && briefSource && (
                <p style={{
                  margin: "0 0 var(--space-2)",
                  fontSize: 12,
                  color: "var(--color-success, #22c55e)",
                  background: "rgba(34,197,94,0.06)",
                  border: "1px solid rgba(34,197,94,0.25)",
                  borderRadius: 4,
                  padding: "6px 10px",
                }}>
                  ✓ Loaded from <b>{labelForSource(briefSource)}</b>
                  {briefSavedAt && <> · last saved {formatLastSaved(briefSavedAt)}</>}.
                  Edit any field and click Save to update — every agent run that
                  follows will use the new values.
                </p>
              )}
              {!loaded && (
                <p style={{
                  margin: "0 0 var(--space-2)",
                  fontSize: 12,
                  color: "var(--color-text-secondary)",
                  background: "var(--color-neutral-50, rgba(0,0,0,0.04))",
                  border: "1px solid var(--color-border)",
                  borderRadius: 4,
                  padding: "6px 10px",
                }}>
                  <b>No brief saved yet.</b> If you started this project via the
                  New Project wizard's AI flow, the wizard's premise + key promises
                  should land here automatically — when they don't, fill in any
                  fields and save to seed the project.
                </p>
              )}

              <fieldset style={s.fieldset}>
                <legend style={s.legend}>Brief structure</legend>
                <div style={s.gridTwo}>
                  <Field label="Genre">
                    <input style={s.input} value={form.genre}
                      onChange={e => field("genre", e.target.value)} />
                  </Field>
                  <Field label="Audience">
                    <input style={s.input} value={form.audience}
                      onChange={e => field("audience", e.target.value)} />
                  </Field>
                </div>
                <div style={s.gridTwo}>
                  <Field label="Tone (e.g. spare, propulsive, wry)">
                    <input style={s.input} value={form.tone}
                      onChange={e => field("tone", e.target.value)} />
                  </Field>
                  <Field label="Target word count" hint="5,000 – 250,000">
                    <input style={s.input} type="number"
                      min={5000} max={250000} step={500}
                      value={form.target_word_count}
                      onChange={e => field("target_word_count", e.target.value)} />
                  </Field>
                </div>
                <Field label="Premise" hint="1–3 sentences in your own register.">
                  <textarea style={{ ...s.input, minHeight: 64 }}
                    value={form.premise}
                    onChange={e => field("premise", e.target.value)} />
                </Field>
                <Field label="Title candidates"
                  hint="Separate with semicolons or commas.">
                  <input style={s.input}
                    value={form.title_suggestions.join("; ")}
                    onChange={e => listField("title_suggestions", e.target.value)} />
                </Field>
                <Field label="Key promises"
                  hint="1–6 short sentences naming what the reader will get.">
                  <textarea style={{ ...s.input, minHeight: 60 }}
                    value={form.key_promises.join("\n")}
                    onChange={e => listField("key_promises", e.target.value)} />
                </Field>
              </fieldset>

              <fieldset style={s.fieldset}>
                <legend style={s.legend}>Story uniqueness (drives the creative profile)</legend>

                <Field label="Comp titles or authors"
                  hint="Touchstones for voice + mood, not models to imitate. Examples: 'Ursula K. Le Guin', 'Station Eleven'.">
                  <input style={s.input}
                    value={form.comp_titles_or_authors.join("; ")}
                    onChange={e => listField("comp_titles_or_authors", e.target.value)} />
                </Field>

                <Field label="Theme keywords"
                  hint="Recurring obsessions to braid through every scene. Examples: 'loneliness', 'inheritance'.">
                  <input style={s.input}
                    value={form.theme_keywords.join("; ")}
                    onChange={e => listField("theme_keywords", e.target.value)} />
                </Field>

                <Field label="Forbidden tropes / patterns"
                  hint='Hard "do not use." Examples: "no chosen-one", "no AI tells like ‘tapestry’".'>
                  <input style={s.input}
                    value={form.forbidden_tropes.join("; ")}
                    onChange={e => listField("forbidden_tropes", e.target.value)} />
                </Field>

                <div style={s.gridTwo}>
                  <Field label="Era / setting anchor"
                    hint="When and where; sensory details must respect it.">
                    <input style={s.input}
                      value={form.era_setting}
                      onChange={e => field("era_setting", e.target.value)} />
                  </Field>
                  <Field label="Cultural context"
                    hint="Cultural lens that shapes voice + idiom + stakes.">
                    <input style={s.input}
                      value={form.cultural_context}
                      onChange={e => field("cultural_context", e.target.value)} />
                  </Field>
                </div>

                <Field label="Creative seed (structural angle)"
                  hint='One short phrase the drafter uses as a north-star angle. Example: "tell it backwards from the funeral".'>
                  <input style={s.input}
                    value={form.creative_seed}
                    onChange={e => field("creative_seed", e.target.value)} />
                </Field>
              </fieldset>

              {error && <div style={s.error}>{error}</div>}
              {savedHint && <div style={s.savedHint}>{savedHint}</div>}

              <div style={s.footer}>
                <button style={s.ghostBtn} onClick={onClose} disabled={saving}>Close</button>
                <button style={s.primaryBtn} onClick={handleSave} disabled={saving}>
                  {saving ? "Saving…" : "Save brief"}
                </button>
              </div>
            </>
          )}
        </div>
      </div>
    </div>
  );
}

// ── helpers ─────────────────────────────────────────────────────────────────

function asStr(v: unknown): string {
  return typeof v === "string" ? v : "";
}

function asStrArr(v: unknown): string[] {
  return Array.isArray(v) ? v.filter((x): x is string => typeof x === "string") : [];
}

function Field({ label, hint, children }: {
  label: string; hint?: string; children: React.ReactNode;
}) {
  return (
    <label style={s.field}>
      <span style={s.fieldLabel}>{label}</span>
      {children}
      {hint && <span style={s.fieldHint}>{hint}</span>}
    </label>
  );
}

const s: Record<string, React.CSSProperties> = {
  overlay:    { position: "fixed", inset: 0, background: "rgba(0,0,0,0.5)", zIndex: 200, display: "flex", alignItems: "center", justifyContent: "center" },
  dialog:     { width: "min(820px, 96vw)", maxHeight: "92vh", display: "flex", flexDirection: "column", background: "var(--color-surface)", border: "1px solid var(--color-border)", borderRadius: 8, overflow: "hidden", boxShadow: "0 20px 60px rgba(0,0,0,0.4)" },
  header:     { display: "flex", alignItems: "center", justifyContent: "space-between", padding: "12px 16px", borderBottom: "1px solid var(--color-border)" },
  close:      { background: "transparent", border: "none", fontSize: 18, cursor: "pointer", color: "inherit" },
  body:       { padding: 14, overflowY: "auto", display: "flex", flexDirection: "column", gap: 14 },
  blurb:      { margin: 0, fontSize: 13, opacity: 0.85, lineHeight: 1.5 },
  fieldset:   { border: "1px solid var(--color-border)", borderRadius: 6, padding: 12, display: "flex", flexDirection: "column", gap: 12 },
  legend:     { padding: "0 6px", fontSize: 12, fontWeight: 600, opacity: 0.8 },
  gridTwo:    { display: "grid", gridTemplateColumns: "1fr 1fr", gap: 10 },
  field:      { display: "flex", flexDirection: "column", gap: 4, fontSize: 12 },
  fieldLabel: { fontWeight: 600 },
  fieldHint:  { opacity: 0.65, fontSize: 11 },
  input:      { padding: "6px 8px", border: "1px solid var(--color-border)", borderRadius: 4, background: "var(--color-bg)", color: "inherit", fontFamily: "inherit", fontSize: 12 },
  hint:       { fontSize: 13, opacity: 0.75 },
  footer:     { display: "flex", justifyContent: "flex-end", gap: 8 },
  ghostBtn:   { padding: "6px 14px", border: "1px solid var(--color-border)", borderRadius: 4, cursor: "pointer", background: "transparent", color: "inherit" },
  primaryBtn: { padding: "6px 16px", border: "1px solid var(--color-border)", borderRadius: 4, cursor: "pointer", background: "var(--color-success-bg, #e8f5e9)", color: "var(--color-success, #2e7d32)", fontWeight: 600 },
  error:      { color: "var(--color-error, #c62828)", padding: "6px 10px", fontSize: 12, background: "var(--color-error-bg, rgba(198,40,40,0.08))", borderRadius: 4 },
  savedHint:  { color: "var(--color-success, #2e7d32)", padding: "6px 10px", fontSize: 12, background: "var(--color-success-bg, #e8f5e9)", borderRadius: 4 },
};

// ── Provenance helpers ──────────────────────────────────────────────────────

/**
 * Map an `agent_id` source string from the audit ledger to a human label.
 * Mirrors the agent_ids written by `agent_run_outline` (`"wizard"`),
 * `agent_run_intake` (`"intake"`), and `project_brief_save` (`"user-edit"`).
 */
function labelForSource(source: string): string {
  switch (source) {
    case "wizard":    return "the New Project wizard";
    case "intake":    return "the intake agent";
    case "user-edit": return "your manual edit";
    default:          return source || "unknown";
  }
}

/** Render an ISO-8601 timestamp as a short relative phrase. */
function formatLastSaved(iso: string): string {
  const t = Date.parse(iso);
  if (Number.isNaN(t)) return "recently";
  const elapsedMs = Date.now() - t;
  const minutes = Math.floor(elapsedMs / 60_000);
  if (minutes < 1) return "just now";
  if (minutes < 60) return `${minutes} min ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours} h ago`;
  const days = Math.floor(hours / 24);
  if (days < 30) return `${days} d ago`;
  return new Date(t).toLocaleDateString();
}
