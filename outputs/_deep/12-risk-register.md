# Risk Register & Mitigations — BooksForge

**Version:** 1.0.0-draft  •  **Status:** For review  •  **Date:** 2026-05-06

Severity is the product of impact and likelihood, scored 1–5 each. Status: **Open / Mitigating / Accepted / Closed**.

| ID | Risk | Likelihood | Impact | Severity | Status | Mitigation |
|----|------|------------|--------|----------|--------|------------|
| **R-01** | Tauri v2 ecosystem instability or breaking changes mid-development | 3 | 4 | 12 | Mitigating | Pin Tauri version; subscribe to release notes; abstraction layer over Tauri APIs in `apps/desktop`; maintain a 2-week buffer in roadmap for upgrades |
| **R-02** | llama.cpp Rust binding regressions or API churn | 3 | 3 | 9 | Mitigating | Pin commit; mirror to private registry; isolate behind `LlmProvider` trait; fallback bindgen+cc route documented |
| **R-03** | ProseMirror performance hits for 200k+ word manuscripts | 3 | 4 | 12 | Mitigating | Per-chapter EditorViews with virtualised scroll list; perf budgets enforced in CI; benchmark fixture maintained |
| **R-04** | DOCX tracked-changes round-trip fidelity poor | 4 | 4 | 16 | Open | Phase 6 dedicated to this; real publisher fixtures; `pandoc-diff` comparator; fallback: ship "lossy round-trip" warning if fidelity slips |
| **R-05** | Local LLM quality vs. cloud (users disappointed by 7B output) | 4 | 3 | 12 | Mitigating | Set expectation in onboarding; offer cloud option for premium tasks; curate prompt presets specifically tuned for 7–8B; show clear "Local AI" labels |
| **R-06** | Hardware floor too high — users on 8 GB laptops can't run AI | 4 | 3 | 12 | Mitigating | Ship 3B model in catalogue; "AI off" mode is fully functional product; documented hardware tiers; CPU-only fallback acknowledged |
| **R-07** | Pandoc GPL licensing — sidecar approach contested | 2 | 5 | 10 | Open | Legal review before V1.0; sidecar precedent well-established; back-up plan: write minimal exporters for top 3 profiles ourselves |
| **R-08** | epubcheck JRE bundle inflates installer | 4 | 2 | 8 | Mitigating | Use jlink-stripped JRE (~30 MB); long-term: contribute to a Rust epubcheck port |
| **R-09** | Code-signing certificate cost or revocation | 2 | 4 | 8 | Mitigating | Procure all 3 certs by Phase 1; budget for renewals; use HSM-backed signing |
| **R-10** | Plugin author publishes a malicious plugin to marketplace | 3 | 5 | 15 | Mitigating | Manual review for first 6 months; signature revocation; capability sandbox; user-visible install prompt; community reporting |
| **R-11** | Plugin sandbox bypass found (WASM escape, WebView XSS → host) | 2 | 5 | 10 | Open | External pen-test pre-V1.0; bug bounty program post-V1.0; defense-in-depth (capability tokens + resource caps + CSP) |
| **R-12** | Sync (V1.5) — server-side breach exposes ciphertext | 3 | 3 | 9 | Mitigating | E2EE design; ciphertext only on server; key never on server; bug bounty |
| **R-13** | Cloud LLM provider terms change (start training on user data) | 3 | 4 | 12 | Mitigating | DPAs with no-train terms; contractual breach response; ability to switch provider with config change; audit log proves we didn't change |
| **R-14** | Offline license validation abused for piracy | 4 | 2 | 8 | Accepted | Periodic re-validation; offline grace period; honour-system-friendly free tier; piracy is not the target market |
| **R-15** | Internationalisation gaps in editor (CJK word count, RTL) | 3 | 3 | 9 | Mitigating | ICU break iterator; Phase 2 includes bidi tests; community translations |
| **R-16** | Manuscript corruption from a buggy plugin | 2 | 5 | 10 | Mitigating | Plugin write-suggestions only (not direct mutation); capability gating; pre-AI-edit snapshot also covers plugin edits |
| **R-17** | Ollama API changes break external runtime path | 3 | 2 | 6 | Accepted | Optional path; degrade gracefully to embedded; pin tested versions |
| **R-18** | Validator false positives anger users | 4 | 3 | 12 | Mitigating | Severity tuning; user-disable per validator; "fix" actions deterministic and reversible; community feedback channel |
| **R-19** | Auto-update bricks an installation | 2 | 5 | 10 | Mitigating | Signed updates; staged rollout; rollback supported; phased percentage rollout |
| **R-20** | KDP / IngramSpark / Apple change submission rules silently | 3 | 3 | 9 | Open | Quarterly profile review; community-reported regressions; profile versioning so users can pin |
| **R-21** | Disk corruption in user's filesystem corrupts project bundle | 3 | 4 | 12 | Mitigating | Markdown mirror as recovery surface; SQLite WAL; periodic snapshot to local backup directory; "Self-contained zip" affordance |
| **R-22** | Dependency supply-chain attack (npm/cargo malicious release) | 3 | 4 | 12 | Mitigating | Renovate with manual approval; locked checksums; SBOM published; mirror critical deps |
| **R-23** | macOS notarisation outage delays a release | 3 | 2 | 6 | Accepted | Two-week release buffer; Mac App Store path as fallback (later) |
| **R-24** | Editor data model change requires painful migration | 3 | 4 | 12 | Mitigating | Forward-compat principle (TAD §23); pre-migration snapshots; staged rollouts of schema changes |
| **R-25** | Team velocity insufficient for V1.0 timeline | 4 | 3 | 12 | Mitigating | Phase contingencies in 10-Roadmap; explicit "ship without X" cuts identified per phase |
| **R-26** | User confused by AI / cloud / Studio distinctions; reputation hit | 3 | 3 | 9 | Mitigating | Clear in-product labels; documentation; onboarding video; in-product "what is sent" transparency |
| **R-27** | Privacy regulator flags cloud LLM proxy as "data processor" | 3 | 3 | 9 | Mitigating | Run as data-processor with DPA; published DPIA; DPO contact |
| **R-28** | Heavy AI use crashes underpowered laptops (OOM) | 4 | 2 | 8 | Mitigating | RAM probe at model load; refuse to load if insufficient; suggest a smaller model |
| **R-29** | Snapshot storage grows unboundedly | 3 | 2 | 6 | Mitigating | Retention policy (FR-SNAP-005); manual purge UI; dedupe via content addressing |
| **R-30** | Editor save fails silently due to disk full | 3 | 5 | 15 | Mitigating | Disk-space probe before save; surface error; do not lose buffer until save confirmed |
| **R-31** | Contractor / employee exfiltrates source or signing key | 2 | 5 | 10 | Mitigating | HSM-backed signing; offboarding checklist; no static signing creds; audit logs |
| **R-32** | Cross-platform bug (font rendering, line break) hits one OS only | 4 | 2 | 8 | Mitigating | Visual regression tests per OS; community beta; conditional fixes documented |

## Top-5 attention list

These are the risks the project must keep in monthly view:

**R-04 DOCX tracked-changes** is the highest-impact-high-likelihood risk because Theo (Persona B) cannot adopt the product without it.

**R-10 Malicious plugin** is the highest *severity* risk because a single incident can permanently damage trust.

**R-25 Team velocity** is the perennial silent risk; phase contingencies in 10-Roadmap exist for this.

**R-30 Silent save failure** is catastrophic for trust if it ever happens; durability tests in 11-Test §6.1 directly address it.

**R-21 Bundle corruption** is the user's worst-case scenario; the Markdown mirror is the architectural answer.

## Process

Every phase exit reviews all open risks. Risks move to Closed only when their mitigation is testable and tested. New risks are added as discovered. The risk register is part of the documentation and ships with the repo.
