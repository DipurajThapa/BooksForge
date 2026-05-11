/**
 * Export panel — Phase 6 (BACKLOG H1+H2+H3+H4+H7).
 *
 * Picks an export profile, runs it, shows the result + EPUBCheck
 * verdict (where applicable), and renders the persisted export history
 * for the open project.
 *
 * Profiles routed:
 *   - markdown                → in-process renderer (no sidecar needed)
 *   - generic_epub / kdp_ebook → in-process EPUB-3 packager + opt-in EPUBCheck
 *   - docx / trade_pdf_5x8 / trade_pdf_6x9 → Pandoc subprocess
 */
import React, { useEffect, useState } from "react";
import { save as saveDialog } from "@tauri-apps/plugin-dialog";
import type {
  ExportDependencyReport,
  ExportDependencyStatus,
  ExportHistoryEntry,
  ExportRunResult,
} from "@booksforge/shared-types";
import { ipc, type PublishingTargetRow } from "../lib/ipc";
import { useDialogA11y } from "../lib/useDialogA11y";
import { errorMessage } from "../lib/errorMessage";

interface Props {
  onClose: () => void;
}

interface Profile {
  id:        string;
  name:      string;
  ext:       string;
  blurb:     string;
  needsBin?: "pandoc";
}

interface FormatProfileOption {
  id:    string;
  name:  string;
  blurb: string;
}

interface GenreOption {
  id:         string;
  name:       string;
  subGenres:  FormatProfileOption[];
}

/**
 * Genre × Sub-genre typography catalogue.  Mirrors `FormatProfile` +
 * `Genre` in `crates/booksforge-domain/src/format_profile.rs` — keep
 * in sync.  Affects EPUB CSS + Pandoc PDF geometry / font / TOC
 * behaviour, and DOCX reference-template lookup
 * (`reference-<profile>.docx` / `reference-<genre>.docx`).
 */
const GENRES: GenreOption[] = [
  {
    id: "romance", name: "Romance",
    subGenres: [
      { id: "romance_contemporary", name: "Contemporary",        blurb: "Modern romance — Lora body + Playfair Display heads." },
      { id: "romance_historical",   name: "Historical / Regency", blurb: "Period typography — Cormorant Garamond, ornate flourish ornament." },
      { id: "romance_paranormal",   name: "Paranormal",          blurb: "Crimson Pro + Cormorant heads, moonlit ornament." },
      { id: "romance_suspense",     name: "Suspense",            blurb: "Romance pacing with thriller-adjacent typography (Lora + Inter)." },
    ],
  },
  {
    id: "comedy", name: "Comedy",
    subGenres: [
      { id: "comedy_romcom",         name: "Romantic Comedy",  blurb: "Lora + Playfair Display, soft wave ornament." },
      { id: "comedy_satire",         name: "Satire",           blurb: "Source Serif 4 + Inter — sharp, dry, literary-leaning." },
      { id: "comedy_literary_humor", name: "Literary Humor",   blurb: "Garamond throughout — comedy with literary credentials." },
      { id: "comedy_cozy",           name: "Cozy",             blurb: "Smaller trim, friendly Lora + Playfair heads." },
    ],
  },
  {
    id: "non_fiction", name: "Non-fiction",
    subGenres: [
      { id: "non_fiction_narrative", name: "Narrative",  blurb: "Long-form journalism — reads like literary fiction." },
      { id: "non_fiction_cookbook",  name: "Cookbook",   blurb: "7×9 trim, Source Sans 3 throughout, callout-friendly." },
      { id: "non_fiction_workbook",  name: "Workbook",   blurb: "Letter-size workbook with checkbox ornaments + block paragraphs." },
      { id: "non_fiction_self_help", name: "Self-help",  blurb: "Crimson Pro + Inter — action-oriented, motivational." },
    ],
  },
  {
    id: "thriller", name: "Thriller",
    subGenres: [
      { id: "thriller_psychological", name: "Psychological",  blurb: "Mass-market lean, Crimson Pro + Inter, vertical-bar break." },
      { id: "thriller_crime",         name: "Crime / Hard-boiled", blurb: "Vollkorn body, Inter heads, slash-stroke ornament." },
      { id: "thriller_espionage",     name: "Spy / Espionage", blurb: "Source Serif 4 + Inter, diamond-cluster ornament." },
      { id: "thriller_action",        name: "Action",         blurb: "Mass-market trim, Vollkorn body, vertical-bar break." },
    ],
  },
  {
    id: "horror", name: "Horror",
    subGenres: [
      { id: "horror_gothic",       name: "Gothic",        blurb: "Cormorant Garamond throughout — period-gothic mood, cross ornament." },
      { id: "horror_cosmic",       name: "Cosmic",        blurb: "Vollkorn weight + Cormorant heads, descending-triangle ornament." },
      { id: "horror_slasher",      name: "Slasher",       blurb: "Mass-market trim, Crimson Pro + Inter, jagged-stroke break." },
      { id: "horror_supernatural", name: "Supernatural",  blurb: "Vollkorn + Cormorant heads, crescent-moon ornament." },
    ],
  },
  {
    id: "generic", name: "Generic (legacy)",
    subGenres: [
      { id: "fiction_trade_standard", name: "Fiction — Trade Paperback (6×9)", blurb: "Default for novels.  Garamond, drop caps, recto chapter starts." },
      { id: "fiction_trade_mass",     name: "Fiction — Mass-Market (5×8)",     blurb: "Compact mass-market paperback.  Tighter leading, smaller body." },
      { id: "fiction_literary",       name: "Fiction — Literary (6×9)",        blurb: "More generous leading, ornament scene break (❦), refined drop caps." },
      { id: "fiction_young_adult",    name: "Young Adult (5.5×8.5)",           blurb: "Larger 12pt body, looser leading, no drop cap." },
      { id: "non_fiction_practical",  name: "Non-Fiction — Practical (6×9)",   blurb: "Sans-serif headings, block paragraphs, callouts, TOC included." },
      { id: "non_fiction_memoir",     name: "Non-Fiction — Memoir (6×9)",      blurb: "Trade-fiction feel with footnotes + photo-plate support." },
      { id: "academic",               name: "Academic (6×9)",                  blurb: "Numbered headings, narrow margins, bibliography styling, TOC." },
    ],
  },
];

/// Default sub-genre per genre (the first one in each list).
const DEFAULT_GENRE_ID = "generic";
const DEFAULT_FORMAT_PROFILE_ID = "fiction_trade_standard";

const PROFILES: Profile[] = [
  { id: "markdown",        name: "Markdown",            ext: "md",    blurb: "Plain markdown — no external dependency.  Round-trips through Word, Pages, and any GitHub preview." },
  { id: "generic_epub",    name: "EPUB-3 (generic)",    ext: "epub",  blurb: "Reflowable e-book.  In-process packager, EPUBCheck-validated when configured." },
  { id: "kdp_ebook",       name: "EPUB-3 (KDP)",        ext: "epub",  blurb: "Same packager with KDP-friendly metadata defaults.  Validate with EPUBCheck before upload." },
  { id: "docx",            name: "Word (.docx)",        ext: "docx",  blurb: "Pandoc-rendered DOCX.  Requires Pandoc 3.x on PATH.", needsBin: "pandoc" },
  { id: "trade_pdf_5x8",   name: "Trade paperback PDF (5×8)", ext: "pdf", blurb: "Pandoc → xelatex.  Requires Pandoc + a TeX install.", needsBin: "pandoc" },
  { id: "trade_pdf_6x9",   name: "Trade paperback PDF (6×9)", ext: "pdf", blurb: "Pandoc → xelatex.  Requires Pandoc + a TeX install.", needsBin: "pandoc" },
];

export default function ExportPanel({ onClose }: Props) {
  const { dialogProps, titleId } = useDialogA11y(onClose);
  const [profileId, setProfileId] = useState<string>("markdown");
  const [genreId,   setGenreId]   = useState<string>(DEFAULT_GENRE_ID);
  const [formatProfileId, setFormatProfileId] = useState<string>(DEFAULT_FORMAT_PROFILE_ID);
  const [running,   setRunning]   = useState(false);
  const [result,    setResult]    = useState<ExportRunResult | null>(null);
  const [error,     setError]     = useState<string | null>(null);
  const [history,   setHistory]   = useState<ExportHistoryEntry[]>([]);
  const [deps,      setDeps]      = useState<ExportDependencyReport | null>(null);
  const [pubTargets,    setPubTargets]    = useState<PublishingTargetRow[]>([]);
  const [pubTargetId,   setPubTargetId]   = useState<string>("kdp_paperback");

  const profile = PROFILES.find(p => p.id === profileId)!;
  // Genre typography is meaningful for EPUB, PDF, and DOCX.  Markdown
  // ignores it.
  const formatPickerVisible = ["generic_epub", "kdp_ebook", "trade_pdf_5x8", "trade_pdf_6x9", "docx"].includes(profile.id);

  const currentGenre = GENRES.find(g => g.id === genreId) ?? GENRES[0]!;
  const subGenres    = currentGenre.subGenres;
  const currentSub   = subGenres.find(s => s.id === formatProfileId) ?? subGenres[0]!;

  // When the user picks a new genre, snap the sub-genre to the first
  // entry of the new list so the selector stays consistent.
  function selectGenre(id: string) {
    setGenreId(id);
    const g = GENRES.find(x => x.id === id);
    if (g && g.subGenres[0]) setFormatProfileId(g.subGenres[0].id);
  }

  useEffect(() => {
    ipc.exportHistory().then(setHistory).catch(() => null);
    // Probe export-pipeline dependencies (Pandoc, Java, EPUBCheck JAR)
    // on mount so the user sees a missing-dep banner BEFORE clicking
    // Export and getting an opaque error. Read-only; no side effects.
    ipc.exportCheckDependencies().then(setDeps).catch(() => null);
    // Load the publishing-target catalogue (KDP Paperback / KDP Kindle /
    // IngramSpark / Apple Books / Google Books / Kobo / Shunn manuscript /
    // Generic). Each row carries the platform's compliance spec — trim
    // sizes, ISBN scheme, ToC depth cap, image DPI minimum, cover dims,
    // EPUBCheck/PDF-X requirements — surfaced in the picker briefing.
    ipc.publishingTargetsList().then(setPubTargets).catch(() => null);
  }, []);

  const pubTarget = pubTargets.find((t) => t.id === pubTargetId);

  // The top-level banner only needs the union of missing binaries.
  // Per-profile blocked state is computed inline at each radio render
  // via `(deps?.items ?? []).find(...)`.
  const missingBins: ExportDependencyStatus[] =
    (deps?.items ?? []).filter(d => !d.found);

  async function handleExport() {
    setError(null);
    setResult(null);
    const path = await saveDialog({
      title: `Export ${profile.name}`,
      defaultPath: `manuscript.${profile.ext}`,
      filters: [{ name: profile.name, extensions: [profile.ext] }],
    }).catch(() => null);
    if (!path) return;

    setRunning(true);
    try {
      // Tauri v2's `save` plugin returns `string | null`; null was caught above.
      const r = await ipc.exportRun({
        profile:        profile.id,
        output_path:    path,
        format_profile: formatProfileId,
      });
      setResult(r);
      // Refresh history after a successful run.
      ipc.exportHistory().then(setHistory).catch(() => null);
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
          <strong id={titleId}>Export manuscript</strong>
          <button style={s.close} onClick={onClose} aria-label="Close export panel">✕</button>
        </header>

        <div style={s.body}>
          {missingBins.length > 0 && (
            <section
              role="status"
              aria-live="polite"
              style={{
                background:    "var(--color-warning-bg, #fff8e1)",
                border:        "1px solid var(--color-warning-border, #f4c542)",
                borderRadius:  6,
                padding:       "10px 12px",
                marginBottom:  12,
              }}
            >
              <strong style={{ display: "block", marginBottom: 4 }}>
                Some export formats need extra software.
              </strong>
              <ul style={{ margin: "4px 0 0 0", paddingLeft: 18 }}>
                {missingBins.map((d) => (
                  <li key={d.id} style={{ marginBottom: 4 }}>
                    <strong>{d.name}</strong>{" "}
                    <span style={{ opacity: 0.65 }}>
                      (unlocks: {d.unlocks.join(", ") || "—"})
                    </span>
                    <div style={{ fontSize: "0.85em", opacity: 0.85 }}>
                      {d.install_hint}
                    </div>
                  </li>
                ))}
              </ul>
              <div style={{ fontSize: "0.85em", marginTop: 6, opacity: 0.7 }}>
                Markdown export works regardless. EPUB-3 works without Java
                (validation is skipped). Restart BooksForge after installing.
              </div>
            </section>
          )}

          <section>
            <h4 style={s.sectionTitle}>Choose profile</h4>
            <div style={s.grid}>
              {PROFILES.map(p => (
                <label
                  key={p.id}
                  style={{
                    ...s.card,
                    borderColor: p.id === profileId
                      ? "var(--color-accent, #2e7d32)"
                      : "var(--color-border)",
                  }}
                >
                  <input
                    type="radio"
                    name="profile"
                    checked={p.id === profileId}
                    onChange={() => setProfileId(p.id)}
                    style={{ marginRight: 6 }}
                  />
                  <span>
                    <span style={s.cardName}>{p.name}</span>
                    {p.needsBin && (
                      <span style={s.depTag}>needs {p.needsBin}</span>
                    )}
                    <span style={s.cardBlurb}>{p.blurb}</span>
                  </span>
                </label>
              ))}
            </div>
          </section>

          {pubTargets.length > 0 && (
            <section>
              <h4 style={s.sectionTitle}>Publishing target</h4>
              <p style={{ ...s.cardBlurb, marginTop: 0, marginBottom: 8 }}>
                Pick the storefront / distributor your book will go to. The
                target's compliance spec (ISBN scheme, ToC depth, cover dims,
                trim sizes, EPUBCheck/PDF/X requirements) is shown below — use
                it as your pre-flight checklist before uploading.
              </p>
              <select
                value={pubTargetId}
                onChange={(e) => setPubTargetId(e.target.value)}
                style={s.select}
                aria-label="Publishing target"
              >
                {pubTargets.map((t) => (
                  <option key={t.id} value={t.id}>{t.label}</option>
                ))}
              </select>
              {pubTarget && (
                <div
                  style={{
                    marginTop:    10,
                    padding:      "10px 12px",
                    border:       "1px solid var(--color-border)",
                    borderRadius: 6,
                    background:   "var(--color-surface-alt, #fafaf7)",
                    fontSize:     "0.9em",
                  }}
                >
                  <div style={{ marginBottom: 6 }}>{pubTarget.user_briefing}</div>
                  <div style={{ display: "flex", flexWrap: "wrap", gap: 6 }}>
                    {pubTarget.artifact_formats.map((f) => (
                      <span key={f} style={{
                        background:   "var(--color-accent-bg, #e8f4ff)",
                        border:       "1px solid var(--color-accent-border, #b6dcff)",
                        borderRadius: 4,
                        padding:      "2px 6px",
                        fontSize:     "0.85em",
                      }}>{f}</span>
                    ))}
                    <span style={{ opacity: 0.65 }}>·</span>
                    <span>ToC depth ≤ {pubTarget.toc_depth_max || "—"}</span>
                    {pubTarget.image_min_dpi > 0 && (
                      <>
                        <span style={{ opacity: 0.65 }}>·</span>
                        <span>image DPI ≥ {pubTarget.image_min_dpi}</span>
                      </>
                    )}
                    {pubTarget.cover_min_px[0] > 0 && (
                      <>
                        <span style={{ opacity: 0.65 }}>·</span>
                        <span>
                          cover ≥ {pubTarget.cover_min_px[0]}×{pubTarget.cover_min_px[1]}
                          {pubTarget.cover_aspect_x100 > 0 &&
                            ` (${(pubTarget.cover_aspect_x100 / 100).toFixed(2)}:1)`}
                        </span>
                      </>
                    )}
                    <span style={{ opacity: 0.65 }}>·</span>
                    <span>
                      ID:{" "}
                      {pubTarget.identifier_scheme === "urn_isbn"
                        ? "ISBN-13 required"
                        : pubTarget.identifier_scheme === "urn_isbn_preferred"
                          ? "ISBN-13 preferred"
                          : "no ISBN required"}
                    </span>
                    {pubTarget.fonts_embedded_required && (
                      <>
                        <span style={{ opacity: 0.65 }}>·</span>
                        <span>fonts embedded</span>
                      </>
                    )}
                    {pubTarget.pdfx_required && (
                      <>
                        <span style={{ opacity: 0.65 }}>·</span>
                        <span>PDF/X-1a</span>
                      </>
                    )}
                    {pubTarget.epubcheck_required && (
                      <>
                        <span style={{ opacity: 0.65 }}>·</span>
                        <span>EPUBCheck pass</span>
                      </>
                    )}
                    {pubTarget.accessibility_required && (
                      <>
                        <span style={{ opacity: 0.65 }}>·</span>
                        <span>a11y metadata</span>
                      </>
                    )}
                  </div>
                  {pubTarget.allowed_trims.length > 0 && (
                    <div style={{ marginTop: 6, opacity: 0.85 }}>
                      Allowed trims:{" "}
                      {pubTarget.allowed_trims.map((t) => t.label).join(" · ")}
                    </div>
                  )}
                </div>
              )}
            </section>
          )}

          {formatPickerVisible && (
            <section>
              <h4 style={s.sectionTitle}>Genre / typography</h4>
              <div style={s.twoLevel}>
                <label style={s.twoLevelLabel}>
                  <span style={s.twoLevelLabelText}>Genre</span>
                  <select
                    value={genreId}
                    onChange={e => selectGenre(e.target.value)}
                    style={s.select}
                  >
                    {GENRES.map(g => (
                      <option key={g.id} value={g.id}>{g.name}</option>
                    ))}
                  </select>
                </label>
                <label style={s.twoLevelLabel}>
                  <span style={s.twoLevelLabelText}>Sub-genre</span>
                  <select
                    value={formatProfileId}
                    onChange={e => setFormatProfileId(e.target.value)}
                    style={s.select}
                  >
                    {subGenres.map(sg => (
                      <option key={sg.id} value={sg.id}>{sg.name}</option>
                    ))}
                  </select>
                </label>
              </div>
              <div style={s.formatBlurb}>{currentSub.blurb}</div>
            </section>
          )}

          <button
            style={s.runBtn}
            onClick={handleExport}
            disabled={running}
          >
            {running ? "Exporting…" : `Export as ${profile.name}`}
          </button>

          {error && <div style={s.error} role="alert">{error}</div>}

          {result && (
            <div style={s.resultBox} role="status" aria-live="polite">
              <div style={s.resultLine}>
                <strong>Done.</strong> Wrote {result.bytes.toLocaleString()} bytes
                {" "}<code>{shortHash(result.hash)}</code> to
                {" "}<code>{result.output_path}</code>
              </div>
              {result.validation_message && (
                <div style={{
                  ...s.validationLine,
                  color: result.validation_ok
                    ? "var(--color-success, #2e7d32)"
                    : "var(--color-error, #c62828)",
                }}>
                  {result.validation_message}
                </div>
              )}
              {(result.error_count > 0 || result.warning_count > 0) && (
                <div style={s.counts}>
                  errors: {result.error_count} · warnings: {result.warning_count}
                </div>
              )}
            </div>
          )}

          <section>
            <h4 style={s.sectionTitle}>History</h4>
            {history.length === 0 ? (
              <div style={s.empty}>No exports yet.</div>
            ) : (
              <ul style={s.history}>
                {history.map(h => (
                  <li key={h.id} style={s.historyRow}>
                    <span style={s.histProfile}>{h.profile}</span>
                    <span style={s.histPath} title={h.output_path}>{h.output_path}</span>
                    <code style={s.histHash}>{shortHash(h.hash)}</code>
                    <span style={s.histDate}>{formatDate(h.created_at)}</span>
                  </li>
                ))}
              </ul>
            )}
          </section>
        </div>
      </div>
    </div>
  );
}

function shortHash(s: string): string {
  return s.length >= 8 ? `${s.slice(0, 8)}…` : s;
}

function formatDate(iso: string): string {
  try {
    const d = new Date(iso);
    return d.toLocaleString(undefined, { dateStyle: "short", timeStyle: "short" });
  } catch { return iso; }
}

const s: Record<string, React.CSSProperties> = {
  overlay:  { position: "fixed", inset: 0, background: "rgba(0,0,0,0.4)", zIndex: 50, display: "flex", alignItems: "center", justifyContent: "center" },
  dialog:   { width: "min(820px, 94vw)", maxHeight: "90vh", display: "flex", flexDirection: "column", background: "var(--color-surface)", border: "1px solid var(--color-border)", borderRadius: 6, overflow: "hidden" },
  header:   { display: "flex", alignItems: "center", justifyContent: "space-between", padding: "10px 14px", borderBottom: "1px solid var(--color-border)" },
  close:    { background: "transparent", border: "none", fontSize: 18, cursor: "pointer", color: "inherit" },
  body:     { padding: "12px 14px", overflowY: "auto", flex: 1, display: "flex", flexDirection: "column", gap: 16 },
  sectionTitle: { fontSize: 13, fontWeight: 600, margin: "0 0 6px 0" },
  grid:     { display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(280px, 1fr))", gap: 8 },
  card:     { display: "flex", alignItems: "flex-start", padding: 10, border: "1px solid var(--color-border)", borderRadius: 4, cursor: "pointer", background: "var(--color-bg)" },
  cardName: { fontWeight: 600, fontSize: 13, display: "block" },
  depTag:   { display: "inline-block", marginLeft: 6, fontSize: 10, padding: "1px 5px", borderRadius: 3, background: "var(--color-warn-bg, rgba(249,168,37,0.15))", color: "var(--color-warn, #f9a825)" },
  cardBlurb:{ fontSize: 12, opacity: 0.8, display: "block", marginTop: 4 },
  runBtn:   { alignSelf: "flex-start", padding: "8px 16px", border: "1px solid var(--color-border)", borderRadius: 4, cursor: "pointer", background: "var(--color-bg)", fontWeight: 600 },
  error:    { color: "var(--color-error, #c62828)", padding: 8, border: "1px solid var(--color-error, #c62828)", borderRadius: 4 },
  resultBox: { padding: 10, border: "1px solid var(--color-border)", borderRadius: 4, background: "var(--color-bg)", fontSize: 13, display: "flex", flexDirection: "column", gap: 6 },
  resultLine: {},
  validationLine: { fontSize: 12 },
  counts:   { fontSize: 12, opacity: 0.85 },
  empty:    { fontSize: 13, fontStyle: "italic", opacity: 0.7 },
  history:  { listStyle: "none", padding: 0, margin: 0, display: "flex", flexDirection: "column", gap: 4 },
  historyRow: { display: "grid", gridTemplateColumns: "100px 1fr 90px 140px", alignItems: "baseline", gap: 8, fontSize: 12, padding: "4px 0", borderBottom: "1px dashed var(--color-border)" },
  histProfile: { fontWeight: 600 },
  histPath:    { overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" },
  histHash:    { opacity: 0.6 },
  histDate:    { opacity: 0.7, textAlign: "right" },
  select:      { padding: "6px 8px", border: "1px solid var(--color-border)", borderRadius: 4, background: "var(--color-bg)", color: "inherit", minWidth: 200 },
  formatBlurb: { fontSize: 12, opacity: 0.75, marginTop: 6 },
  twoLevel:    { display: "flex", gap: 12, alignItems: "flex-end", flexWrap: "wrap" },
  twoLevelLabel: { display: "flex", flexDirection: "column", gap: 4 },
  twoLevelLabelText: { fontSize: 11, opacity: 0.7, textTransform: "uppercase", letterSpacing: "0.05em" },
};
