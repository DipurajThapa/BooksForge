/**
 * Settings panel (BACKLOG §B4).
 *
 * Five sections:
 *   1. Telemetry / crash reports — both off by default, surfaced here
 *      so the user can confirm they're off.  This is the
 *      *informational* face of CLAUDE.md privacy invariant 1: "no
 *      content leaves the device by default".
 *   2. Diagnostic bundle — opt-in support flow (BACKLOG §B3).  Saves
 *      a redacted ZIP the user can attach to a support email.
 *   3. Originality protection — surfaces the active provider
 *      (LocalOnly default, gated remote providers in §E0d.11) and a
 *      one-click "revoke remote consent" affordance.
 *   4. Export dependencies — Pandoc / Java / EPUBCheck status with
 *      install hints (powers the export panel's badges).
 *   5. App version + log directory location (read-only).
 */
import React, { useEffect, useState } from "react";
import { useDialogA11y } from "../lib/useDialogA11y";
import { save as saveDialog } from "@tauri-apps/plugin-dialog";
import type {
  ExportDependencyReport, ExportDependencyStatus, SaveDiagnosticBundleResult,
} from "@booksforge/shared-types";
import { ipc } from "../lib/ipc";
import {
  getThemePreference,
  setThemePreference,
  type ThemePreference,
} from "../lib/theme";

interface Props { onClose: () => void; }

interface ConsentRecord {
  provider:    "local_only" | "copyleaks" | "plagscan" | "turnitin";
  accepted_at: string;
  note:        string;
}

export default function SettingsPanel({ onClose }: Props) {
  const { dialogProps, titleId } = useDialogA11y(onClose);
  const [deps,        setDeps]        = useState<ExportDependencyReport | null>(null);
  const [consent,     setConsent]     = useState<ConsentRecord | null>(null);
  const [bundling,    setBundling]    = useState(false);
  const [bundleResult, setBundleResult] = useState<SaveDiagnosticBundleResult | string | null>(null);
  const [appVersion,  setAppVersion]  = useState<string>("");

  // Telemetry switches — both default to false; persisted (in MVP)
  // only in component state since the storage layer doesn't have a
  // dedicated settings table.  The contract here is the off-by-default
  // promise; persistence lands when the settings table arrives.
  const [crashReports, setCrashReports] = useState(false);
  const [usageMetrics, setUsageMetrics] = useState(false);

  // Theme preference (System / Light / Dark) — persisted in
  // localStorage by lib/theme.ts.  initThemeSystem() in main.tsx
  // applies the saved preference at boot; changing it here applies
  // immediately via setThemePreference.
  const [theme, setTheme] = useState<ThemePreference>(() => getThemePreference());

  useEffect(() => {
    ipc.exportCheckDependencies().then(setDeps).catch(() => null);
    ipc.originalityConsentLoad().then(c => setConsent(c as ConsentRecord)).catch(() => null);
    ipc.appVersion().then(v => setAppVersion(`${v.major}.${v.minor}.${v.patch}${v.pre ? `-${v.pre}` : ""}`)).catch(() => null);
  }, []);

  async function handleBundle() {
    const path = await saveDialog({
      title:       "Save diagnostic bundle",
      defaultPath: `booksforge-diagnostic-${new Date().toISOString().slice(0, 10)}.zip`,
      filters:     [{ name: "Diagnostic bundle", extensions: ["zip"] }],
    }).catch(() => null);
    if (!path) return;
    setBundling(true);
    setBundleResult(null);
    try {
      const r = await ipc.saveDiagnosticBundle({
        output_path: typeof path === "string" ? path : path.path,
      });
      setBundleResult(r);
    } catch (e) {
      setBundleResult(String(e));
    } finally {
      setBundling(false);
    }
  }

  async function handleRevokeConsent() {
    try {
      await ipc.originalityConsentClear();
      const c = await ipc.originalityConsentLoad();
      setConsent(c as ConsentRecord);
    } catch (e) {
      console.error(e);
    }
  }

  return (
    <div style={s.overlay} role="presentation">
      <div {...dialogProps} style={s.dialog}>
        <header style={s.header}>
          <strong id={titleId}>Settings</strong>
          <button style={s.close} onClick={onClose} aria-label="Close panel">✕</button>
        </header>

        <div style={s.body}>
          {/* ── Section 0: Appearance (theme) ── */}
          <section style={s.section}>
            <h4 style={s.sectionTitle}>Appearance</h4>
            <p style={s.sectionBlurb}>
              Match the operating-system theme, or pin BooksForge to a
              specific light or dark theme.
            </p>
            <div role="radiogroup" aria-label="Theme" style={{ display: "flex", gap: "0.5rem" }}>
              {(["system", "light", "dark"] as const).map((value) => (
                <label
                  key={value}
                  style={{
                    flex: 1,
                    padding: "0.5rem 0.75rem",
                    border: theme === value
                      ? "1px solid var(--color-primary, #0969da)"
                      : "1px solid var(--color-neutral-300, #d0d7de)",
                    borderRadius: "0.375rem",
                    cursor: "pointer",
                    textTransform: "capitalize",
                    background: theme === value
                      ? "var(--color-primary-bg, #ddf4ff)"
                      : "transparent",
                  }}
                >
                  <input
                    type="radio"
                    name="bf-theme"
                    value={value}
                    checked={theme === value}
                    onChange={() => {
                      setTheme(value);
                      setThemePreference(value);
                    }}
                    style={{ marginRight: "0.5rem" }}
                  />
                  {value === "system" ? "Match system" : value}
                </label>
              ))}
            </div>
          </section>

          {/* ── Section 1: Telemetry ── */}
          <section style={s.section}>
            <h4 style={s.sectionTitle}>Telemetry &amp; crash reports</h4>
            <p style={s.sectionBlurb}>
              BooksForge is local-first.  Nothing leaves your device by default
              — these toggles are off and will stay off unless you flip them.
            </p>
            <Toggle
              label="Crash reports"
              hint="Send anonymous crash logs (after PII redaction) to help diagnose stability issues.  Requires consent every session."
              value={crashReports}
              onChange={setCrashReports}
            />
            <Toggle
              label="Usage metrics"
              hint="Send aggregate usage events (button clicks, no manuscript content).  Strictly opt-in."
              value={usageMetrics}
              onChange={setUsageMetrics}
            />
            {(crashReports || usageMetrics) && (
              <div style={s.warn}>
                Note: both telemetry channels are <em>scaffolded but not wired</em>
                {" "}in MVP — even if these are on, no remote endpoint is
                actually contacted.  Toggles preserve user intent for when the
                opt-in flow ships.
              </div>
            )}
          </section>

          {/* ── Section 2: Diagnostic bundle ── */}
          <section style={s.section}>
            <h4 style={s.sectionTitle}>Diagnostic bundle</h4>
            <p style={s.sectionBlurb}>
              Save a ZIP containing PII-redacted log files and app metadata
              for support.  Manuscript content is <strong>never</strong> included.
            </p>
            <button
              style={s.actionBtn}
              onClick={handleBundle}
              disabled={bundling}
            >
              {bundling ? "Building bundle…" : "Save diagnostic bundle…"}
            </button>
            {bundleResult && typeof bundleResult !== "string" && (
              <div style={s.ok}>
                Saved {bundleResult.bytes.toLocaleString()} bytes,
                {" "}{bundleResult.log_files_included} log file(s) included
                {bundleResult.redaction_applied && " · PII redacted"}.
              </div>
            )}
            {typeof bundleResult === "string" && (
              <div style={s.error}>{bundleResult}</div>
            )}
          </section>

          {/* ── Section 3: Originality protection ── */}
          <section style={s.section}>
            <h4 style={s.sectionTitle}>Originality protection</h4>
            <p style={s.sectionBlurb}>
              Plagiarism / verbatim-overlap detection.  The local detector
              runs on every prose-emitting agent's output and on demand
              (chapter scan).  Online providers are opt-in and gated on a
              one-time consent flow that hasn't shipped yet.
            </p>
            <div style={s.detail}>
              Active provider:{" "}
              <strong>
                {consent?.provider === "local_only"
                  ? "Local only"
                  : consent?.provider ?? "loading…"}
              </strong>
            </div>
            {consent && consent.provider !== "local_only" && (
              <button style={s.actionBtn} onClick={handleRevokeConsent}>
                Revoke consent &amp; switch back to local-only
              </button>
            )}
          </section>

          {/* ── Section 4: Export dependencies ── */}
          <section style={s.section}>
            <h4 style={s.sectionTitle}>Export dependencies</h4>
            <p style={s.sectionBlurb}>
              External tools the export pipeline can use.  EPUB and
              Markdown profiles work with no extras; DOCX / PDF need
              Pandoc, EPUB validation needs Java + EPUBCheck.
            </p>
            {deps ? (
              <ul style={s.depList}>
                {deps.items.map(d => <DepRow key={d.id} dep={d} />)}
              </ul>
            ) : (
              <div style={s.muted}>checking…</div>
            )}
          </section>

          {/* ── Section 5: App info ── */}
          <section style={s.section}>
            <h4 style={s.sectionTitle}>About</h4>
            <div style={s.detail}>BooksForge {appVersion || "—"}</div>
            <div style={s.detail}>
              Logs: <code style={s.code}>~/Library/Logs/BooksForge/</code>{" "}
              (macOS) · <code style={s.code}>%LOCALAPPDATA%\BooksForge\Logs\</code>{" "}
              (Windows) · <code style={s.code}>~/.local/state/booksforge/</code>{" "}
              (Linux)
            </div>
          </section>
        </div>
      </div>
    </div>
  );
}

function Toggle({ label, hint, value, onChange }: {
  label: string; hint: string; value: boolean; onChange: (v: boolean) => void;
}) {
  return (
    <label style={s.toggle}>
      <input
        type="checkbox"
        checked={value}
        onChange={e => onChange(e.target.checked)}
      />
      <span>
        <span style={s.toggleLabel}>{label}</span>
        <span style={s.toggleHint}>{hint}</span>
      </span>
    </label>
  );
}

function DepRow({ dep }: { dep: ExportDependencyStatus }) {
  return (
    <li style={s.depRow}>
      <div style={s.depHead}>
        <strong>{dep.name}</strong>
        <span style={{
          ...s.depStatus,
          color: dep.found
            ? "var(--color-success, #2e7d32)"
            : "var(--color-warn, #f9a825)",
        }}>
          {dep.found ? "found" : "not configured"}
        </span>
      </div>
      {dep.found ? (
        <>
          {dep.version && <div style={s.muted}>{dep.version}</div>}
          {dep.path && <div style={s.muted}><code style={s.code}>{dep.path}</code></div>}
          <div style={s.muted}>
            Unlocks: {dep.unlocks.join(", ")}
          </div>
        </>
      ) : (
        <div style={s.installHint}>{dep.install_hint}</div>
      )}
    </li>
  );
}

const s: Record<string, React.CSSProperties> = {
  overlay:  { position: "fixed", inset: 0, background: "rgba(0,0,0,0.4)", zIndex: 50, display: "flex", alignItems: "center", justifyContent: "center" },
  dialog:   { width: "min(720px, 92vw)", maxHeight: "90vh", display: "flex", flexDirection: "column", background: "var(--color-surface)", border: "1px solid var(--color-border)", borderRadius: 6, overflow: "hidden" },
  header:   { display: "flex", alignItems: "center", justifyContent: "space-between", padding: "10px 14px", borderBottom: "1px solid var(--color-border)" },
  close:    { background: "transparent", border: "none", fontSize: 18, cursor: "pointer", color: "inherit" },
  body:     { padding: "12px 14px", overflowY: "auto", flex: 1, display: "flex", flexDirection: "column", gap: 18 },
  section:  { display: "flex", flexDirection: "column", gap: 8 },
  sectionTitle: { fontSize: 14, fontWeight: 600, margin: 0 },
  sectionBlurb: { fontSize: 12, opacity: 0.85, margin: 0 },
  toggle:   { display: "flex", alignItems: "flex-start", gap: 8, padding: 6, fontSize: 13 },
  toggleLabel: { fontWeight: 600, display: "block" },
  toggleHint:  { fontSize: 12, opacity: 0.75, display: "block" },
  warn:     { padding: 8, fontSize: 12, background: "var(--color-warn-bg, rgba(249,168,37,0.12))", color: "var(--color-warn, #f9a825)", borderRadius: 4 },
  ok:       { padding: 8, fontSize: 12, background: "var(--color-success-bg, rgba(46,125,50,0.12))", color: "var(--color-success, #2e7d32)", borderRadius: 4 },
  error:    { color: "var(--color-error, #c62828)", padding: 8, fontSize: 12 },
  actionBtn:{ alignSelf: "flex-start", padding: "6px 12px", border: "1px solid var(--color-border)", borderRadius: 4, cursor: "pointer", background: "var(--color-bg)", color: "inherit" },
  detail:   { fontSize: 12 },
  depList:  { listStyle: "none", padding: 0, margin: 0, display: "flex", flexDirection: "column", gap: 8 },
  depRow:   { padding: 8, border: "1px solid var(--color-border)", borderRadius: 4, display: "flex", flexDirection: "column", gap: 4 },
  depHead:  { display: "flex", alignItems: "baseline", gap: 8 },
  depStatus:{ marginLeft: "auto", fontSize: 11, fontWeight: 600 },
  installHint: { fontSize: 12, fontStyle: "italic", opacity: 0.85 },
  muted:    { fontSize: 12, opacity: 0.75 },
  code:     { fontFamily: "ui-monospace, SFMono-Regular, monospace", fontSize: 11 },
};
