/**
 * Ollama Setup Wizard — 4-step modal.
 *
 * Step 1 — Detect: probe the local environment.
 * Step 2 — Install: guide the user to install Ollama if not found.
 * Step 3 — Pick model: show the curated list filtered by detected RAM.
 * Step 4 — Smoke test: verify the chosen model responds correctly.
 *
 * The wizard can be dismissed at any time via the × button.
 */
import React, { useCallback, useEffect, useRef, useState } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { ModelListEntry, OllamaProbeResult, PullProgressPayload } from "@booksforge/shared-types";
import { useDialogA11y } from "../lib/useDialogA11y";
import { ipc } from "../lib/ipc";
import { errorMessage } from "../lib/errorMessage";
interface Props {
  onClose: () => void;
  /** Called when the wizard completes successfully with a usable model. */
  onComplete: (modelId: string) => void;
}

type Step = "detect" | "install" | "pick" | "pull" | "smoke" | "done";

interface PullState {
  status: string;
  completed: number | null;
  total: number | null;
}

// ── Helpers ───────────────────────────────────────────────────────────────────

function formatBytes(bytes: number): string {
  if (bytes >= 1_073_741_824) return `${(bytes / 1_073_741_824).toFixed(1)} GB`;
  if (bytes >= 1_048_576) return `${(bytes / 1_048_576).toFixed(0)} MB`;
  return `${bytes} B`;
}

function pullPercent(p: PullState): number | null {
  if (p.total == null || p.total === 0) return null;
  return Math.min(100, Math.round(((p.completed ?? 0) / p.total) * 100));
}

// ── Component ─────────────────────────────────────────────────────────────────

export default function OllamaWizard({ onClose, onComplete }: Props) {
  const { dialogProps, titleId } = useDialogA11y(onClose);
  const [step, setStep] = useState<Step>("detect");
  const [probe, setProbe] = useState<OllamaProbeResult | null>(null);
  const [models, setModels] = useState<ModelListEntry[]>([]);
  const [selectedModel, setSelectedModel] = useState<string | null>(null);
  const [pullState, setPullState] = useState<PullState | null>(null);
  const [pullError, setPullError] = useState<string | null>(null);
  const [smokeLoading, setSmokeLoading] = useState(false);
  const [smokeError, setSmokeError] = useState<string | null>(null);
  const [smokeResponse, setSmokeResponse] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const unlistenRef = useRef<UnlistenFn | null>(null);

  // ── Step 1: Detect ──────────────────────────────────────────────────────────

  const runProbe = useCallback(async () => {
    setStep("detect");
    setError(null);
    try {
      const result = await ipc.ollamaProbe();
      setProbe(result);
      if (result.api_reachable) {
        // Already running — jump to model pick.
        await loadModels();
        setStep("pick");
      } else if (result.binary_found) {
        // Installed but not running.
        setStep("install");
      } else {
        // Not installed at all.
        setStep("install");
      }
    } catch (e) {
      setError(errorMessage(e));
    }
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  useEffect(() => {
    runProbe();
    return () => {
      unlistenRef.current?.();
    };
  }, [runProbe]);

  async function loadModels() {
    const list = await ipc.ollamaListModels().catch(() => []);
    setModels(list);
    // Pre-select: prefer an installed model, else the first default.
    const installed = list.find((m) => m.is_installed);
    const first = list[0];
    setSelectedModel((installed ?? first)?.id ?? null);
  }

  // ── Step 2: Install / Launch ────────────────────────────────────────────────

  async function handleLaunch() {
    setError(null);
    try {
      await ipc.ollamaLaunch();
    } catch {
      // Launch is best-effort — ignore errors, just start polling.
    }
    // Poll for up to 30 s.
    let attempts = 0;
    const maxAttempts = 15;
    const poll = setInterval(async () => {
      attempts++;
      const result = await ipc.ollamaProbe().catch(() => null);
      if (result?.api_reachable) {
        clearInterval(poll);
        setProbe(result);
        await loadModels();
        setStep("pick");
      } else if (attempts >= maxAttempts) {
        clearInterval(poll);
        setError("Ollama did not start within 30 seconds. Please start it manually.");
      }
    }, 2000);
  }

  // ── Step 3: Pull ────────────────────────────────────────────────────────────

  async function handlePull() {
    if (!selectedModel) return;
    setPullError(null);
    setPullState({ status: "Starting…", completed: null, total: null });
    setStep("pull");

    // Subscribe to progress events.
    unlistenRef.current = await listen<PullProgressPayload>(
      "ollama:pull-progress",
      (event) => {
        const p = event.payload;
        if (p.model === selectedModel) {
          setPullState({
            status: p.status,
            completed: p.completed ?? null,
            total: p.total ?? null,
          });
        }
      }
    );

    try {
      await ipc.ollamaPull(selectedModel);
      unlistenRef.current?.();
      unlistenRef.current = null;
      // Refresh models list so is_installed flips to true.
      await loadModels();
      setStep("smoke");
      await runSmokeTest(selectedModel);
    } catch (e) {
      unlistenRef.current?.();
      unlistenRef.current = null;
      setPullError(errorMessage(e));
      setStep("pick");
    }
  }

  // ── Step 4: Smoke test ──────────────────────────────────────────────────────

  async function runSmokeTest(model: string) {
    setSmokeLoading(true);
    setSmokeError(null);
    setSmokeResponse(null);
    try {
      const result = await ipc.ollamaSmokeTest(model);
      if (result.success) {
        setSmokeResponse(result.response ?? "");
        setStep("done");
      } else {
        setSmokeError(result.error ?? "Smoke test returned an empty response.");
        setStep("smoke");
      }
    } catch (e) {
      setSmokeError(errorMessage(e));
      setStep("smoke");
    } finally {
      setSmokeLoading(false);
    }
  }

  // ── Render ──────────────────────────────────────────────────────────────────

  const ramGb = probe?.ram_gb ?? null;

  // Filter models to those fitting available RAM (show all if RAM unknown).
  const filteredModels = ramGb != null
    ? models.filter((m) => m.ram_min_gb <= ramGb)
    : models;
  const displayModels = filteredModels.length > 0 ? filteredModels : models;

  return (
    <div style={s.overlay} role="presentation">
      <div {...dialogProps} style={s.dialog}>
        {/* Header */}
        <div style={s.header}>
          <span id={titleId} style={s.title}>AI Setup</span>
          <button style={s.closeBtn} onClick={onClose} aria-label="Close">×</button>
        </div>

        {/* Step indicator */}
        <StepBar current={step} />

        {/* Body */}
        <div style={s.body}>
          {step === "detect" && (
            <CenteredMessage icon="🔍" heading="Checking for Ollama…">
              <p style={s.sub}>Probing your local environment.</p>
            </CenteredMessage>
          )}

          {step === "install" && probe && (
            <InstallStep
              apiReachable={probe.api_reachable}
              binaryFound={probe.binary_found}
              error={error}
              onLaunch={handleLaunch}
              onRetry={runProbe}
            />
          )}

          {step === "pick" && (
            <PickStep
              models={displayModels}
              ramGb={ramGb}
              selectedModel={selectedModel}
              onSelect={setSelectedModel}
              pullError={pullError}
              onPull={handlePull}
            />
          )}

          {step === "pull" && selectedModel && pullState && (
            <PullStep
              model={selectedModel}
              pullState={pullState}
            />
          )}

          {(step === "smoke" || step === "done") && (
            <SmokeStep
              loading={smokeLoading}
              error={smokeError}
              response={smokeResponse}
              model={selectedModel}
              onRetry={() => selectedModel && runSmokeTest(selectedModel)}
              onPickDifferent={() => setStep("pick")}
              onFinish={() => selectedModel && onComplete(selectedModel)}
            />
          )}
        </div>
      </div>
    </div>
  );
}

// ── Sub-components ────────────────────────────────────────────────────────────

function StepBar({ current }: { current: Step }) {
  const steps: [Step | Step[], string][] = [
    [["detect", "install"], "Detect"],
    [["pick", "pull"], "Model"],
    [["smoke", "done"], "Test"],
  ];
  return (
    <div style={s.stepBar}>
      {steps.map(([keys, label], i) => {
        const stepKeys = Array.isArray(keys) ? keys : [keys];
        const active = stepKeys.includes(current);
        const done = (
          (i === 0 && ["pick", "pull", "smoke", "done"].includes(current)) ||
          (i === 1 && ["smoke", "done"].includes(current))
        );
        return (
          <React.Fragment key={label}>
            {i > 0 && <div style={{ ...s.stepLine, background: done ? "var(--color-amber-500)" : "var(--color-border)" }} />}
            <div style={{
              ...s.stepDot,
              background: done
                ? "var(--color-amber-500)"
                : active
                ? "var(--color-amber-600)"
                : "var(--color-neutral-200)",
              color: (done || active) ? "#fff" : "var(--color-text-tertiary)",
            }}>
              {done ? "✓" : i + 1}
            </div>
            <span style={{
              ...s.stepLabel,
              color: active ? "var(--color-text-primary)" : "var(--color-text-tertiary)",
              fontWeight: active ? 600 : 400,
            }}>{label}</span>
          </React.Fragment>
        );
      })}
    </div>
  );
}

function CenteredMessage({
  icon, heading, children,
}: { icon: string; heading: string; children?: React.ReactNode }) {
  return (
    <div style={s.centered}>
      <div style={s.bigIcon}>{icon}</div>
      <h3 style={s.heading}>{heading}</h3>
      {children}
    </div>
  );
}

function InstallStep({
  apiReachable, binaryFound, error, onLaunch, onRetry,
}: {
  apiReachable: boolean;
  binaryFound: boolean;
  error: string | null;
  onLaunch: () => void;
  onRetry: () => void;
}) {
  if (apiReachable) {
    return (
      <CenteredMessage icon="✅" heading="Ollama is running">
        <p style={s.sub}>Ollama was detected on your system.</p>
      </CenteredMessage>
    );
  }

  if (binaryFound) {
    return (
      <div style={s.installBlock}>
        <div style={s.bigIcon}>🟡</div>
        <h3 style={s.heading}>Ollama is installed but not running</h3>
        <p style={s.body2}>Click below to launch it, or start it manually from your Applications.</p>
        {error && <p style={s.errorText}>{error}</p>}
        <div style={s.btnRow}>
          <button style={s.primaryBtn} onClick={onLaunch}>Launch Ollama</button>
          <button style={s.secondaryBtn} onClick={onRetry}>Re-check</button>
        </div>
      </div>
    );
  }

  return (
    <div style={s.installBlock}>
      <div style={s.bigIcon}>📥</div>
      <h3 style={s.heading}>Install Ollama</h3>
      <p style={s.body2}>
        Ollama provides free local AI inference. BooksForge uses it to power all
        writing assistance without sending your manuscript to the internet.
      </p>
      <div style={s.installOptions}>
        <div style={s.installCard}>
          <strong>Guided install</strong>
          <p style={s.installCardSub}>Download the official installer.</p>
          <a
            href="https://ollama.com/download"
            style={s.linkBtn}
            target="_blank"
            rel="noreferrer noopener"
          >
            Download Ollama →
          </a>
        </div>
        <div style={s.installCard}>
          <strong>macOS Homebrew</strong>
          <code style={s.codeBlock}>brew install ollama</code>
        </div>
      </div>
      <p style={s.sub}>After installing, come back here and click Re-check.</p>
      <button style={s.secondaryBtn} onClick={onRetry}>Re-check</button>
    </div>
  );
}

function PickStep({
  models, ramGb, selectedModel, onSelect, pullError, onPull,
}: {
  models: ModelListEntry[];
  ramGb: number | null;
  selectedModel: string | null;
  onSelect: (id: string) => void;
  pullError: string | null;
  onPull: () => void;
}) {
  const installed = models.filter((m) => m.is_installed);
  const notInstalled = models.filter((m) => !m.is_installed);

  function renderModel(m: ModelListEntry) {
    const isSelected = m.id === selectedModel;
    return (
      <div
        key={m.id}
        style={{
          ...s.modelRow,
          background: isSelected ? "var(--color-amber-50, #fffbeb)" : undefined,
          borderColor: isSelected ? "var(--color-amber-400)" : "var(--color-border)",
        }}
        onClick={() => onSelect(m.id)}
        role="radio"
        aria-checked={isSelected}
        tabIndex={0}
        onKeyDown={(e) => e.key === "Enter" && onSelect(m.id)}
      >
        <div style={s.modelCheck}>
          {isSelected ? "●" : "○"}
        </div>
        <div style={s.modelInfo}>
          <div style={s.modelName}>
            {m.display_name}
            {m.is_installed && <span style={s.installedBadge}>Installed</span>}
            {m.default_for_modes.length > 0 && (
              <span style={s.defaultBadge}>Recommended</span>
            )}
          </div>
          <div style={s.modelMeta}>
            {formatBytes(m.size_bytes)} · {m.ram_min_gb} GB RAM · {(m.context_window / 1000).toFixed(0)}K ctx
          </div>
          <div style={s.modelStrengths}>{m.strengths.join(" · ")}</div>
          <div style={s.modelNotes}>{m.notes}</div>
        </div>
      </div>
    );
  }

  const selectedEntry = models.find((m) => m.id === selectedModel);
  const isSelectedInstalled = selectedEntry?.is_installed ?? false;

  return (
    <div style={s.pickBlock}>
      {ramGb != null && (
        <p style={s.ramNote}>
          Detected {ramGb} GB RAM — showing compatible models.
        </p>
      )}

      {installed.length > 0 && (
        <>
          <p style={s.sectionLabel}>INSTALLED</p>
          {installed.map(renderModel)}
        </>
      )}

      {notInstalled.length > 0 && (
        <>
          <p style={s.sectionLabel}>AVAILABLE TO DOWNLOAD</p>
          {notInstalled.map(renderModel)}
        </>
      )}

      {pullError && <p style={s.errorText}>{pullError}</p>}

      <div style={s.btnRow}>
        {isSelectedInstalled ? (
          <button style={s.primaryBtn} onClick={onPull} disabled={!selectedModel}>
            Use {selectedEntry?.display_name ?? "this model"} →
          </button>
        ) : (
          <button style={s.primaryBtn} onClick={onPull} disabled={!selectedModel}>
            Download &amp; use {selectedEntry?.display_name ?? "model"} →
          </button>
        )}
      </div>
    </div>
  );
}

function PullStep({ model, pullState }: { model: string; pullState: PullState }) {
  const pct = pullPercent(pullState);

  return (
    <div style={s.centered}>
      <div style={s.bigIcon}>⬇️</div>
      <h3 style={s.heading}>Downloading model…</h3>
      <p style={s.sub} title={model}>{model}</p>
      <div style={s.progressBar}>
        <div
          style={{
            ...s.progressFill,
            width: pct != null ? `${pct}%` : "30%",
            // Indeterminate animation when no percentage is available.
            animation: pct == null ? "booksforge-indeterminate 1.4s ease infinite" : undefined,
          }}
        />
      </div>
      <p style={s.sub}>
        {pct != null ? `${pct}%` : pullState.status}
        {pullState.total != null && pct != null
          ? ` · ${formatBytes(pullState.completed ?? 0)} / ${formatBytes(pullState.total)}`
          : ""}
      </p>
    </div>
  );
}

function SmokeStep({
  loading, error, response, model, onRetry, onPickDifferent, onFinish,
}: {
  loading: boolean;
  error: string | null;
  response: string | null;
  model: string | null;
  onRetry: () => void;
  onPickDifferent: () => void;
  onFinish: () => void;
}) {
  if (loading) {
    return (
      <CenteredMessage icon="🧪" heading="Testing model…">
        <p style={s.sub}>Running a quick check to make sure everything works.</p>
      </CenteredMessage>
    );
  }

  if (error) {
    return (
      <div style={s.centered}>
        <div style={s.bigIcon}>❌</div>
        <h3 style={s.heading}>Model test failed</h3>
        <p style={{ ...s.errorText, maxWidth: 380, textAlign: "center" }}>{error}</p>
        <div style={s.btnRow}>
          <button style={s.secondaryBtn} onClick={onPickDifferent}>Try a different model</button>
          <button style={s.primaryBtn} onClick={onRetry}>Retry</button>
        </div>
      </div>
    );
  }

  return (
    <div style={s.centered}>
      <div style={s.bigIcon}>🎉</div>
      <h3 style={s.heading}>AI is ready!</h3>
      {response && (
        <p style={{ ...s.sub, fontStyle: "italic", maxWidth: 360 }}>"{response}"</p>
      )}
      <p style={s.sub}>
        {model && <><strong>{model}</strong> is installed and working.</>}
      </p>
      <button style={s.primaryBtn} onClick={onFinish}>
        Start writing →
      </button>
    </div>
  );
}

// ── Styles ────────────────────────────────────────────────────────────────────

const s: Record<string, React.CSSProperties> = {
  overlay: {
    position: "fixed",
    inset: 0,
    background: "rgba(0,0,0,0.55)",
    display: "flex",
    alignItems: "center",
    justifyContent: "center",
    zIndex: 600,
  },
  dialog: {
    width: 540,
    maxHeight: "90vh",
    background: "var(--color-surface)",
    borderRadius: 12,
    boxShadow: "0 32px 80px rgba(0,0,0,0.40)",
    display: "flex",
    flexDirection: "column",
    overflow: "hidden",
  },
  header: {
    display: "flex",
    alignItems: "center",
    justifyContent: "space-between",
    padding: "var(--space-4) var(--space-5)",
    borderBottom: "1px solid var(--color-border)",
    flexShrink: 0,
  },
  title: {
    fontWeight: 700,
    fontSize: 16,
    color: "var(--color-text-primary)",
    fontFamily: "var(--font-ui)",
  },
  closeBtn: {
    background: "none",
    border: "none",
    fontSize: 22,
    cursor: "pointer",
    color: "var(--color-text-tertiary)",
    lineHeight: 1,
    padding: "0 2px",
  },
  stepBar: {
    display: "flex",
    alignItems: "center",
    gap: "var(--space-2)",
    padding: "var(--space-3) var(--space-5)",
    borderBottom: "1px solid var(--color-border)",
    flexShrink: 0,
  },
  stepDot: {
    width: 22,
    height: 22,
    borderRadius: "50%",
    display: "flex",
    alignItems: "center",
    justifyContent: "center",
    fontSize: 11,
    fontWeight: 700,
    flexShrink: 0,
  },
  stepLine: {
    flex: 1,
    height: 2,
    borderRadius: 1,
    maxWidth: 60,
  },
  stepLabel: {
    fontSize: 12,
    fontFamily: "var(--font-ui)",
    flexShrink: 0,
  },
  body: {
    flex: 1,
    overflow: "auto",
    padding: "var(--space-6)",
  },
  centered: {
    display: "flex",
    flexDirection: "column",
    alignItems: "center",
    gap: "var(--space-3)",
    paddingTop: "var(--space-4)",
    textAlign: "center",
  },
  bigIcon: { fontSize: 48, lineHeight: 1 },
  heading: {
    margin: 0,
    fontSize: 20,
    fontWeight: 700,
    color: "var(--color-text-primary)",
  },
  sub: {
    margin: 0,
    fontSize: 13,
    color: "var(--color-text-secondary)",
    lineHeight: 1.5,
  },
  errorText: {
    margin: 0,
    fontSize: 13,
    color: "var(--color-error)",
    lineHeight: 1.5,
  },
  btnRow: {
    display: "flex",
    gap: "var(--space-2)",
    flexWrap: "wrap",
    justifyContent: "center",
    marginTop: "var(--space-2)",
  },
  primaryBtn: {
    padding: "var(--space-2) var(--space-5)",
    background: "var(--color-amber-600)",
    border: "none",
    borderRadius: 6,
    fontSize: 14,
    fontWeight: 600,
    color: "#fff",
    cursor: "pointer",
    fontFamily: "var(--font-ui)",
  },
  secondaryBtn: {
    padding: "var(--space-2) var(--space-4)",
    background: "transparent",
    border: "1px solid var(--color-border)",
    borderRadius: 6,
    fontSize: 14,
    color: "var(--color-text-secondary)",
    cursor: "pointer",
    fontFamily: "var(--font-ui)",
  },
  linkBtn: {
    display: "inline-block",
    marginTop: "var(--space-2)",
    padding: "var(--space-2) var(--space-4)",
    background: "var(--color-amber-600)",
    borderRadius: 6,
    fontSize: 13,
    fontWeight: 600,
    color: "#fff",
    textDecoration: "none",
    fontFamily: "var(--font-ui)",
  },
  // Install step
  installBlock: {
    display: "flex",
    flexDirection: "column",
    gap: "var(--space-3)",
  },
  body2: {
    margin: 0,
    fontSize: 14,
    color: "var(--color-text-primary)",
    lineHeight: 1.6,
  },
  installOptions: {
    display: "flex",
    gap: "var(--space-3)",
    flexWrap: "wrap",
  },
  installCard: {
    flex: 1,
    minWidth: 180,
    border: "1px solid var(--color-border)",
    borderRadius: 8,
    padding: "var(--space-3)",
    display: "flex",
    flexDirection: "column",
    gap: "var(--space-1)",
    fontSize: 13,
    color: "var(--color-text-primary)",
  },
  installCardSub: {
    margin: 0,
    fontSize: 12,
    color: "var(--color-text-secondary)",
  },
  codeBlock: {
    display: "block",
    background: "var(--color-neutral-100, #f5f5f5)",
    borderRadius: 4,
    padding: "var(--space-2) var(--space-3)",
    fontFamily: "var(--font-mono)",
    fontSize: 12,
    marginTop: "var(--space-1)",
    userSelect: "text",
  },
  // Pick step
  pickBlock: {
    display: "flex",
    flexDirection: "column",
    gap: "var(--space-2)",
  },
  ramNote: {
    margin: 0,
    fontSize: 12,
    color: "var(--color-text-tertiary)",
    fontStyle: "italic",
  },
  sectionLabel: {
    margin: "var(--space-2) 0 var(--space-1)",
    fontSize: 10,
    fontWeight: 700,
    letterSpacing: "0.08em",
    color: "var(--color-text-tertiary)",
    textTransform: "uppercase",
  },
  modelRow: {
    display: "flex",
    gap: "var(--space-3)",
    padding: "var(--space-3)",
    border: "1px solid var(--color-border)",
    borderRadius: 8,
    cursor: "pointer",
    transition: "background 0.1s",
  },
  modelCheck: {
    fontSize: 16,
    color: "var(--color-amber-600)",
    flexShrink: 0,
    paddingTop: 2,
  },
  modelInfo: {
    flex: 1,
    display: "flex",
    flexDirection: "column",
    gap: 2,
  },
  modelName: {
    fontSize: 14,
    fontWeight: 600,
    color: "var(--color-text-primary)",
    display: "flex",
    alignItems: "center",
    gap: "var(--space-2)",
    flexWrap: "wrap",
  },
  installedBadge: {
    fontSize: 10,
    fontWeight: 700,
    padding: "1px 6px",
    borderRadius: 10,
    background: "var(--color-success)",
    color: "#fff",
  },
  defaultBadge: {
    fontSize: 10,
    fontWeight: 700,
    padding: "1px 6px",
    borderRadius: 10,
    background: "var(--color-amber-500)",
    color: "#fff",
  },
  modelMeta: {
    fontSize: 12,
    color: "var(--color-text-secondary)",
    fontFamily: "var(--font-mono)",
  },
  modelStrengths: {
    fontSize: 11,
    color: "var(--color-text-tertiary)",
    fontStyle: "italic",
  },
  modelNotes: {
    fontSize: 12,
    color: "var(--color-text-secondary)",
    lineHeight: 1.4,
  },
  // Pull step
  progressBar: {
    width: "100%",
    maxWidth: 400,
    height: 8,
    background: "var(--color-neutral-200, #e5e5e5)",
    borderRadius: 4,
    overflow: "hidden",
  },
  progressFill: {
    height: "100%",
    background: "var(--color-amber-500)",
    borderRadius: 4,
    transition: "width 0.3s ease",
  },
};
