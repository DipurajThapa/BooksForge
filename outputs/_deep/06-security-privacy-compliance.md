# Security, Privacy & Compliance — BooksForge

**Version:** 1.0.0-draft  •  **Status:** For review  •  **Date:** 2026-05-06

---

## 1. Scope and posture

BooksForge handles unpublished manuscripts — among the most sensitive content a writer owns — and runs third-party plugins on the user's behalf. The security posture is **default-deny with informed consent**: nothing leaves the device, nothing executes with network access, nothing reads the manuscript, until the user explicitly approves it in a clear UI prompt with rationale.

This document covers: threat model, trust boundaries, controls, plugin sandbox, encryption, telemetry, vulnerability handling, and compliance posture. It is the authoritative source on security; any deviation in implementation requires an entry in the security exception log.

## 2. Trust boundaries

```
┌────────────────────────────────────────────────────────────────────┐
│ Boundary A: User device                                            │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │ Boundary B: BooksForge process                                │  │
│  │  ┌────────────────────────────────────────────────────────┐  │  │
│  │  │ Boundary C: WebView (frontend)                         │  │  │
│  │  │  ┌──────────────────────────────────────────────────┐  │  │  │
│  │  │  │ Boundary D: Plugin WebView (UI plugin)           │  │  │  │
│  │  │  └──────────────────────────────────────────────────┘  │  │  │
│  │  └────────────────────────────────────────────────────────┘  │  │
│  │  ┌────────────────────────────────────────────────────────┐  │  │
│  │  │ Boundary E: Plugin WASM (compute plugin)               │  │  │
│  │  └────────────────────────────────────────────────────────┘  │  │
│  └──────────────────────────────────────────────────────────────┘  │
│  Files outside bundle      Other apps                              │
└────────────────────────────────────────────────────────────────────┘
       │
   Boundary F: Network (cloud LLM, sync, marketplace)
```

The Rust core trusts itself and Tauri-validated allowlists. The WebView is treated as **semi-trusted** — XSS in our own UI must not lead to arbitrary file or network access. Plugins are **untrusted** by default; capabilities granted by the user are the only way they gain reach.

## 3. Threat model (STRIDE, condensed)

| Threat | Vector | Asset at risk | Mitigation |
|--------|--------|---------------|------------|
| **Spoofing** | Fake plugin claiming to be from a known publisher | Plugin install | Marketplace plugins signed; sideload shows clear "unsigned" warning |
| **Spoofing** | Phishing site impersonating BooksForge for license key | License | Activation never via in-app link; license entered manually or via signed magic link |
| **Tampering** | Modified app binary | Updates, bundle integrity | Tauri updater verifies signature; manifest hash checked on open |
| **Tampering** | Plugin modifies project file outside its capabilities | Manuscript | Plugin host enforces FS capability scope; writes go through host API |
| **Repudiation** | User claims AI changed something they did | Audit | `ai_calls` audit log + pre-AI-edit snapshots make every AI write reconstructable |
| **Information disclosure** | Manuscript exfiltrated by malicious plugin | Manuscript | Plugins default-deny network; capability `network-domain:X` is host-mediated and logged |
| **Information disclosure** | Cloud LLM provider stores prompt | Manuscript | Use enterprise endpoints with no-train terms; user-visible "what is sent" preview before each call |
| **Information disclosure** | Malware reads `project.db` | Manuscript | Optional at-rest encryption (AES-256-GCM) with passphrase |
| **Information disclosure** | Crash dump contains manuscript | Manuscript | Crash dumps scrubbed before any upload; upload is opt-in |
| **Denial of service** | Pathological manuscript crashes editor | Availability | Bounded ProseMirror state, validator timeouts, plugin runtime resource caps |
| **Elevation of privilege** | Plugin gains host-level FS access | Whole device | WASM sandbox + capability tokens; UI plugin in isolated WebView with strict CSP |

## 4. Controls

### 4.1 At rest

Optional **per-project encryption**: Argon2id (m=64MiB, t=3, p=1) → 256-bit master key → AES-256-GCM per blob. SQLite database via SQLCipher with the same master key. Salt and KDF parameters in `manifest.toml`. The master key never persists to disk; user re-enters passphrase on open or grants OS-keyring storage.

License keys and OAuth tokens: stored in OS keyring (DPAPI on Windows, Keychain on macOS, Secret Service on Linux). Never in plain config files.

### 4.2 In transit

All cloud calls over **TLS 1.3** with certificate pinning for the BooksForge backend (sync, marketplace) and certificate validation for third-party LLM endpoints. No cleartext fallback. Plugin updates served over signed HTTPS with manifest signature verified before unpacking.

### 4.3 In memory

Manuscript content held in process memory only. We do **not** zeroise on free in V1 (would require pervasive `Zeroizing<T>` and isn't a meaningful win against an attacker with debugger access). Memory dumps are not shipped to vendors — crash reports use minidumps that exclude heap.

### 4.4 Updates

Tauri updater verifies a code-signing signature against a hard-coded public key shipped with the binary. Update server is HTTPS with certificate pinning. Failed verification → rollback, alert user. **[GUARD]** No unsigned binary may be loaded.

### 4.5 Code signing

Windows EV certificate; macOS Developer ID + notarisation + stapling; Linux GPG-signed packages. Build pipeline secrets in GitHub Actions OIDC + a hardware-backed signing service (e.g., AWS KMS HSM-backed, or 1Password CLI for small-team start). Signing keys are never on developer laptops.

## 5. Plugin sandbox

Plugins fall into two execution models. Each has its own constraints.

### 5.1 UI plugins (WebView)

Run in an **isolated Tauri WebView** with a separate origin from the host UI. Cross-origin messaging via Tauri IPC only (host mediates). Strict Content-Security-Policy: `default-src 'none'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: capability:`. Plugin cannot make `fetch` or `XMLHttpRequest` directly — it asks the host through the capability API. The host enforces capability scope per call.

### 5.2 Compute plugins (WASM)

Run in `wasmtime` with WASI-preview2. Default deny: no filesystem, no clocks, no network, no environment, no random. Capabilities are explicit: `read-manuscript`, `read-bibliography`, `write-suggestions` (cannot write directly to manuscript — must go through suggestion API), `network-domain:foo.com`, `read-asset:image/*`. Resource caps: 256 MB memory, 30 s wall clock, 10s CPU; killed if exceeded.

### 5.3 Capability prompts

On install (FR-PLUG-002) the host shows the user every requested capability with the plugin's stated rationale. Capabilities can be revoked at any time from Settings → Plugins. Revoking does not uninstall, but the plugin stops working until granted again.

### 5.4 Plugin signing

Marketplace plugins are signed with a BooksForge marketplace key (publisher submits a CSR; we sign after review). Sideload plugins may be unsigned but the install dialog shows a stark warning. The signature covers the whole package contents.

## 6. AI safety

Three categories of risk: **prompt injection** (untrusted text inside the manuscript or imported docs trying to alter the AI's behaviour), **content disclosure** (unintended content sent to an LLM), and **trust** (AI outputs being treated as authoritative).

**Prompt injection mitigation.** Every prompt template separates trusted instructions from untrusted content with explicit fences (`<<<USER_CONTENT>>> ... <<<END_USER_CONTENT>>>`) and a system-level instruction to ignore any embedded "instructions" within fenced content. Imported web content (research module, V1.5) is treated as untrusted by default. Multi-document context limits are enforced to avoid sneaking instructions through obscure documents.

**Content disclosure mitigation.** Before every cloud AI call, the user can preview exactly the content being sent (FR-AI-008). The preview is produced from the same call site that ships the data — there is no way to send more than was previewed.

**Trust mitigation.** AI outputs never auto-apply (FR-AI-015 + the cross-flow invariant in 05-Workflow §14). Every AI-applied edit produces a snapshot. The audit log stores prompt template hash + model + duration so a user can later reconstruct what happened.

## 7. Privacy posture

### 7.1 By default

Nothing leaves the device. Telemetry off. Crash reporting off. Plugin marketplace queries off until the user opens the marketplace UI. AI is local-only until the user opts in to a cloud provider.

### 7.2 Telemetry (when enabled)

Anonymous, granular, revocable. Sent events are: app launch, project open (no project name), feature usage counts, performance percentiles, crash signatures (without content). Never sent: manuscript content, file paths under user home (except project name with explicit permission), email, identifiable plugin lists. The user sees the exact event schema in Settings → Privacy and can disable per category. The transport is HTTPS to a privacy-friendly aggregator (e.g., self-hosted PostHog or Plausible-style); we do not use Google Analytics or similar.

### 7.3 Crash reporting (when enabled)

Minidumps with stack symbolication. Manuscript heap excluded. Path scrubbing applied. Reports go to a self-hosted Sentry-compatible service.

### 7.4 Cloud sync (Studio tier)

Project bundles encrypted client-side with the user's master key before upload. The server stores opaque ciphertext; servers cannot decrypt. Conflict resolution uses encrypted CRDT operations (V1.5 design — see TAD §7 for collaboration roadmap). No content is used for any analytics or training. Data residency: EU and US regions selectable; default by user IP.

### 7.5 Compliance

- **GDPR / UK-GDPR**: lawful basis is contract for paid tiers, consent for telemetry; DPA available; data subject access via Settings → Export My Data; right to erasure honoured by deleting account + sync ciphertext.
- **CCPA**: California users have the same export and delete rights; no sale of personal data ever.
- **Children**: not directed at children; we do not knowingly collect from under-13s. EU age of digital consent honoured.

## 8. Vulnerability disclosure and incident response

Public security policy at `booksforge.app/security` with a reporting email and PGP key. Acknowledgement target: 48 h. Triaged severity within 5 business days. Critical issues patched and released within 14 days; we publish CVEs for impacted versions. Hall of fame for researchers who follow the policy. Incident playbook in the runbook repository covers triage, customer notification, and post-mortem within 30 days.

## 9. Supply-chain security

Dependencies pinned with checksum verification. `cargo-deny` and `npm audit` gate CI. Renovate weekly. SBOM (CycloneDX) generated per release and shipped with the installer. We mirror critical dependencies (llama.cpp, Pandoc) to a private registry to survive upstream disappearance.

## 10. Identity and authentication

Free tier: no account.
Pro tier: email + magic link (recommended) or password (Argon2id). No SSO required in V1; SSO is V2.0 enterprise.
Studio tier: same as Pro plus optional 2FA (TOTP).
**Passwords** are never stored; only Argon2id hashes. Magic links expire in 10 minutes and are single-use.

## 11. Data retention

Free tier: nothing on our servers.
Pro tier: license records (email, license key, purchase metadata) retained while the licence is active and 5 years after for tax/audit.
Studio tier: encrypted project ciphertext retained while subscription is active; on cancel, ciphertext deleted within 30 days.
Telemetry: aggregated and anonymous; raw events retained 90 days then aggregated.
Crash reports: retained 90 days then deleted unless ticketed.

## 12. Audit and logging

Local rotating logs (TAD §16) with PII redaction. AI calls audit table per project (Data Model §4). Plugin grants and revocations logged in the user's data dir. Export pipeline logs every export with parameters and validator results. None of these are sent off-device unless the user uploads a diagnostic bundle.

## 13. Hardening checklist (pre-V1 release)

The release is blocked until each item is checked and signed by the security lead:

The Tauri allowlist is the minimum required for shipping features. CSP applies to host and plugin WebViews. WASM resource caps are tested with adversarial fixtures (allocator bombs, infinite loops, recursive imports). Plugin capability bypass attempts are part of the security test suite. Update signature verification has a negative test (unsigned update is rejected). Cloud sync end-to-end test confirms server cannot decrypt. Threat model is reviewed by an external party (a paid penetration test before V1.0 GA). Telemetry payloads are inspected for any leakage with a network capture during a long session. Crash dump scrubber tested with a manuscript containing distinctive markers. License activation flow does not leak email to a third party.

## 14. Known residual risks

These remain after controls and are accepted explicitly:

A user with full local privileges can read another user's project on a shared machine if encryption is not enabled — encryption is opt-in by design, and we surface this in onboarding. A determined attacker with persistent malware on the device can capture keystrokes — outside our threat model. AI cloud providers are subject to their own subpoena and breach risks — we mitigate via no-train terms and selectable providers but cannot eliminate. Pandoc and other GPL sidecars run as separate processes; their CVEs are tracked but their attack surface is not our code.
