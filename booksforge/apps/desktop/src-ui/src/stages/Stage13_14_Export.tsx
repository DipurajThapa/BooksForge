/**
 * Stage 6 — Format & Ship (Phase B Step 5 + Phase C cover/boilerplate).
 *
 * Wraps three IPC paths:
 *   - `publishing_targets_list` → per-platform metadata (KDP, Apple,
 *     Google Play, IngramSpark) including trim, cover requirements,
 *     EPUBCheck flag, etc.
 *   - `prepare_for_publishing`  → builds per-platform packages under
 *     `<bundle>/exports/<platform>/` with a readiness checklist per
 *     item (PASS / WARN / FAIL / HUMAN_REQUIRED).
 *   - Cover & boilerplate (Phase C):
 *     `cover_load` / `cover_import` / `cover_remove` for cover slots,
 *     `boilerplate_load` / `boilerplate_save` for front/back-matter pages.
 *
 * The simpler `export_run` (Markdown / DOCX / PDF straight from the
 * manuscript) is also surfaced as a quick-export row at the top.
 */
import { useEffect, useState } from "react";
import { open as openDialog, save as saveDialog } from "@tauri-apps/plugin-dialog";
import type {
  BoilerplatePageDto,
  CoverAssetDto,
  CoverSetDto,
  NodeInfo,
  OpenProjectResult,
  PlatformReadiness,
  PrepareForPublishingResult,
  PublishingMetadata,
} from "@booksforge/shared-types";
import { ipc, type PublishingTargetRow } from "../lib/ipc";
import { errorMessage } from "../lib/errorMessage";

// ── Boilerplate kinds (mirrors `booksforge_domain::BoilerplateKind`).
// CI parity: a Rust test (`booksforge-domain::tests::boilerplate_ui_parity`)
// fails if a Rust variant is missing here or its `front` flag disagrees
// with `BoilerplateKind::is_front_matter()`. Update this list whenever
// the enum changes.
const BOILERPLATE_KINDS: Array<{
  id: string; label: string; front: boolean;
}> = [
  { id: "title_page",       label: "Title page",        front: true  },
  { id: "copyright",        label: "Copyright",         front: true  },
  { id: "dedication",       label: "Dedication",        front: true  },
  { id: "epigraph",         label: "Epigraph",          front: true  },
  { id: "foreword",         label: "Foreword",          front: true  },
  { id: "preface",          label: "Preface",           front: true  },
  { id: "acknowledgments",  label: "Acknowledgments",   front: false },
  { id: "about_author",     label: "About the author",  front: false },
  { id: "also_by",          label: "Also by",           front: false },
  { id: "back_cover_blurb", label: "Back-cover blurb",  front: false },
  { id: "other",            label: "Other",             front: false },
];

const COVER_SLOTS = ["front", "back", "spine"] as const;
type CoverSlot = typeof COVER_SLOTS[number];

const COVER_SLOT_LABEL: Record<CoverSlot, string> = {
  front: "Front cover",
  back:  "Back cover (paperback)",
  spine: "Spine (paperback)",
};

interface Props {
  project:    OpenProjectResult;
  onChanged?: () => void;
}

const PLATFORMS = ["kdp", "google_play", "apple_books"] as const;
type PlatformId = (typeof PLATFORMS)[number];

const PLATFORM_LABELS: Record<PlatformId, string> = {
  kdp:          "Amazon KDP",
  google_play:  "Google Play Books",
  apple_books:  "Apple Books",
};

export default function Stage13_14_Export({ project, onChanged }: Props) {
  void onChanged;  // Stage 6 doesn't change cross-stage state yet
  const [targets,     setTargets]     = useState<PublishingTargetRow[]>([]);
  const [nodes,       setNodes]       = useState<NodeInfo[]>([]);
  const [selected,    setSelected]    = useState<Set<PlatformId>>(new Set(PLATFORMS));
  const [loading,     setLoading]     = useState(true);

  const [running,     setRunning]     = useState(false);
  const [result,      setResult]      = useState<PrepareForPublishingResult | null>(null);
  const [error,       setError]       = useState<string | null>(null);

  const [mdExporting, setMdExporting] = useState(false);
  const [mdResult,    setMdResult]    = useState<string | null>(null);

  // Phase C — cover & boilerplate state.
  const [coverSet,       setCoverSet]       = useState<CoverSetDto | null>(null);
  const [coverBusy,      setCoverBusy]      = useState<CoverSlot | null>(null);
  const [coverError,     setCoverError]     = useState<string | null>(null);
  const [boilerplate,    setBoilerplate]    = useState<BoilerplatePageDto[]>([]);
  const [boilerplateSavedAt, setBoilerplateSavedAt] = useState<string | null>(null);
  const [boilerplateSaving,  setBoilerplateSaving]  = useState(false);
  const [boilerplateError,   setBoilerplateError]   = useState<string | null>(null);

  useEffect(() => {
    Promise.all([
      ipc.publishingTargetsList(),
      ipc.nodeList(),
      ipc.coverLoad().catch(() => ({ front: null, back: null, spine: null }) as CoverSetDto),
      ipc.boilerplateLoad().catch(() => [] as BoilerplatePageDto[]),
    ])
      .then(([t, n, cs, bp]) => {
        setTargets(t);
        setNodes(n);
        setCoverSet(cs);
        setBoilerplate(bp);
      })
      .catch((e) => setError(errorMessage(e)))
      .finally(() => setLoading(false));
  }, []);

  const sceneCount   = nodes.filter((n) => n.kind === "scene").length;
  const draftedScenes = nodes.filter(
    (n) => n.kind === "scene" && n.word_count > 0,
  ).length;
  const totalWords = nodes
    .filter((n) => n.kind === "scene")
    .reduce((sum, n) => sum + (n.word_count ?? 0), 0);

  function togglePlatform(id: PlatformId) {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }

  async function quickExportMarkdown() {
    setMdExporting(true); setMdResult(null);
    const safe = project.title.trim().replace(/[^a-zA-Z0-9_\- ]/g, "") || "manuscript";
    const target = await saveDialog({
      title:       "Export manuscript as Markdown",
      defaultPath: `${safe}.md`,
      filters:     [{ name: "Markdown", extensions: ["md"] }],
    }).catch(() => null);
    if (!target) { setMdExporting(false); return; }
    try {
      const r = await ipc.exportMarkdown({ output_path: target });
      setMdResult(`Exported ${r.scene_count} scenes · ${r.word_count.toLocaleString()} words → ${target}`);
    } catch (e) {
      setMdResult(`Failed: ${errorMessage(e)}`);
    } finally {
      setMdExporting(false);
    }
  }

  // ── Cover handlers ─────────────────────────────────────────────────────

  async function importCover(slot: CoverSlot) {
    setCoverError(null);
    const picked = await openDialog({
      title:    `Import ${COVER_SLOT_LABEL[slot].toLowerCase()}`,
      multiple: false,
      filters:  [{ name: "Images", extensions: ["jpg", "jpeg", "png", "webp", "tif", "tiff"] }],
    }).catch(() => null);
    if (!picked || typeof picked !== "string") return;
    setCoverBusy(slot);
    try {
      const next = await ipc.coverImport({ source_path: picked, slot });
      setCoverSet(next);
    } catch (e) {
      setCoverError(errorMessage(e));
    } finally {
      setCoverBusy(null);
    }
  }

  async function removeCover(slot: CoverSlot) {
    if (!window.confirm(`Clear the ${COVER_SLOT_LABEL[slot].toLowerCase()} slot? The image file in the bundle's assets folder is preserved.`)) return;
    setCoverBusy(slot); setCoverError(null);
    try {
      const next = await ipc.coverRemove({ slot });
      setCoverSet(next);
    } catch (e) {
      setCoverError(errorMessage(e));
    } finally {
      setCoverBusy(null);
    }
  }

  // ── Boilerplate handlers ───────────────────────────────────────────────

  function addBoilerplatePage(kindId: string) {
    const meta = BOILERPLATE_KINDS.find((k) => k.id === kindId);
    if (!meta) return;
    // Order: append at the end of its half (front vs. back matter).
    const sameHalf = boilerplate.filter((p) => {
      const m = BOILERPLATE_KINDS.find((k) => k.id === p.kind);
      return m ? m.front === meta.front : false;
    });
    const nextOrder = sameHalf.length === 0
      ? (meta.front ? 0 : 100)
      : Math.max(...sameHalf.map((p) => p.order)) + 1;
    const id = `${kindId}-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
    const newPage: BoilerplatePageDto = {
      id,
      kind: kindId,
      title: meta.label,
      body_md: "",
      order: nextOrder,
      include_in_export: true,
    };
    setBoilerplate((prev) => [...prev, newPage].sort((a, b) => a.order - b.order));
    setBoilerplateSavedAt(null);
  }

  function updateBoilerplate(id: string, patch: Partial<BoilerplatePageDto>) {
    setBoilerplate((prev) => prev.map((p) => p.id === id ? { ...p, ...patch } : p));
    setBoilerplateSavedAt(null);
  }

  function removeBoilerplate(id: string) {
    const target = boilerplate.find((p) => p.id === id);
    if (target && target.body_md.trim().length > 0) {
      if (!window.confirm(`Remove "${target.title}"? Its body text will be lost.`)) return;
    }
    setBoilerplate((prev) => prev.filter((p) => p.id !== id));
    setBoilerplateSavedAt(null);
  }

  function moveBoilerplate(id: string, direction: -1 | 1) {
    setBoilerplate((prev) => {
      const idx = prev.findIndex((p) => p.id === id);
      if (idx < 0) return prev;
      const swapWith = idx + direction;
      if (swapWith < 0 || swapWith >= prev.length) return prev;
      const next = [...prev];
      const a = next[idx];
      const b = next[swapWith];
      if (!a || !b) return prev;
      // Swap their orders.
      const orderA = a.order;
      next[idx] = { ...b, order: orderA };
      next[swapWith] = { ...a, order: b.order };
      return next.sort((p1, p2) => p1.order - p2.order);
    });
    setBoilerplateSavedAt(null);
  }

  async function saveBoilerplate() {
    setBoilerplateSaving(true); setBoilerplateError(null); setBoilerplateSavedAt(null);
    try {
      const r = await ipc.boilerplateSave({ pages: boilerplate });
      setBoilerplateSavedAt(`Saved ${r.saved_count} page${r.saved_count === 1 ? "" : "s"}.`);
    } catch (e) {
      setBoilerplateError(errorMessage(e));
    } finally {
      setBoilerplateSaving(false);
    }
  }

  async function prepareForPublishing() {
    setRunning(true); setError(null); setResult(null);
    try {
      const metadata: PublishingMetadata = {
        subtitle:          null,
        description:       null,
        short_description: null,
        keywords:          null,
        bisac_codes:       null,
        age_range:         null,
        language:          null,  // defaults to en-US from manifest
        isbn:              null,
        price_usd:         null,
        publication_date:  null,
        publisher:         null,
        rights_statement:  null,
      };
      const r = await ipc.prepareForPublishing({
        platforms: Array.from(selected),
        metadata_overrides: metadata,
      });
      setResult(r);
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setRunning(false);
    }
  }

  // ── Render ──────────────────────────────────────────────────────────────

  if (loading) {
    return (
      <div style={s.root}>
        <div style={s.col}>
          <Header />
          <p style={s.muted}>Loading publishing targets…</p>
        </div>
      </div>
    );
  }

  return (
    <div style={s.root}>
      <div style={s.col}>
        <Header />

        {/* Status snapshot */}
        <section style={s.section}>
          <header style={s.sectionHeader}>
            <h2 style={s.sectionTitle}>Manuscript snapshot</h2>
          </header>
          <div style={s.statRow}>
            <Stat label="Scenes" value={sceneCount.toString()} />
            <Stat label="Drafted" value={`${draftedScenes} / ${sceneCount}`} />
            <Stat label="Word count" value={totalWords.toLocaleString()} />
          </div>
          {draftedScenes < sceneCount && (
            <div style={{ ...s.sectionBody, paddingTop: 0 }}>
              <div style={s.bannerNote}>
                {sceneCount - draftedScenes} {sceneCount - draftedScenes === 1 ? "scene is" : "scenes are"} still
                empty. Quick-export to Markdown works regardless;
                publishing packages will warn about empty content.
              </div>
            </div>
          )}
        </section>

        {/* Quick export */}
        <section style={s.section}>
          <header style={s.sectionHeader}>
            <h2 style={s.sectionTitle}>Quick export</h2>
            <p style={s.sectionHint}>
              Plain manuscript output. Useful for human review, editor
              hand-off, or backup before publishing.
            </p>
          </header>
          <div style={s.sectionBody}>
            <div style={s.actionsRow} role="group">
              <button
                style={s.ghostBtn}
                onClick={quickExportMarkdown}
                disabled={mdExporting}
              >
                {mdExporting ? "Exporting…" : "Export Markdown (.md)"}
              </button>
            </div>
            {mdResult && (
              <div style={mdResult.startsWith("Failed") ? s.error : s.savedHint}>
                {mdResult}
              </div>
            )}
          </div>
        </section>

        {/* Cover & boilerplate (Phase C) */}
        <section style={s.section}>
          <header style={s.sectionHeader}>
            <h2 style={s.sectionTitle}>Cover &amp; boilerplate</h2>
            <p style={s.sectionHint}>
              Cover images are copied into{" "}
              <code style={s.code}>&lt;bundle&gt;/assets/</code> and the
              export pipeline references them. Boilerplate pages
              (copyright, dedication, acknowledgments, …) are rendered
              as front/back matter. Per-platform DPI/aspect validation
              runs at export time.
            </p>
          </header>
          <div style={s.sectionBody}>
            <h3 style={s.subH}>Cover images</h3>
            <div style={s.coverGrid}>
              {COVER_SLOTS.map((slot) => {
                const asset = coverSet?.[slot] ?? null;
                return (
                  <CoverCard
                    key={slot}
                    slot={slot}
                    label={COVER_SLOT_LABEL[slot]}
                    asset={asset}
                    busy={coverBusy === slot}
                    onImport={() => importCover(slot)}
                    onRemove={() => removeCover(slot)}
                  />
                );
              })}
            </div>
            {coverError && <div style={s.error}>{coverError}</div>}

            <h3 style={{ ...s.subH, marginTop: 16 }}>Boilerplate pages</h3>
            <p style={s.muted}>
              Add the pages that show up before chapter 1 (front matter)
              and after the final chapter (back matter). Markdown is
              rendered verbatim by the export template.
            </p>

            <div style={s.boilerAddRow}>
              {BOILERPLATE_KINDS.map((k) => (
                <button
                  key={k.id}
                  style={s.smallBtn}
                  onClick={() => addBoilerplatePage(k.id)}
                  title={`Add a ${k.label.toLowerCase()} page (${k.front ? "front" : "back"} matter)`}
                >
                  + {k.label}
                </button>
              ))}
            </div>

            {boilerplate.length === 0 && (
              <p style={s.empty}>
                No boilerplate pages yet. Click one of the buttons above
                to add your first page. Most novels want at minimum a
                copyright page, a dedication, and an acknowledgments
                page.
              </p>
            )}

            <ul style={s.boilerList}>
              {boilerplate.map((page, idx) => (
                <BoilerplateEditor
                  key={page.id}
                  page={page}
                  isFirst={idx === 0}
                  isLast={idx === boilerplate.length - 1}
                  onChange={(patch) => updateBoilerplate(page.id, patch)}
                  onRemove={() => removeBoilerplate(page.id)}
                  onMoveUp={() => moveBoilerplate(page.id, -1)}
                  onMoveDown={() => moveBoilerplate(page.id, 1)}
                />
              ))}
            </ul>

            {boilerplateError && <div style={s.error}>{boilerplateError}</div>}
            {boilerplateSavedAt && <div style={s.savedHint}>{boilerplateSavedAt}</div>}

            {boilerplate.length > 0 && (
              <div style={s.actionsRow}>
                <button
                  style={{
                    ...s.primaryBtn,
                    ...(boilerplateSaving ? s.primaryBtnBusy : {}),
                  }}
                  onClick={saveBoilerplate}
                  disabled={boilerplateSaving}
                >
                  {boilerplateSaving ? "Saving…" : "Save boilerplate"}
                </button>
              </div>
            )}
          </div>
        </section>

        {/* Per-platform publishing */}
        <section style={s.section}>
          <header style={s.sectionHeader}>
            <h2 style={s.sectionTitle}>Per-platform packages</h2>
            <p style={s.sectionHint}>
              <code style={s.code}>prepare_for_publishing</code> builds one
              folder per selected platform under
              {" "}<code style={s.code}>&lt;bundle&gt;/exports/&lt;platform&gt;/</code>
              {" "}with the right file formats and a readiness checklist.
              Each item is graded PASS / WARN / FAIL / HUMAN_REQUIRED.
            </p>
          </header>
          <div style={s.sectionBody}>
            <ul style={s.platformList}>
              {PLATFORMS.map((id) => {
                const target = targets.find((t) => t.id === id);
                const checked = selected.has(id);
                return (
                  <li key={id} style={s.platformRow}>
                    <label style={s.platformLabel}>
                      <input
                        type="checkbox"
                        checked={checked}
                        onChange={() => togglePlatform(id)}
                      />
                      <span style={s.platformName}>{PLATFORM_LABELS[id]}</span>
                    </label>
                    {target && (
                      <span style={s.platformMeta}>
                        {target.artifact_formats.join(" · ")}
                        {target.epubcheck_required && " · EPUBCheck"}
                      </span>
                    )}
                  </li>
                );
              })}
            </ul>

            {error && <div style={s.error}>{error}</div>}

            <div style={s.actionsRow}>
              <button
                style={s.primaryBtn}
                onClick={prepareForPublishing}
                disabled={running || selected.size === 0}
              >
                {running
                  ? "Preparing packages…"
                  : `Prepare ${selected.size} platform${selected.size === 1 ? "" : "s"}`}
              </button>
            </div>
          </div>
        </section>

        {/* Readiness report */}
        {result && (
          <section style={s.section}>
            <header style={s.sectionHeader}>
              <h2 style={s.sectionTitle}>
                Readiness report · {result.elapsed_s.toFixed(1)}s
              </h2>
            </header>
            <div style={s.sectionBody}>
              {result.platforms.map((p) => (
                <PlatformReport key={p.platform} platform={p} />
              ))}
            </div>
          </section>
        )}
      </div>
    </div>
  );
}

// ── Subcomponents ───────────────────────────────────────────────────────────

// ── Cover & boilerplate subcomponents ──────────────────────────────────

function CoverCard({
  slot, label, asset, busy, onImport, onRemove,
}: {
  slot:     CoverSlot;
  label:    string;
  asset:    CoverAssetDto | null;
  busy:     boolean;
  onImport: () => void;
  onRemove: () => void;
}) {
  return (
    <div style={s.coverCard}>
      <div style={s.coverCardHeader}>
        <span style={s.coverCardTitle}>{label}</span>
        {slot !== "front" && !asset && (
          <span style={s.coverCardOptional}>optional</span>
        )}
      </div>
      {asset ? (
        <>
          <div style={s.coverThumbBox}>
            <span style={s.coverThumbLabel}>
              {(Number(asset.size_bytes) / 1024).toFixed(0)} KB · {asset.mime_type.split("/")[1] ?? "image"}
            </span>
          </div>
          <p style={s.coverFilename} title={asset.bundle_path}>
            {asset.original_filename}
          </p>
          <div style={s.coverCardActions}>
            <button style={s.smallBtn} onClick={onImport} disabled={busy}>
              Replace
            </button>
            <button style={s.smallBtnGhost} onClick={onRemove} disabled={busy}>
              Remove
            </button>
          </div>
        </>
      ) : (
        <>
          <div style={s.coverThumbEmpty}>
            <span style={s.coverThumbEmptyLabel}>No image</span>
          </div>
          <p style={s.coverHint}>
            {slot === "front"
              ? "Required for ebook + paperback exports."
              : "Paperback targets only."}
          </p>
          <div style={s.coverCardActions}>
            <button style={s.primaryBtnSmall} onClick={onImport} disabled={busy}>
              {busy ? "Importing…" : "Import…"}
            </button>
          </div>
        </>
      )}
    </div>
  );
}

function BoilerplateEditor({
  page, isFirst, isLast, onChange, onRemove, onMoveUp, onMoveDown,
}: {
  page:        BoilerplatePageDto;
  isFirst:     boolean;
  isLast:      boolean;
  onChange:    (patch: Partial<BoilerplatePageDto>) => void;
  onRemove:    () => void;
  onMoveUp:    () => void;
  onMoveDown:  () => void;
}) {
  const meta = BOILERPLATE_KINDS.find((k) => k.id === page.kind);
  const half = meta?.front ? "front matter" : "back matter";
  const words = page.body_md.trim().split(/\s+/).filter(Boolean).length;
  return (
    <li style={s.boilerCard}>
      <div style={s.boilerCardHeader}>
        <span style={s.boilerKindBadge}>{half}</span>
        <input
          style={{ ...s.input, flex: 1, fontWeight: 600 }}
          value={page.title}
          placeholder={meta?.label ?? "Page title"}
          onChange={(e) => onChange({ title: e.target.value })}
        />
        <select
          style={s.input}
          value={page.kind}
          onChange={(e) => onChange({ kind: e.target.value })}
          title="Page kind"
        >
          {BOILERPLATE_KINDS.map((k) => (
            <option key={k.id} value={k.id}>{k.label}</option>
          ))}
        </select>
        <div style={s.boilerOrderBtns}>
          <button
            style={s.smallBtnGhost}
            onClick={onMoveUp}
            disabled={isFirst}
            title="Move up"
          >↑</button>
          <button
            style={s.smallBtnGhost}
            onClick={onMoveDown}
            disabled={isLast}
            title="Move down"
          >↓</button>
          <button style={s.smallBtnGhost} onClick={onRemove}>Remove</button>
        </div>
      </div>
      <textarea
        style={{
          ...s.input,
          minHeight: 100,
          fontFamily: "var(--font-prose, serif)",
        }}
        value={page.body_md}
        placeholder={placeholderForKind(page.kind)}
        onChange={(e) => onChange({ body_md: e.target.value })}
      />
      <div style={s.boilerFooter}>
        <label style={s.boilerCheckbox}>
          <input
            type="checkbox"
            checked={page.include_in_export}
            onChange={(e) => onChange({ include_in_export: e.target.checked })}
          />
          <span>Include in export</span>
        </label>
        <span style={s.boilerWords}>{words.toLocaleString()} words</span>
      </div>
    </li>
  );
}

function placeholderForKind(kind: string): string {
  switch (kind) {
    case "copyright":
      return "Copyright © 2026 by Author Name. All rights reserved. No part of this publication may be reproduced…";
    case "dedication":
      return "For my grandmother, who first taught me to listen.";
    case "epigraph":
      return '"Quote with attribution at the end."\n— Source';
    case "acknowledgments":
      return "Thanks to my early readers, my agent, the booksellers who hand-sold the first one…";
    case "about_author":
      return "Author Name is the author of two previous novels. They live in…";
    case "also_by":
      return "*Other Novel* (2024)\n*First Novel* (2022)";
    case "back_cover_blurb":
      return "When [protagonist] discovers [inciting incident], everything they thought about [theme] starts to unravel…";
    case "foreword":
      return "Foreword by Forewordist Name…";
    case "preface":
      return "I started writing this book in…";
    case "title_page":
      return "TITLE\nBy Author Name\n\n(rendered without heading; freeform Markdown)";
    default:
      return "Markdown content — paragraphs, *italic*, **bold**.";
  }
}

function Header() {
  return (
    <header style={s.header}>
      <p style={s.stageNum}>Stage 6 of 6</p>
      <h1 style={s.title}>Format &amp; Ship</h1>
      <p style={s.lede}>
        Quick exports, cover + boilerplate assembly, and per-platform
        publishing packages. The 16 validators (HRC, KDP, AI-tells, originality)
        run automatically against every package — issues surface as
        readiness checklist items.
      </p>
    </header>
  );
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div style={s.stat}>
      <div style={s.statValue}>{value}</div>
      <div style={s.statLabel}>{label}</div>
    </div>
  );
}

function PlatformReport({ platform }: { platform: PlatformReadiness }) {
  const passes = platform.items.filter((i) => i.status === "PASS").length;
  const warns  = platform.items.filter((i) => i.status === "WARN").length;
  const fails  = platform.items.filter((i) => i.status === "FAIL").length;
  const human  = platform.items.filter((i) => i.status === "HUMAN_REQUIRED").length;
  const label  = PLATFORM_LABELS[platform.platform as PlatformId] ?? platform.platform;
  return (
    <details open={!platform.uploadable} style={s.platformReport}>
      <summary style={s.platformReportSummary}>
        <span style={statusBigDot(platform.uploadable ? "PASS" : "FAIL")} />
        <span style={s.platformReportName}>{label}</span>
        <span style={s.platformReportStats}>
          {passes} pass · {warns} warn · {fails} fail{human > 0 && ` · ${human} human`}
        </span>
        <span style={s.platformReportBadge}>
          {platform.uploadable ? "ready to upload" : "needs work"}
        </span>
      </summary>
      <div style={s.platformReportBody}>
        <p style={s.platformOutputDir}>
          Output: <code style={s.code}>{platform.output_dir}</code>
        </p>
        <ul style={s.itemList}>
          {platform.items.map((item) => (
            <li key={item.id} style={s.item}>
              <span style={statusBigDot(item.status)} />
              <div style={s.itemBody}>
                <span style={s.itemLabel}>{item.label}</span>
                <span style={s.itemDetail}>{item.detail}</span>
              </div>
            </li>
          ))}
        </ul>
      </div>
    </details>
  );
}

function statusBigDot(status: string): React.CSSProperties {
  const color =
    status === "PASS"           ? "var(--color-green-500, #22c55e)" :
    status === "WARN"           ? "var(--color-amber-500, #f59e0b)" :
    status === "FAIL"           ? "var(--color-red-500, #ef4444)"   :
    status === "HUMAN_REQUIRED" ? "var(--color-amber-600, #d97706)" :
    "var(--color-neutral-300)";
  return {
    display: "inline-block",
    width: 10, height: 10, borderRadius: "50%",
    background: color, flexShrink: 0,
  };
}

const s: Record<string, React.CSSProperties> = {
  root: {
    height: "100%", overflow: "auto",
    padding: "32px 24px 48px",
    display: "flex", justifyContent: "center",
    fontFamily: "var(--font-ui)",
  },
  col: { width: "min(820px, 100%)", display: "flex", flexDirection: "column", gap: 16 },
  header: { display: "flex", flexDirection: "column", gap: 4, marginBottom: 8 },
  stageNum: {
    margin: 0, fontSize: 11, fontWeight: 600,
    textTransform: "uppercase", letterSpacing: "0.1em",
    color: "var(--color-amber-600)",
  },
  title: {
    margin: 0, fontFamily: "var(--font-prose, serif)",
    fontSize: 32, fontWeight: 700, lineHeight: 1.2,
    color: "var(--color-neutral-900)",
  },
  lede: { margin: "4px 0 0", fontSize: 14, color: "var(--color-neutral-700)", lineHeight: 1.6 },
  muted: { color: "var(--color-neutral-500)", fontSize: 13, margin: 0 },
  section: {
    background: "#fff",
    border: "1px solid var(--color-neutral-200)",
    borderRadius: 6, overflow: "hidden",
  },
  sectionHeader: {
    padding: "12px 16px",
    background: "var(--color-neutral-50)",
    borderBottom: "1px solid var(--color-neutral-200)",
  },
  sectionTitle: {
    margin: 0, fontSize: 15, fontWeight: 600,
    color: "var(--color-neutral-900)",
  },
  sectionHint: {
    margin: "4px 0 0", fontSize: 12,
    color: "var(--color-neutral-600)", lineHeight: 1.5,
  },
  sectionBody: { padding: 16, display: "flex", flexDirection: "column", gap: 12 },
  statRow: {
    display: "flex", gap: 24,
    padding: "16px",
  },
  stat: { display: "flex", flexDirection: "column", gap: 2 },
  statValue: {
    fontFamily: "var(--font-prose, serif)",
    fontSize: 22, fontWeight: 700,
    color: "var(--color-neutral-900)",
    fontVariantNumeric: "tabular-nums",
  },
  statLabel: {
    fontSize: 11, color: "var(--color-neutral-500)",
    textTransform: "uppercase", letterSpacing: "0.06em",
  },
  bannerNote: {
    padding: "10px 14px",
    background: "var(--color-neutral-50)",
    border: "1px solid var(--color-neutral-200)",
    borderRadius: 4,
    fontSize: 12, color: "var(--color-neutral-700)", lineHeight: 1.6,
  },
  platformList: {
    listStyle: "none", margin: 0, padding: 0,
    display: "flex", flexDirection: "column", gap: 4,
  },
  platformRow: {
    display: "flex", alignItems: "center", justifyContent: "space-between",
    padding: "10px 12px",
    background: "var(--color-neutral-50)",
    border: "1px solid var(--color-neutral-200)",
    borderRadius: 4,
  },
  platformLabel: {
    display: "flex", alignItems: "center", gap: 8,
    cursor: "pointer", fontSize: 13,
  },
  platformName: { fontWeight: 500, color: "var(--color-neutral-900)" },
  platformMeta: { fontSize: 11, color: "var(--color-neutral-500)", fontFamily: "var(--font-mono)" },
  actionsRow: {
    display: "flex", justifyContent: "flex-end", gap: 12, marginTop: 4,
  },
  primaryBtn: {
    padding: "10px 20px",
    background: "var(--color-amber-600)", color: "#fff",
    border: "none", borderRadius: 5,
    fontSize: 14, fontWeight: 600, cursor: "pointer",
    fontFamily: "var(--font-ui)",
  },
  ghostBtn: {
    padding: "10px 16px",
    background: "transparent", color: "var(--color-neutral-700)",
    border: "1px solid var(--color-neutral-300)", borderRadius: 5,
    fontSize: 13, fontWeight: 500, cursor: "pointer",
    fontFamily: "var(--font-ui)",
  },
  error: {
    padding: "8px 12px",
    background: "rgba(220,38,38,0.06)",
    color: "var(--color-red-700, #b91c1c)",
    border: "1px solid rgba(220,38,38,0.25)",
    borderRadius: 4, fontFamily: "var(--font-mono)", fontSize: 12,
  },
  savedHint: {
    padding: "8px 12px",
    background: "rgba(34,197,94,0.08)",
    color: "var(--color-green-700, #15803d)",
    border: "1px solid rgba(34,197,94,0.3)",
    borderRadius: 4, fontSize: 12,
  },
  platformReport: {
    border: "1px solid var(--color-neutral-200)",
    borderRadius: 4, marginBottom: 8, overflow: "hidden",
  },
  platformReportSummary: {
    display: "flex", alignItems: "center", gap: 10,
    padding: "10px 14px",
    background: "var(--color-neutral-50)",
    cursor: "pointer", userSelect: "none",
    fontSize: 13,
  },
  platformReportName: { fontWeight: 600, color: "var(--color-neutral-900)" },
  platformReportStats: {
    flex: 1, marginLeft: 8,
    fontSize: 11, color: "var(--color-neutral-500)",
    fontVariantNumeric: "tabular-nums",
  },
  platformReportBadge: {
    fontSize: 10, fontWeight: 600,
    textTransform: "uppercase", letterSpacing: "0.06em",
    color: "var(--color-neutral-600)",
  },
  platformReportBody: {
    padding: 12,
    background: "#fff",
    borderTop: "1px solid var(--color-neutral-200)",
  },
  platformOutputDir: {
    margin: "0 0 8px",
    fontSize: 11, color: "var(--color-neutral-600)",
  },
  itemList: { listStyle: "none", margin: 0, padding: 0, display: "flex", flexDirection: "column", gap: 4 },
  item: {
    display: "flex", alignItems: "flex-start", gap: 8,
    padding: "6px 8px",
    fontSize: 12,
  },
  itemBody: { display: "flex", flexDirection: "column", gap: 1, flex: 1 },
  itemLabel: { fontWeight: 500, color: "var(--color-neutral-900)" },
  itemDetail: { fontSize: 11, color: "var(--color-neutral-600)" },
  code: {
    fontFamily: "var(--font-mono)", fontSize: 11,
    padding: "1px 4px",
    background: "var(--color-neutral-100)", borderRadius: 3,
  },
  // ── Cover & boilerplate styles ─────────────────────────────────────────
  subH: {
    margin: "var(--space-2, 8px) 0 0",
    fontSize: 11, fontWeight: 600,
    color: "var(--color-neutral-600)",
    textTransform: "uppercase", letterSpacing: "0.06em",
  },
  empty: {
    fontSize: 12, color: "var(--color-neutral-500)",
    lineHeight: 1.6, margin: 0,
  },
  input: {
    boxSizing: "border-box",
    padding: "6px 10px",
    border: "1px solid var(--color-neutral-300)", borderRadius: 4,
    background: "#fff", color: "var(--color-neutral-900)",
    fontFamily: "var(--font-ui)", fontSize: 13, outline: "none",
  },
  primaryBtnBusy: { opacity: 0.7, cursor: "wait" },
  primaryBtnSmall: {
    padding: "6px 14px",
    background: "var(--color-amber-600)", color: "#fff",
    border: "none", borderRadius: 4,
    fontSize: 12, fontWeight: 600, cursor: "pointer",
    fontFamily: "var(--font-ui)",
  },
  smallBtn: {
    padding: "4px 10px",
    background: "var(--color-amber-50, #fffbeb)",
    color: "var(--color-amber-700, #b45309)",
    border: "1px solid var(--color-amber-300, #fcd34d)",
    borderRadius: 4,
    fontSize: 12, fontWeight: 600, cursor: "pointer",
    fontFamily: "var(--font-ui)",
    flexShrink: 0,
  },
  smallBtnGhost: {
    padding: "4px 10px",
    background: "transparent",
    color: "var(--color-neutral-600)",
    border: "1px solid var(--color-neutral-300)",
    borderRadius: 4,
    fontSize: 12, fontWeight: 500, cursor: "pointer",
    fontFamily: "var(--font-ui)",
    flexShrink: 0,
  },
  // Cover cards
  coverGrid: {
    display: "grid",
    gridTemplateColumns: "repeat(3, 1fr)",
    gap: 12,
  },
  coverCard: {
    display: "flex", flexDirection: "column", gap: 8,
    padding: 12,
    background: "var(--color-neutral-50)",
    border: "1px solid var(--color-neutral-200)",
    borderRadius: 4,
  },
  coverCardHeader: {
    display: "flex", justifyContent: "space-between", alignItems: "center",
  },
  coverCardTitle: {
    fontSize: 11, fontWeight: 600,
    color: "var(--color-neutral-700)",
    textTransform: "uppercase", letterSpacing: "0.04em",
  },
  coverCardOptional: {
    fontSize: 10, color: "var(--color-neutral-500)",
    fontStyle: "italic",
  },
  coverThumbBox: {
    aspectRatio: "5 / 8",
    background: "var(--color-amber-50, #fffbeb)",
    border: "1px solid var(--color-amber-300, #fcd34d)",
    borderRadius: 4,
    display: "flex", alignItems: "center", justifyContent: "center",
    padding: 8,
  },
  coverThumbLabel: {
    fontFamily: "var(--font-mono)", fontSize: 11,
    color: "var(--color-amber-700, #b45309)",
    textAlign: "center", lineHeight: 1.5,
  },
  coverThumbEmpty: {
    aspectRatio: "5 / 8",
    background: "#fff",
    border: "1px dashed var(--color-neutral-300)",
    borderRadius: 4,
    display: "flex", alignItems: "center", justifyContent: "center",
  },
  coverThumbEmptyLabel: {
    fontSize: 11, color: "var(--color-neutral-400)",
  },
  coverFilename: {
    margin: 0,
    fontSize: 11, color: "var(--color-neutral-600)",
    overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap",
  },
  coverHint: {
    margin: 0,
    fontSize: 11, color: "var(--color-neutral-500)",
    lineHeight: 1.5,
  },
  coverCardActions: {
    display: "flex", gap: 6, marginTop: "auto",
  },
  // Boilerplate
  boilerAddRow: {
    display: "flex", gap: 6, flexWrap: "wrap",
    padding: 8,
    background: "var(--color-neutral-50)",
    border: "1px dashed var(--color-neutral-300)",
    borderRadius: 4,
  },
  boilerList: {
    listStyle: "none", margin: 0, padding: 0,
    display: "flex", flexDirection: "column", gap: 10,
  },
  boilerCard: {
    background: "#fff",
    border: "1px solid var(--color-neutral-200)",
    borderRadius: 4,
    padding: 10,
    display: "flex", flexDirection: "column", gap: 8,
  },
  boilerCardHeader: {
    display: "flex", gap: 8, alignItems: "center",
  },
  boilerKindBadge: {
    fontFamily: "var(--font-mono)", fontSize: 10,
    fontWeight: 600, letterSpacing: "0.04em",
    textTransform: "uppercase",
    color: "var(--color-amber-700, #b45309)",
    background: "var(--color-amber-50, #fffbeb)",
    border: "1px solid var(--color-amber-300, #fcd34d)",
    borderRadius: 3,
    padding: "2px 8px",
    flexShrink: 0,
  },
  boilerOrderBtns: {
    display: "flex", gap: 4, flexShrink: 0,
  },
  boilerFooter: {
    display: "flex", justifyContent: "space-between", alignItems: "center",
  },
  boilerCheckbox: {
    display: "flex", alignItems: "center", gap: 6,
    fontSize: 12, color: "var(--color-neutral-700)",
    cursor: "pointer",
  },
  boilerWords: {
    fontFamily: "var(--font-mono)", fontSize: 11,
    color: "var(--color-neutral-500)",
    fontVariantNumeric: "tabular-nums",
  },
};
