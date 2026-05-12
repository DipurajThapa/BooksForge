/**
 * Voice Anchor panel (BACKLOG §A16 / Phase 3).
 *
 * Sets the project's voice anchor — the comp-sample fingerprint that
 * drafter / polish runs consume as numeric voice constraints. Persisted
 * to book-scope memory under `voice:anchor`.
 *
 * Numbers, not vibes: median sentence length, IQR, dialogue ratio,
 * em-dash density, type-token ratio. LLMs respect numeric constraints.
 */
import React, { useEffect, useState } from "react";
import { useDialogA11y } from "../../lib/useDialogA11y";
import type { VoiceProfile } from "@booksforge/shared-types";
import { ipc } from "../../lib/ipc";
import { errorMessage } from "../../lib/errorMessage";

interface Props {
  onClose: () => void;
}

export default function VoiceAnchorPanel({ onClose }: Props) {
  const { dialogProps, titleId } = useDialogA11y(onClose);
  const [compSamples,      setCompSamples]      = useState("");
  const [profile,          setProfile]          = useState<VoiceProfile | null>(null);
  const [constraintsBlock, setConstraintsBlock] = useState<string | null>(null);
  const [busy,             setBusy]             = useState(false);
  const [error,            setError]            = useState<string | null>(null);
  const [savedBanner,      setSavedBanner]      = useState<string | null>(null);

  // Load any existing anchor on mount.
  useEffect(() => {
    ipc.voiceAnchorGet()
      .then(r => {
        if (r.profile) {
          setProfile(r.profile);
          setConstraintsBlock(r.constraints_block ?? null);
        }
      })
      .catch(() => null);
  }, []);

  async function handlePreview() {
    if (!compSamples.trim()) {
      setError("Paste at least 1-3 paragraphs of comp prose.");
      return;
    }
    setError(null);
    setBusy(true);
    try {
      const r = await ipc.voiceFingerprint({ text: compSamples });
      setProfile(r.profile);
      setConstraintsBlock(r.constraints_block);
      setSavedBanner(null);
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  }

  async function handleSave() {
    if (!compSamples.trim()) {
      setError("Paste comp prose first, then save.");
      return;
    }
    setError(null);
    setBusy(true);
    try {
      const r = await ipc.voiceAnchorSet({ comp_samples: compSamples });
      setProfile(r.profile);
      setConstraintsBlock(r.constraints_block);
      setSavedBanner(`✓ Saved. ${r.profile.word_count.toLocaleString()} comp words measured. The drafter and polish stack will now anchor against this voice.`);
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div style={s.overlay} role="presentation">
      <div {...dialogProps} style={s.dialog}>
        <header style={s.header}>
          <strong id={titleId}>Voice Anchor — comp-sample fingerprint for the drafter / polish stack</strong>
          <button style={s.close} onClick={onClose} aria-label="Close panel">✕</button>
        </header>

        <div style={s.body}>
          <p style={s.blurb}>
            Paste 1–3 paragraphs of prose that sound like your target voice
            (a comp title, a previously accepted scene, anything that nails
            the tone). BooksForge measures the cadence + lexicon as numeric
            constraints and injects them into every drafter and polish run.
          </p>

          <textarea
            style={s.textarea}
            value={compSamples}
            onChange={e => setCompSamples(e.target.value)}
            placeholder="Paste comp prose here. The more representative the better — three paragraphs is plenty."
            rows={8}
          />

          <div style={s.row}>
            <button style={s.previewBtn} onClick={handlePreview} disabled={busy}>
              {busy ? "Measuring…" : "Preview measurement"}
            </button>
            <button style={s.saveBtn} onClick={handleSave} disabled={busy}>
              {busy ? "Saving…" : "Save as project voice anchor"}
            </button>
          </div>

          {error && <div style={s.error}>{error}</div>}
          {savedBanner && <div style={s.successBanner}>{savedBanner}</div>}

          {profile && (
            <div style={s.profileCard}>
              <h4 style={s.profileTitle}>Measured fingerprint</h4>
              <div style={s.profileGrid}>
                <ProfileRow label="Words measured" value={profile.word_count.toLocaleString()} />
                <ProfileRow label="Sentences" value={profile.sentence_count.toLocaleString()} />
                <ProfileRow label="Median sentence length"
                  value={`${Math.round(profile.median_sentence_length)} words (IQR ${Math.round(profile.p25_sentence_length)}–${Math.round(profile.p75_sentence_length)})`} />
                <ProfileRow label="Short-sentence share (<8 words)"
                  value={`${Math.round(profile.pct_short_sentences * 100)}%`} />
                <ProfileRow label="Long-sentence share (>25 words)"
                  value={`${Math.round(profile.pct_long_sentences * 100)}%`} />
                <ProfileRow label="Dialogue line share"
                  value={`${Math.round(profile.dialogue_ratio * 100)}%`} />
                <ProfileRow label="Vocabulary richness (TTR)"
                  value={profile.type_token_ratio.toFixed(2)} />
                <ProfileRow label="Rare-word share"
                  value={`${Math.round(profile.rare_word_ratio * 100)}%`} />
                <ProfileRow label="Em-dashes per 1000 words"
                  value={profile.em_dash_per_1000.toFixed(1)} />
                <ProfileRow label="Avg word length"
                  value={`${profile.avg_word_length.toFixed(2)} chars`} />
                <ProfileRow label="Monosyllabic-word share"
                  value={`${Math.round(profile.pct_monosyllabic_words * 100)}%`} />
              </div>
            </div>
          )}

          {constraintsBlock && (
            <details style={s.details}>
              <summary style={s.detailsHead}>Constraint block (what the drafter sees)</summary>
              <pre style={s.pre}>{constraintsBlock}</pre>
            </details>
          )}
        </div>
      </div>
    </div>
  );
}

function ProfileRow({ label, value }: { label: string; value: string }) {
  return (
    <div style={s.profileRow}>
      <span style={s.profileLabel}>{label}</span>
      <span style={s.profileValue}>{value}</span>
    </div>
  );
}

const s: Record<string, React.CSSProperties> = {
  overlay:  { position: "fixed", inset: 0, background: "rgba(0,0,0,0.4)", zIndex: 50, display: "flex", alignItems: "center", justifyContent: "center" },
  dialog:   { width: "min(820px, 94vw)", maxHeight: "92vh", display: "flex", flexDirection: "column", background: "var(--color-surface)", border: "1px solid var(--color-border)", borderRadius: 6, overflow: "hidden" },
  header:   { display: "flex", alignItems: "center", justifyContent: "space-between", padding: "10px 14px", borderBottom: "1px solid var(--color-border)" },
  close:    { background: "transparent", border: "none", fontSize: 18, cursor: "pointer", color: "inherit" },
  body:     { padding: "10px 14px", overflowY: "auto", display: "flex", flexDirection: "column", gap: 10 },
  blurb:    { margin: 0, fontSize: 13, opacity: 0.85 },
  textarea: { padding: "8px", border: "1px solid var(--color-border)", borderRadius: 3, fontSize: 13, background: "var(--color-bg)", color: "inherit", fontFamily: "Georgia, serif", resize: "vertical" },
  row:      { display: "flex", gap: 8 },
  previewBtn: { padding: "6px 14px", border: "1px solid var(--color-border)", borderRadius: 4, cursor: "pointer", background: "var(--color-bg)" },
  saveBtn:  { padding: "6px 14px", border: "1px solid var(--color-border)", borderRadius: 4, cursor: "pointer", background: "var(--color-success-bg, #e8f5e9)", color: "var(--color-success, #2e7d32)", fontWeight: 600 },
  error:    { color: "var(--color-error, #c62828)", padding: "6px 10px", fontSize: 12 },
  successBanner: { padding: 10, background: "var(--color-success-bg, #e8f5e9)", color: "var(--color-success, #2e7d32)", borderRadius: 4, fontSize: 12 },
  profileCard: { border: "1px solid var(--color-border)", borderRadius: 4, padding: 10, background: "var(--color-bg)" },
  profileTitle: { margin: "0 0 6px", fontSize: 13 },
  profileGrid: { display: "grid", gridTemplateColumns: "1fr 1fr", gap: 4, fontSize: 12 },
  profileRow:  { display: "flex", justifyContent: "space-between", padding: "2px 8px", borderBottom: "1px dashed var(--color-border)" },
  profileLabel:{ opacity: 0.75 },
  profileValue:{ fontWeight: 600, fontFamily: "ui-monospace, SFMono-Regular, monospace" },
  details:  { fontSize: 12, border: "1px solid var(--color-border)", borderRadius: 4, padding: 8 },
  detailsHead: { cursor: "pointer", fontWeight: 600 },
  pre:      { margin: "8px 0 0", whiteSpace: "pre-wrap", fontSize: 11, fontFamily: "ui-monospace, SFMono-Regular, monospace" },
};
