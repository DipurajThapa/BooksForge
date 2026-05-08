# Security Policy

BooksForge is a local-first writing tool. Its defining promise is that
**no manuscript content leaves the device by default**. We take security
and privacy reports seriously and treat them as first-class issues.

## Reporting a vulnerability

**Do not open a public GitHub issue for security vulnerabilities.**

Report privately by either:

1. **GitHub Security Advisories** (preferred) — open a draft advisory
   from the repository's *Security → Advisories* tab. This stays private
   between you and the maintainers until a fix is published.
2. **Email** — send a detailed report to **security@booksforge.app**
   *(if this address is not yet provisioned, use GitHub Security
   Advisories until it is — see EXTERNAL_AUDIT_BACKLOG.md #5)*.

Include in your report:

- A description of the vulnerability and its impact.
- Steps to reproduce, or a proof-of-concept.
- The version / commit you tested against.
- Whether you believe the vulnerability is being exploited in the wild.

## Response targets

- **Acknowledgement:** within 3 business days.
- **Triage decision:** within 7 business days.
- **Fix or mitigation timeline:** shared with the reporter within 14 business days.
- **Public disclosure:** coordinated with the reporter; we publish a
  Security Advisory and a release containing the fix on the same day.

We do not currently run a paid bug-bounty programme, but we will credit
reporters in the published advisory unless they prefer to remain
anonymous.

## Scope

In scope:

- Code in this repository, including the Tauri desktop binary, every
  crate under `booksforge/crates/`, and the React frontend under
  `booksforge/apps/desktop/src-ui/`.
- Bundled assets and configuration (`tauri.conf.json`, capability JSONs,
  CSP, the bundled Pandoc / EPUBCheck sidecars).
- The privacy invariants stated in `outputs/SECURITY_PRIVACY.md`. In
  particular, any way to cause manuscript content to leave the device
  without explicit user consent is a high-severity issue.

Out of scope:

- Vulnerabilities in Ollama itself — please report those upstream.
- Vulnerabilities in the user's operating system, GPU drivers, or other
  third-party software.
- Social-engineering attacks against maintainers or users.
- Denial-of-service via running BooksForge on hardware below the
  documented minimum specs.

## Safe-harbour

We will not pursue legal action against good-faith security researchers
who:

- Report vulnerabilities to us privately as described above.
- Do not access, modify, or destroy data they do not own.
- Do not exploit the vulnerability beyond what is necessary to confirm
  it.
- Give us a reasonable opportunity to fix the issue before public
  disclosure (typically 90 days, less for actively exploited issues).

## Supported versions

While BooksForge is pre-1.0, we patch security issues only on the latest
released version. Once 1.0 ships, we will publish a support matrix here.

## Known good practice (for users)

- Keep BooksForge up to date once auto-updates are configured
  (EXTERNAL_AUDIT_BACKLOG.md #39).
- Keep Ollama and your local models pinned to the version BooksForge
  ships with, unless you understand the change.
- Project bundles (`*.booksforge/`) from untrusted sources should be
  treated with the same care as any other downloaded archive — open
  them only if you trust the source.

---

*Refs: EXTERNAL_AUDIT_BACKLOG.md #5 (SECURITY.md missing); the privacy
invariants section of `outputs/SECURITY_PRIVACY.md`.*
