# BooksForge — Support

> **Audience:** users who need help with BooksForge.
> **Refs:** `SECURITY.md`, `PRIVACY_POLICY.md`,
> `docs/RUNBOOK.md` (maintainer-side incident response),
> `EXTERNAL_AUDIT_BACKLOG.md #50`.

---

## 1. Quick reference — where do I go?

| What | Where | Response time |
|------|-------|--------------|
| **Security or privacy vulnerability** | [GitHub Security Advisory](https://github.com/DipurajThapa/BooksForge/security/advisories/new) (preferred) or `security@booksforge.app` *(pending)* | Acknowledgement ≤ 3 business days. |
| **Bug report** | [Open a GitHub issue](https://github.com/DipurajThapa/BooksForge/issues/new/choose) using the *Bug report* template. | Triage ≤ 7 business days. |
| **Feature request** | [Open a GitHub issue](https://github.com/DipurajThapa/BooksForge/issues/new/choose) using the *Feature request* template. | Triage ≤ 14 business days. Many requests will be closed with a pointer to `outputs/MVP_SCOPE.md §3` if they are explicitly out of MVP. |
| **"How do I…" question** | [GitHub Discussions](https://github.com/DipurajThapa/BooksForge/discussions) (post-launch) | Best effort by the maintainer + community. |
| **General contact** | `support@booksforge.app` *(pending)* | ≤ 14 business days, but slower paths usually go via GitHub. |

---

## 2. Before you contact support

### 2.1 Try the in-app diagnostics first

1. *Help → About* — confirm BooksForge version + Ollama status.
2. *Settings → Diagnostics → Save diagnostic bundle* — produces a
   `.zip` containing recent logs, app config, and Ollama health
   info, **with manuscript content automatically redacted**. This
   is the fastest way for us to understand the problem; attach it
   to your bug report.
3. *Help → Recent issues* — known issues for your version.

### 2.2 Check the FAQ in `docs/USER_HELP.md`

Most "BooksForge can't find Ollama" / "export fails" / "agent
panel is empty" questions have known answers. Check there before
opening an issue.

### 2.3 Search existing issues

A quick search of [open issues](https://github.com/DipurajThapa/BooksForge/issues)
often turns up an existing thread covering your question.

---

## 3. Service-Level Targets (SLT)

These are **targets**, not contractual obligations — BooksForge
is pre-1.0 and maintained by a small team. We aim for:

| Channel | First response | Triage decision | Resolution / fix |
|---------|----------------|-----------------|------------------|
| Security advisory | ≤ 3 business days | ≤ 7 business days | ≤ 14 business days for a fix or coordinated public disclosure |
| Bug report (data loss / cannot launch) | ≤ 1 business day | same day | hotfix on the next release |
| Bug report (regular) | ≤ 7 business days | ≤ 14 business days | next minor release |
| Feature request | ≤ 14 business days | ≤ 30 business days | per `MILESTONES.md` priority |

For **paid users** post-V1 (if the pricing model includes paid
support), separate SLAs will be published in the EULA.

---

## 4. Privacy when you contact us

- **Diagnostic bundles** are automatically PII-redacted (manuscript
  text, file paths inside the bundle, project IDs). You can open
  the bundle as a `.zip` and review every file before sharing.
- **Screenshots** in bug reports may contain manuscript text; please
  blur / crop sensitive passages before posting.
- **Email** (when provisioned): we keep your message and any
  attachments only as long as needed to resolve the issue, then
  delete them. Not stored in a third-party support tool.
- We will **never** ask you to share your project bundle wholesale.
  If a problem genuinely requires that, we will offer a private
  encrypted upload with a documented retention period.

See `PRIVACY_POLICY.md` for the full posture.

---

## 5. Known limitations of the support model (MVP)

- We do not run a 24/7 on-call rotation. SEV-0 issues (data loss,
  privacy breach) are escalated as fast as the maintainer can act,
  but realistic mitigation time is hours, not minutes.
- We do not offer per-user remote debugging. The diagnostic bundle
  is the primary forensic artefact.
- We do not currently maintain a community Discord / Slack / Matrix.
  Discussions on the GitHub repo serve that purpose post-launch.
- Localised support is English-only in MVP. (i18n is on the
  roadmap — see `EXTERNAL_AUDIT_BACKLOG.md #36`.)

---

## 6. Support channels we do NOT use

To keep the privacy posture clean, BooksForge **does not** use:

- Third-party live-chat widgets (Intercom, Drift, etc.) on the
  website.
- Third-party support-ticket SaaS (Zendesk, Freshdesk, etc.) for
  bug intake.
- Third-party crash-reporting SaaS (Sentry, Bugsnag, Rollbar). See
  `docs/CRASH_REPORTING_DESIGN.md`.
- Third-party analytics on the website. The website is a static
  download landing page.

---

## 7. For the maintainer — operational expectations

> Internal-only section.

- **Triage cadence:** check new issues + security advisories at
  least once per business day. Set `triage` label, prioritise per
  §3.
- **Escalation:** SEV-0 follows `docs/RUNBOOK.md §1` / `§2` / `§6`.
- **Public communication:** all incident communication originates
  on GitHub Security Advisory or Release notes; social-media posts
  link back, never duplicate.
- **Response templates:** keep stock first-response replies for
  common categories (out-of-MVP feature, missing Ollama, export
  fails because Pandoc not installed). They live as canned
  responses in the founder's editor or a private gist.

---

*Last updated 2026-05-08. This document is iterated as the support
load grows; see `MILESTONES.md L7`.*
