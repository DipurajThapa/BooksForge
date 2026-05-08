# BooksForge — Operations Runbook

> **Audience.** The person on call when something is on fire. Today
> that's the founder; eventually a small support team.
>
> **Contract:** every entry below has *Symptom*, *Triage*, *Action*,
> *Recovery*, and *Prevention*. If a section drifts from this
> structure during an incident, fix the structure afterwards.
>
> **Refs:** `SECURITY.md` (vulnerability disclosure path),
> `PRIVACY_POLICY.md`, `docs/DISTRIBUTION.md` (release + rollback
> mechanics), `EXTERNAL_AUDIT_BACKLOG.md`.

---

## 0. Severity definitions

| Severity | Meaning | Response target |
|----------|---------|-----------------|
| **SEV-0** | Data loss / data corruption / privacy breach in production. | Drop everything. <1h to mitigation. |
| **SEV-1** | Released build cannot launch, cannot save, or cannot export on a supported OS. | <4h to mitigation. |
| **SEV-2** | Major feature broken with no workaround. | <24h to mitigation. |
| **SEV-3** | Minor breakage with a workaround. | Next release. |

If you're not sure of severity, **err high** and downgrade later.

---

## 1. A released build leaks manuscript content over the network

> **The defining product promise has been violated. This is SEV-0.**

**Symptom.** A user reports that traffic to a non-loopback host
appeared after they typed in BooksForge. Or a security researcher
provides a reproducer.

**Triage.**
1. Confirm the report. Ask for the BooksForge version (*Help →
   About*), OS, the diagnostic bundle from *Settings → Diagnostics
   → Save diagnostic bundle*, and the network capture.
2. Determine which version(s) are affected. Cross-reference with
   the GitHub Release page.

**Action (≤ 1 hour).**
1. **Yank** the affected GitHub Release(s): mark each as `draft`.
   The Tauri updater stops serving them within minutes.
2. **Disable the auto-updater feed** for the affected channel
   (`stable` / `beta` / `dev`) by serving an empty `latest.json`
   from the update endpoint, so users on the affected version are
   not pushed onto an even-newer affected build by mistake.
3. **Post a Security Advisory** on the GitHub repo — this is what
   notifies users via the GitHub UI. Include: affected versions,
   what data could have been transmitted, what users should do
   (uninstall, then reinstall the patched version when available).
4. **Notify the reporter** that the issue is being mitigated. If
   they followed `SECURITY.md`, they're already in a private
   channel; keep them in the loop.

**Recovery.**
1. Identify the leak path in code (likely a missing privacy-
   invariant test — see audit #7/#8/#10).
2. Tag a `vX.Y.Z+1` patch from the previous good commit + minimum
   fix. Run `release.yml`. Spot-check artefacts; publish.
3. Restore the auto-updater feed pointing at the patched version.
4. Update the Security Advisory with the patch version + CVE if
   one was assigned.
5. Add a CI test that catches *exactly this leak* and prevents
   regression. Reference it in the advisory.

**Prevention.**
- Audit #7 (startup network audit), #8 (manuscript content guard)
  must close before this scenario can recur silently.
- The five privacy invariants in `outputs/SECURITY_PRIVACY.md`
  must each have a CI test (`tests/privacy_invariants.rs`).
  Status: 2 of 5 invariants currently have direct tests; the
  other 3 are open audit items.

---

## 2. A released build destroys a user's project bundle

> **SEV-0.**

**Symptom.** A user reports that opening a project resulted in
data loss, a corrupt `project.db`, or a missing `manifest.toml`.

**Triage.**
1. Ask for the diagnostic bundle and (with the user's permission
   and *only* if it does not contain sensitive content) a copy of
   the project bundle in its current state.
2. Check the snapshot system: `snapshots/objects/` should still
   contain content-addressed blobs even if the database is broken.
3. Check `~/.booksforge/logs/` for the precise sequence of
   operations before the corruption. PII redaction is on by
   default.

**Action.**
1. Yank affected releases as in §1.
2. Provide the user with a **manual recovery script** that:
   - copies the bundle aside (never destroy the user's bad state),
   - extracts every snapshot blob,
   - reconstructs the project from the most recent valid snapshot.
3. Post a Security Advisory categorised as data-loss.

**Recovery.**
1. Patch the bug. The likely surface area:
   - `booksforge-fs` atomic-bundle-creation (orphan-temp-dir
     cleanup, lock-file lifecycle),
   - `booksforge-storage` migration runner,
   - `booksforge-snapshot` restore path.
2. Add a regression test on the smallest reproducer.
3. Tag, release, restore the updater feed.

**Prevention.**
- `kill -9` zero-data-loss test (MVP_SCOPE.md §6 #6) should
  always be a CI gate.
- Atomic bundle-creation contract documented in
  `outputs/IMPLEMENTATION_PLAN.md MZ-02` must be tested by an
  integration test that simulates SIGKILL between every step.

---

## 3. The auto-updater is pulling users onto a broken build

> **SEV-1 minimum, often SEV-0 if the broken build also corrupts
> data.** Same shape as §1 / §2 but the failure surface is the
> updater itself.

**Action.**
1. Serve an empty `latest.json` for the affected channel
   immediately. Users on the broken version stop receiving update
   prompts.
2. Investigate whether the manifest signature is correct (Tauri
   updater key compromise would be **critical** — escalate per §6
   if so).
3. Tag a `vX.Y.Z+1` patch and re-publish a fresh, signed
   `latest.json` once the patch is verified.

---

## 4. Code-signing or notarisation is failing in CI

> **SEV-2 — release engineering.**

**Symptom.** `release.yml` fails at the signing step with messages
like `notarytool: notarization failed`, `signtool: certificate not
found`, or `tauri-action: Apple certificate password incorrect`.

**Triage.**
1. Check the GitHub Actions log. The exact error message is
   usually self-explanatory.
2. Check the expiry date on the relevant cert — Apple Developer ID
   typically lasts 5 years; Windows EV certs typically 1–3 years.
3. If the cert is current, the secret may have rotated. Verify
   by re-uploading a fresh export.

**Action.**
1. Re-export the `.p12` (macOS) or `.pfx` (Windows) and update
   the corresponding GitHub Secret. Do **not** check the cert
   into the repo, ever.
2. Re-run the failed release-pipeline job.

**Recovery / prevention.**
- `docs/DISTRIBUTION.md §3` lists every secret and its rotation
  policy.
- Calendar reminders 30 days before each cert expires. Add this
  to the founder's calendar; do not rely on memory.

---

## 5. CI is permanently red

**Symptom.** Multiple consecutive PRs fail CI on the same job, and
re-running doesn't help.

**Triage.**
1. Was a CI workflow recently modified (look for changes under
   `.github/workflows/`)? If yes, suspect that change first.
2. Was Rust toolchain or Node pinned-version recently bumped (look
   at `rust-toolchain.toml`, `package.json`, `pnpm-workspace.yaml`)?
3. Are external services (GitHub Actions, crates.io, npm registry)
   degraded? Check status pages.

**Action.**
1. If a workflow regression: `git revert` the offending commit
   and confirm CI green.
2. If a toolchain regression: pin the previous version, fix
   forward in a separate branch.
3. If external degradation: wait. Do not work around by
   disabling gates.

---

## 6. Tauri updater private key is compromised

> **SEV-0 — this is among the most damaging things that can
> happen.** Any attacker holding the key can serve a malicious
> update to every BooksForge user.

**Action.**
1. **Stop the auto-updater feed immediately** (empty
   `latest.json`).
2. Generate a new keypair (`tauri signer generate`).
3. Embed the new public key in `tauri.conf.json` and tag a fresh
   release.
4. **Push the fresh release manually** — auto-update will not
   work for current users because they trust the old key. They
   need to download the new version directly from the website
   or the GitHub Release page.
5. Post a Security Advisory.
6. Rotate every other secret as a precaution: code-signing certs,
   Apple notarisation password, Windows cert.

**Recovery.**
- The transition window where users are on the old key + signed
  manifests is the dangerous one. Communicate clearly: "download
  the new version from the website".
- After the new version reaches enough users, the old key can be
  treated as permanently dead.

**Prevention.**
- The Tauri private key lives **only** in GitHub Secrets and on
  the founder's offline backup. Never on a developer machine.
  Never in CI artefact storage. Never in a backup that's stored
  in the same place as the public key.
- Rotate proactively annually, not just on suspected compromise.

---

## 7. A privacy or security report comes in via the public issue
tracker

**Action.**
1. **Within 5 minutes:** triage the issue on GitHub, mark it
   private (close + delete if necessary; GitHub keeps content
   in the audit log even after deletion), and message the
   reporter privately to move them to the
   `Security → Advisories` flow per `SECURITY.md`.
2. Treat the report as having been public for the duration it
   was visible. If the issue was visible for hours, assume
   bad actors have it.
3. Proceed with normal triage per §1 if the report is real.

**Prevention.**
- The `.github/ISSUE_TEMPLATE/config.yml` routes security
  reports to the private channel, but the user has to read it.
  The `SECURITY.md` link is also surfaced from the *Help →
  About* dialog inside the app.

---

## 8. Day-to-day: a user reports a generic bug

**Action.**
1. Confirm via the bug-report template that we have: app
   version, OS, Ollama version, steps to reproduce, diagnostic
   bundle.
2. Triage to the appropriate label (`bug` / `enhancement` /
   `wontfix`).
3. If it falls under the explicit out-of-MVP list
   (`outputs/MVP_SCOPE.md §3`), close with a polite pointer to
   that section.
4. If it's a real bug, prioritise per severity table (§0).

---

## 9. Pre-release sanity sweep (before tagging)

Run from `booksforge/` root:

```bash
lefthook run release-preflight
```

This re-runs every CI gate locally before you push the tag.
Equivalent to the `preflight` job in `release.yml` and the
content of `CONTRIBUTING.md`'s PR checklist.

If anything fails, **do not tag**. Investigate first. The release
pipeline will fail anyway, but tagging without preflight just
burns CI minutes.

---

## 10. Backups (founder-personal)

Items the founder MUST back up offline (USB drive, paper-based
secure storage, or a separate cloud account with hardware-key
2FA):

| Item | Where it lives in production | Restore implication if lost |
|------|------------------------------|-----------------------------|
| Apple Developer ID `.p12` + password | GitHub Secrets | Cannot sign new macOS releases until re-issued ($99/yr + verification time). |
| Windows EV certificate `.pfx` (or HSM credentials) + password | GitHub Secrets | Cannot sign new Windows releases until re-issued ($300–500/yr + EV verification, days). |
| Tauri updater private key + password | GitHub Secrets | Cannot push auto-updates to existing users; see §6. |
| `booksforge.app` domain registrar credentials | TBD | Domain hijacking risk; can lose canonical download host. |
| GitHub repository admin access | TBD | Cannot tag releases or rotate secrets. |

Rotation cadence: annually for certs, on suspected-compromise for
keys, never for domain credentials (just keep the registrar
account secured).

---

## 11. Communicating with users during an incident

**Tone.** Direct, factual, no excuses, no minimising. Describe what
happened, what users should do, and what we're doing.

**Channels.**
1. **GitHub Security Advisory** — primary, automatically notifies
   users with watch enabled.
2. **GitHub Release notes** — secondary, for the patch release.
3. **`booksforge.app`** — banner notice if the website is up.
4. **Direct email to known users** — if and only if we have an
   opt-in mailing list (V1+; not in MVP).

**Do not** post incident updates to social media before the
GitHub channels have the full story. Twitter/Mastodon updates
should link back to the GitHub Security Advisory, not duplicate
its content.

---

## 12. Post-incident review

Within 7 days of any SEV-0 or SEV-1, write a public post-mortem
covering:

- What happened (factual sequence).
- What was the impact (number of users, data exposed, etc.).
- Why it happened (root cause, not just proximate).
- What we did to mitigate.
- What we're changing so it cannot recur (CI test, process change,
  etc.).

The post-mortem lives in `docs/post-mortems/YYYY-MM-DD-<slug>.md`
and is linked from the related Security Advisory.

---

*Last updated 2026-05-08 (initial scaffold). Update this document
whenever an incident teaches us a new playbook entry.*
