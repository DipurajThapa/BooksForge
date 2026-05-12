/**
 * Prepare-for-Publishing panel — Phase 7 of `PRODUCT_ROADMAP_E2E.md`
 * (closes UX recommendation R4 from the audit).
 *
 * Single-action workflow that bundles per-platform packages
 * (KDP / Google Play / Apple Books) under
 * `<bundle>/exports/<platform>/`. The user fills in optional metadata
 * overrides (placeholders flagged inline if left blank), clicks
 * "Prepare packages", and the backend produces:
 *   - manuscript.epub  (every platform)
 *   - manuscript.pdf   (KDP + Google Play; Apple skips)
 *   - metadata.{kdp.csv | gp.json | apple.json}
 *   - cover_brief.md   (HUMAN_REQUIRED to commission art)
 *   - READY_TO_UPLOAD.md
 *   - readiness.json   (per-item PASS / WARN / FAIL / HUMAN_REQUIRED)
 *
 * The panel renders the per-platform readiness grid + a one-click
 * "Open folder" link to the platform output directory.
 */
import React, { useState } from "react";
import type {
  PrepareForPublishingResult,
  PlatformReadiness,
  PublishingMetadata,
  ReadinessItem,
} from "@booksforge/shared-types";
import { useDialogA11y } from "../lib/useDialogA11y";
import { ipc } from "../lib/ipc";
import { errorMessage } from "../lib/errorMessage";
import Term from "./Term";

interface Props {
  onClose: () => void;
}

interface PlatformChoice {
  id:    "kdp" | "google_play" | "apple_books";
  label: string;
  blurb: string;
}

const PLATFORM_CHOICES: PlatformChoice[] = [
  { id: "kdp",         label: "Amazon Kindle store", blurb: "Kindle eBook + paperback. Largest single retailer." },
  { id: "google_play", label: "Google Play Books",   blurb: "Worldwide preview/sample marketplace; accepts ebook + print PDF." },
  { id: "apple_books", label: "Apple Books",         blurb: "Apple's eBook store — strict file validation, gated on a clean validator pass." },
];

const STATUS_ICON: Record<string, string> = {
  PASS:           "✓",
  WARN:           "⚠",
  FAIL:           "✗",
  HUMAN_REQUIRED: "👤",
};

const STATUS_COLOR: Record<string, string> = {
  PASS:           "var(--color-success, #2e7d32)",
  WARN:           "var(--color-amber-500, #f59e0b)",
  FAIL:           "var(--color-error, #ef4444)",
  HUMAN_REQUIRED: "var(--color-blue-500, #3b82f6)",
};

export default function PrepareForPublishingPanel({ onClose }: Props) {
  const { dialogProps, titleId } = useDialogA11y(onClose);
  const [selected, setSelected] = useState<Record<string, boolean>>({
    kdp: true, google_play: true, apple_books: true,
  });
  const [meta, setMeta] = useState<PublishingMetadata>({
    subtitle:          null,
    description:       null,
    short_description: null,
    keywords:          null,
    bisac_codes:       null,
    age_range:         null,
    language:          null,
    isbn:              null,
    price_usd:         null,
    publication_date:  null,
    publisher:         null,
    rights_statement:  null,
  });
  const [running, setRunning] = useState(false);
  const [error, setError]     = useState<string | null>(null);
  const [result, setResult]   = useState<PrepareForPublishingResult | null>(null);

  function setField<K extends keyof PublishingMetadata>(key: K, value: PublishingMetadata[K]) {
    setMeta(prev => ({ ...prev, [key]: value }));
  }

  function setListField(
    key: "keywords" | "bisac_codes",
    raw:  string,
  ) {
    const arr = raw.split(/[;,\n]+/).map(s => s.trim()).filter(Boolean);
    setField(key, arr.length === 0 ? null : arr);
  }

  async function handleRun() {
    setRunning(true);
    setError(null);
    setResult(null);
    try {
      const platforms = PLATFORM_CHOICES.filter(c => selected[c.id]).map(c => c.id);
      const r = await ipc.prepareForPublishing({
        platforms,
        metadata_overrides: meta,
      });
      setResult(r);
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setRunning(false);
    }
  }

  return (
    <div style={s.overlay} role="presentation">
      <div {...dialogProps} style={s.dialog}>
        <header style={s.header}>
          <strong id={titleId}>Prepare for Publishing</strong>
          <button style={s.close} onClick={onClose} aria-label="Close panel">✕</button>
        </header>

        <div style={s.body}>
          {!result && (
            <>
              <p style={s.blurb}>
                One click bundles every file each marketplace needs into a per-platform folder
                under <code>exports/&lt;platform&gt;/</code>. You'll get the {" "}
                <Term k="EPUB">eBook</Term>, the print {" "}
                <Term k="PDF">PDF</Term> (where applicable), a metadata file, a cover brief,
                and a step-by-step <code>READY_TO_UPLOAD.md</code> walkthrough for each store.
                Anything left blank below is flagged <code>[PLACEHOLDER]</code> so you can
                see exactly what's still required.
              </p>

              {/* ── Platforms ── */}
              <fieldset style={s.fieldset}>
                <legend style={s.legend}>Marketplaces</legend>
                <div style={s.platformRow}>
                  {PLATFORM_CHOICES.map(c => (
                    <label key={c.id} style={{
                      ...s.platformCard,
                      ...(selected[c.id] ? s.platformCardSelected : {}),
                    }}>
                      <input
                        type="checkbox"
                        checked={!!selected[c.id]}
                        onChange={e => setSelected(prev => ({ ...prev, [c.id]: e.target.checked }))}
                        style={s.checkbox}
                      />
                      <div>
                        <strong>{c.label}</strong>
                        <div style={s.platformBlurb}>{c.blurb}</div>
                      </div>
                    </label>
                  ))}
                </div>
              </fieldset>

              {/* ── Metadata overrides ── */}
              <fieldset style={s.fieldset}>
                <legend style={s.legend}>Book details (optional — anything left blank is flagged for you to fill in later)</legend>

                <div style={s.gridTwo}>
                  <Field label="Subtitle" hint="Shown alongside the title on retailer pages.">
                    <input style={s.input} value={meta.subtitle ?? ""} onChange={e => setField("subtitle", e.target.value || null)} />
                  </Field>
                  <Field label="Language" hint="Two-letter language code: en for English, es for Spanish, fr for French.">
                    <input style={s.input} value={meta.language ?? ""} onChange={e => setField("language", e.target.value || null)} />
                  </Field>
                </div>

                <Field
                  label="Long description"
                  hint="200–4 000 characters. The full back-cover blurb readers see on the book's product page."
                >
                  <textarea style={{ ...s.input, minHeight: 80 }}
                    value={meta.description ?? ""}
                    onChange={e => setField("description", e.target.value || null)}
                  />
                </Field>

                <Field
                  label="Short description"
                  hint="≤ 250 characters. The one-line teaser that appears in search snippets."
                >
                  <textarea style={{ ...s.input, minHeight: 50 }}
                    value={meta.short_description ?? ""}
                    onChange={e => setField("short_description", e.target.value || null)}
                  />
                </Field>

                <div style={s.gridTwo}>
                  <Field
                    label="Search keywords"
                    hint="7+ recommended for Amazon search. Separate with semicolons or commas."
                  >
                    <input style={s.input}
                      value={(meta.keywords ?? []).join("; ")}
                      onChange={e => setListField("keywords", e.target.value)}
                    />
                  </Field>
                  <Field
                    label="Subject codes"
                    hint='Industry shelving codes (BISAC). Example: "FIC009000 FICTION / Fantasy / General". 1–3 codes is normal.'
                  >
                    <input style={s.input}
                      value={(meta.bisac_codes ?? []).join("; ")}
                      onChange={e => setListField("bisac_codes", e.target.value)}
                    />
                  </Field>
                </div>

                <div style={s.gridThree}>
                  <Field label="Age range" hint="Required by Apple Books (e.g. 18+, 13–17, 9–12).">
                    <input style={s.input} value={meta.age_range ?? ""} onChange={e => setField("age_range", e.target.value || null)} />
                  </Field>
                  <Field label="ISBN" hint="Optional. KDP can mint a free Amazon ISBN at upload.">
                    <input style={s.input} value={meta.isbn ?? ""} onChange={e => setField("isbn", e.target.value || null)} />
                  </Field>
                  <Field label="Price (USD)" hint="Just the number, e.g. 4.99. KDP 70% royalty band starts at $2.99.">
                    <input style={s.input} value={meta.price_usd ?? ""} onChange={e => setField("price_usd", e.target.value || null)} />
                  </Field>
                </div>

                <div style={s.gridThree}>
                  <Field label="Publication date" hint="YYYY-MM-DD.">
                    <input style={s.input} value={meta.publication_date ?? ""} onChange={e => setField("publication_date", e.target.value || null)} />
                  </Field>
                  <Field label="Publisher / imprint" hint="Self-publishers can use their own imprint name.">
                    <input style={s.input} value={meta.publisher ?? ""} onChange={e => setField("publisher", e.target.value || null)} />
                  </Field>
                  <Field label="Rights statement" hint="e.g. © 2026 Author Name. All rights reserved.">
                    <input style={s.input} value={meta.rights_statement ?? ""} onChange={e => setField("rights_statement", e.target.value || null)} />
                  </Field>
                </div>
              </fieldset>

              {error && <div style={s.error}>{error}</div>}

              <div style={s.footer}>
                <button style={s.ghostBtn} onClick={onClose} disabled={running}>Cancel</button>
                <button
                  style={s.primaryBtn}
                  onClick={handleRun}
                  disabled={running || PLATFORM_CHOICES.every(c => !selected[c.id])}
                >
                  {running ? "Preparing packages…" : "Prepare packages"}
                </button>
              </div>
            </>
          )}

          {result && (
            <ResultView result={result} onReset={() => setResult(null)} onClose={onClose} />
          )}
        </div>
      </div>
    </div>
  );
}

// ── Sub-components ──────────────────────────────────────────────────────────

function Field({ label, hint, children }: { label: string; hint?: string; children: React.ReactNode }) {
  return (
    <label style={s.field}>
      <span style={s.fieldLabel}>{label}</span>
      {children}
      {hint && <span style={s.fieldHint}>{hint}</span>}
    </label>
  );
}

function ResultView({
  result, onReset, onClose,
}: {
  result:  PrepareForPublishingResult;
  onReset: () => void;
  onClose: () => void;
}) {
  return (
    <div style={s.results}>
      <div style={s.resultHeader}>
        <strong>Packages ready in {result.elapsed_s.toFixed(1)} s</strong>
        <span style={s.resultSub}>
          Project <code>{result.project_id}</code>
        </span>
      </div>

      {result.platforms.map(p => <PlatformBlock key={p.platform} p={p} />)}

      <div style={s.footer}>
        <button style={s.ghostBtn} onClick={onReset}>Run again with different fields</button>
        <button style={s.primaryBtn} onClick={onClose}>Done</button>
      </div>
    </div>
  );
}

function PlatformBlock({ p }: { p: PlatformReadiness }) {
  const platLabel = p.platform === "kdp" ? "Amazon KDP"
                  : p.platform === "google_play" ? "Google Play Books"
                  : p.platform === "apple_books" ? "Apple Books"
                  : p.platform;
  const failures = p.items.filter(i => i.status === "FAIL").length;
  const humans   = p.items.filter(i => i.status === "HUMAN_REQUIRED").length;

  return (
    <div style={s.platformBlock}>
      <div style={s.platformHeader}>
        <strong>{platLabel}</strong>
        <span style={{
          ...s.uploadable,
          color: p.uploadable ? STATUS_COLOR.PASS : STATUS_COLOR.FAIL,
        }}>
          {p.uploadable ? "Uploadable" : `${failures} blocker${failures === 1 ? "" : "s"}`}
          {humans > 0 && ` · ${humans} human-required step${humans === 1 ? "" : "s"}`}
        </span>
      </div>
      <div style={s.outputDir}>
        <code>{p.output_dir}</code>
      </div>
      <ul style={s.itemList}>
        {p.items.map(i => <ItemRow key={i.id} item={i} />)}
      </ul>
    </div>
  );
}

function ItemRow({ item }: { item: ReadinessItem }) {
  const color = STATUS_COLOR[item.status] ?? "currentColor";
  const icon  = STATUS_ICON[item.status] ?? "?";
  return (
    <li style={s.itemRow}>
      <span style={{ ...s.itemIcon, color }}>{icon}</span>
      <span style={s.itemLabel}><strong>{item.label}</strong></span>
      <span style={s.itemDetail}>{item.detail}</span>
    </li>
  );
}

// ── Styles ──────────────────────────────────────────────────────────────────

const s: Record<string, React.CSSProperties> = {
  overlay:    { position: "fixed", inset: 0, background: "rgba(0,0,0,0.5)", zIndex: 200, display: "flex", alignItems: "center", justifyContent: "center" },
  dialog:     { width: "min(900px, 96vw)", maxHeight: "92vh", display: "flex", flexDirection: "column", background: "var(--color-surface)", border: "1px solid var(--color-border)", borderRadius: 8, overflow: "hidden", boxShadow: "0 20px 60px rgba(0,0,0,0.4)" },
  header:     { display: "flex", alignItems: "center", justifyContent: "space-between", padding: "12px 16px", borderBottom: "1px solid var(--color-border)" },
  close:      { background: "transparent", border: "none", fontSize: 18, cursor: "pointer", color: "inherit" },
  body:       { padding: 14, overflowY: "auto", display: "flex", flexDirection: "column", gap: 16 },
  blurb:      { margin: 0, fontSize: 13, opacity: 0.85, lineHeight: 1.5 },
  fieldset:   { border: "1px solid var(--color-border)", borderRadius: 6, padding: 12, display: "flex", flexDirection: "column", gap: 12 },
  legend:     { padding: "0 6px", fontSize: 12, fontWeight: 600, opacity: 0.8 },
  platformRow:{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 8 },
  platformCard:{ display: "flex", gap: 8, padding: 10, border: "2px solid var(--color-border)", borderRadius: 6, cursor: "pointer", alignItems: "flex-start" },
  platformCardSelected: { borderColor: "var(--color-success, #2e7d32)", background: "var(--color-success-bg, rgba(46,125,50,0.06))" },
  platformBlurb:{ fontSize: 11, opacity: 0.75, marginTop: 4, lineHeight: 1.4 },
  checkbox:   { marginTop: 3 },
  gridTwo:    { display: "grid", gridTemplateColumns: "1fr 1fr", gap: 10 },
  gridThree:  { display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 10 },
  field:      { display: "flex", flexDirection: "column", gap: 4, fontSize: 12 },
  fieldLabel: { fontWeight: 600 },
  fieldHint:  { opacity: 0.65, fontSize: 11 },
  input:      { padding: "6px 8px", border: "1px solid var(--color-border)", borderRadius: 4, background: "var(--color-bg)", color: "inherit", fontFamily: "inherit", fontSize: 12 },
  footer:     { display: "flex", justifyContent: "flex-end", gap: 8 },
  ghostBtn:   { padding: "6px 14px", border: "1px solid var(--color-border)", borderRadius: 4, cursor: "pointer", background: "transparent", color: "inherit" },
  primaryBtn: { padding: "6px 16px", border: "1px solid var(--color-border)", borderRadius: 4, cursor: "pointer", background: "var(--color-success-bg, #e8f5e9)", color: "var(--color-success, #2e7d32)", fontWeight: 600 },
  error:      { color: "var(--color-error, #c62828)", padding: "6px 10px", fontSize: 12, background: "var(--color-error-bg, rgba(198,40,40,0.08))", borderRadius: 4 },

  results:    { display: "flex", flexDirection: "column", gap: 14 },
  resultHeader:{ display: "flex", alignItems: "baseline", gap: 12 },
  resultSub:  { opacity: 0.7, fontSize: 12 },
  platformBlock:{ border: "1px solid var(--color-border)", borderRadius: 6, padding: 12, display: "flex", flexDirection: "column", gap: 6 },
  platformHeader:{ display: "flex", justifyContent: "space-between", alignItems: "center" },
  uploadable: { fontSize: 12, fontWeight: 600 },
  outputDir:  { fontSize: 11, opacity: 0.75, fontFamily: "var(--font-mono, monospace)", wordBreak: "break-all" },
  itemList:   { listStyle: "none", margin: 0, padding: 0, display: "flex", flexDirection: "column", gap: 4 },
  itemRow:    { display: "grid", gridTemplateColumns: "20px 1fr", gap: 6, alignItems: "baseline", fontSize: 12 },
  itemIcon:   { textAlign: "center", fontSize: 14 },
  itemLabel:  { },
  itemDetail: { opacity: 0.75, gridColumn: "2", fontSize: 11, lineHeight: 1.45, wordBreak: "break-word" },
};
