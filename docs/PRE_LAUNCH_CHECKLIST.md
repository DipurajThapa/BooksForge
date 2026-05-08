# BooksForge — Pre-Launch Checklist

> Single source of truth for "are we ready to ship?" Each row is a
> binary: ✅ done, ⏳ in flight, ⬜ not started, 🚫 deliberately
> deferred.
>
> Promotion gates: each gate's checklist must be **all ✅** before
> promotion. No "we'll fix it after launch" entries below.
>
> *Refs:* `MILESTONES.md`, `EXTERNAL_AUDIT_BACKLOG.md`,
> `outputs/MVP_SCOPE.md §6`, `docs/DISTRIBUTION.md §7`.

---

## Gate 1 — Internal alpha (developer-only smoke build)

The team can install a fresh build, run the seven journeys
end-to-end, and tag `v0.1.0-alpha.1`.

- [ ] **Stabilisation Sprint S1** complete — every working-tree
      file committed against an explicit milestone (MILESTONES.md
      → "Stabilisation Sprint").
- [ ] All MZ-01..MZ-04 acceptance criteria from
      `outputs/IMPLEMENTATION_PLAN.md` re-verified post-stabilisation.
- [ ] All MZ-05..MZ-08 acceptance criteria green per the latest
      `booksforge/BACKLOG.md`.
- [ ] `cargo test --workspace --all-targets` green on macOS-14,
      macOS-13, Windows-2022.
- [ ] `cargo test -p booksforge-orchestrator --test
      privacy_invariants` green.
- [ ] `cargo test -p booksforge-export-epub --test reproducibility`
      byte-for-byte cross-platform.
- [ ] `cargo deny check licenses bans advisories sources` clean.
- [ ] `pnpm typecheck && pnpm lint && pnpm test` green.

## Gate 2 — Closed beta (~50 invited writers)

Building hands-off enough to put in a stranger's hands without
embarrassment.

### Privacy track (HIGH severity, all must close)

- [ ] **Audit #7** — Startup network audit test.
- [ ] **Audit #8** — Static + dynamic guard against manuscript
      content reaching a non-loopback URL.
- [ ] **Audit #9** — Integration test for AI-off-until-consent.
- [ ] **Audit #11** — Consent JSON corruption surfaces a UI banner,
      not a silent default.
- [ ] **Audit #12** — `// SAFETY:` comments on every `unsafe` block
      in `booksforge-fs` and `booksforge-ollama`.

### Security hardening

- [ ] **Audit #13** — Path-traversal hardening on bundle import.
- [ ] **Audit #14** — Sidecar argument allowlist (Pandoc, EPUBCheck).
- [ ] **Audit #15** — CSP `'unsafe-inline'` removed from
      `style-src`.
- [ ] **Audit #16** — CODEOWNERS gate on capability changes ✅
      (landed in `chore/audit-backlog-drive-20260508`).
- [ ] **Audit #17** — Tauri command panic-guard for in-flight
      `job_id`.
- [ ] **Audit #18** — `let _ =` discards replaced with logged
      ignores.

### IPC contract + tests

- [ ] **Audit #19** — `OllamaStatusResponse` ts-rs-generated, not
      hand-written.
- [ ] **Audit #20** — `ollama_status` returns `Result`, not Ok-with-
      flag.
- [ ] **Audit #21** — Unit-test backfill on `storage`, `memory`,
      `template`.
- [ ] **Audit #22** — Frontend test coverage backfill (vitest +
      Playwright).

### UX completeness (the bare minimum to hand to a writer)

- [ ] **Audit #24** — Global React error boundary.
- [ ] **Audit #25** — Global toast queue replacing
      `.catch(() => null)` swallowing.
- [ ] **Audit #26** — Progress + cancel events for export and
      snapshot.
- [ ] **Audit #27** — Pre-submit input validation in the
      project-creation wizard.
- [ ] **Audit #29** — `ProposalReview` component with per-hunk
      accept/reject.

## Gate 3 — Public beta (free download, signed)

A stranger downloads, runs, and is legally / ethically protected.

### Release engineering

- [ ] **Audit #37** — Apple Developer ID + Windows EV certificate
      provisioned and stored in GitHub Secrets per
      `docs/DISTRIBUTION.md §3`.
- [ ] **Audit #38** — `release.yml` produces signed + notarised
      artefacts on all three matrix platforms ✅ scaffold landed in
      `chore/release-readiness-20260508` — needs secrets +
      first dry-run.
- [ ] **Audit #39** — Tauri auto-updater configured with public
      key + endpoint; opt-out toggle prominent in Settings.
- [ ] **Audit #40** — `cargo deny check advisories` explicit step
      ✅ landed in `chore/onboarding-and-runbook-20260508`.
- [ ] **Audit #41** — Dependabot weekly grouped PRs ✅ landed in
      `chore/audit-backlog-drive-20260508`.
- [ ] **Audit #44** — Production `tauri.conf.json` metadata —
      `productName`, `bundleIdentifier`, `publisher`, `copyright`,
      `category`, `version` matching `Cargo.toml`.

### Legal + governance

- [ ] **Audit #2** — `LICENSE` final terms (replace placeholder)
      ⏳ placeholder shipped in PR #1; final terms pending.
- [ ] **Audit #4** — `THIRD_PARTY_LICENSES.md` aggregated from
      `cargo-about` + `pnpm licenses list` ⏳ scaffold landed in
      `chore/audit-backlog-drive-20260508`; aggregate pending.
- [ ] **Audit #46** — Privacy Policy + EULA + Terms reviewed by
      legal counsel ⏳ drafts landed in
      `chore/release-readiness-20260508`; legal review pending.
- [ ] `@booksforge.app` email addresses provisioned: `privacy@`,
      `support@`, `security@`, `conduct@`, `legal@`,
      `licensing@`.
- [ ] `booksforge.app` domain + DNS + TLS live with verifiable
      HTTPS download landing page.

### Distribution

- [ ] **Audit #45** — Distribution & release plan ✅ landed in
      `chore/release-readiness-20260508`.
- [ ] First signed `release.yml` dry-run produced + spot-checked
      artefacts on all three matrix platforms.
- [ ] Auto-updater manifest signed with the Tauri private key;
      first stable channel published.

### Accessibility

- [ ] **Audit #34** — axe-core CI gate; Playwright a11y scan;
      VoiceOver + NVDA pass on the golden path.

## Gate 4 — 1.0 General Availability

Charge for it / accept paid users.

### Polish

- [ ] **Audit #28** — Onboarding tour re-openable from Help menu.
- [ ] **Audit #30** — Memory + Vocabulary panels with manual CRUD.
- [ ] **Audit #31** — Snapshot per-node selective restore.
- [ ] **Audit #32** — Export wizard live preview + dependency
      probe at start time.
- [ ] **Audit #33** — Centralised keymap + `?` help overlay.
- [ ] **Audit #35** — Dark-mode toggle (System / Light / Dark).
- [ ] **Audit #36** — i18n scaffolding even if MVP ships
      English-only.
- [ ] **Audit #43** — Crash reporting opt-in pipeline with
      redaction (design landed in `docs/CRASH_REPORTING_DESIGN.md`).
- [ ] **Audit #51** — Source maps generated + uploaded to private
      artefact store, not shipped.
- [ ] **Audit #52** — Frontend bundle-size monitor in CI.
- [ ] **Audit #54** — Snapshot creation idempotency keys.
- [ ] **Audit #55** — Agent context-size guard (production, not
      just tests).
- [ ] **Audit #57** — Frontend session-id logging.
- [ ] **Audit #59** — Pre-commit hooks ✅ landed in
      `chore/release-readiness-20260508`; needs adoption by
      contributors.
- [ ] **Audit #60** — Empty / loading / error states for every
      panel.

### Business + community

- [ ] **Audit #47** — In-app help / docs system (offline content).
- [ ] **Audit #48** — Public website / landing page live with
      direct download CTA.
- [ ] **Audit #49** — Pricing / monetisation decision documented in
      `docs/BUSINESS_MODEL.md`; license-key flow designed.
- [ ] **Audit #50** — Support channel + SLA documented ✅ landed in
      `chore/onboarding-and-runbook-20260508`; emails provisioned.
- [ ] **Audit #56** — Prompt-template archive with audit query —
      structural part landed in
      `chore/onboarding-and-runbook-20260508`; query
      implementation by team.

---

## Cross-cutting "must remain green"

Every gate above assumes the following stay green throughout
development:

- All CI gating jobs in `booksforge/.github/workflows/ci.yml`
  + `.github/workflows/security-scan.yml`.
- Privacy invariant tests (P1–P5 in `MILESTONES.md`).
- EPUB reproducibility test cross-platform.
- IPC codegen drift test.
- `kill -9` zero-data-loss test.
- Cold-launch p50 ≤ 1 s on `macos-14`.

If any of those break for more than 24 hours, all promotion gates
are paused until they are green.

---

## How this checklist is used

1. Open this file at the start of any release-readiness review.
2. Check off each row that is genuinely done. Be strict — "the
   draft exists" ≠ "legal counsel signed off".
3. The lowest gate with at least one ⬜ is the one we're working on.
4. Promote (e.g. tag a beta release) only when the corresponding
   gate's section is **all ✅**.
5. After each promotion, write a one-line entry in
   `booksforge/CHANGELOG.md`.

---

*Last updated 2026-05-08. Update on every promotion + every audit-
item closure.*
