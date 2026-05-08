# BooksForge — Privacy Policy

**Effective date:** *(to be set on first public release)*
**Status:** **Draft — pending legal review.** Replace this notice with
an effective date before any public download is offered.

> *Refs:* `outputs/SECURITY_PRIVACY.md` (technical reference),
> `SECURITY.md` (vulnerability reporting),
> `EXTERNAL_AUDIT_BACKLOG.md #46` (this file's intent).

---

## Plain-English summary

**BooksForge runs entirely on your computer. Your manuscript never
leaves your device unless you explicitly send it somewhere.** We do
not have a server that receives, stores, or processes your writing.
The "AI" features run via Ollama, a local large-language-model
runtime that we communicate with over your machine's loopback network
(`127.0.0.1`) — never to the internet.

This policy explains exactly what data the app handles, what stays
local, what (if anything) ever crosses the network, and what control
you have over each behaviour.

---

## 1. What information BooksForge handles

### 1.1 Information that **stays on your device**

- **Manuscript content** — every scene, chapter, outline, note,
  comment, tracked change, and snapshot. Stored inside your project
  bundle (`*.booksforge/`) on the disk path you choose.
- **Project metadata** — title, author name, genre, target word
  count, format profile, template selection, model preferences. In
  `manifest.toml` and `project.db` (SQLite) inside the bundle.
- **Memory and vocabulary entries** — entities, character profiles,
  places, rules, vocabulary preferences populated by you and/or the
  AI agents. In the project bundle.
- **AI agent runs** — prompt envelopes, model outputs, applied edits,
  audit ledger. In the project bundle's `agent_runs/` directory and
  the `ai_calls`/`agent_runs`/`agent_outputs`/`agent_applied_edits`
  tables in `project.db`.
- **Diagnostic logs** — rotating `tracing` logs at
  `~/.booksforge/logs/` (max 5 MB × 5 files). PII-redacted by default
  (manuscript text and file paths inside the bundle are stripped
  before anything is written to disk).
- **Recent-projects list and app settings** — at
  `~/.booksforge/settings.toml`.
- **Crash reports** — only if you opt in (Settings → Diagnostics →
  Send crash reports). Off by default. Even when on, manuscript text
  and project IDs are redacted before any report is generated.

None of the above leaves your device unless you take an explicit action.

### 1.2 Information that may cross the network — and only on opt-in

- **Update check** *(opt-out toggle in Settings)* — on launch the app
  asks the BooksForge update endpoint whether a new version is
  available. The request contains:
  - your current app version,
  - your operating system family (macOS / Windows) and CPU
    architecture (x86_64 / arm64).

  It does **not** contain manuscript content, project IDs, your name,
  or your IP address beyond what is implicit in any HTTPS request.
  Disable in *Settings → Updates*.
- **Ollama model installation and pulling** *(initiated by you)* —
  when you click *Install Ollama* or *Pull model* in the Setup
  Wizard, the app downloads the requested binary or model from
  Ollama's official source. The request contains the model name; it
  does **not** contain manuscript content.
- **Online plagiarism / originality check** *(off by default,
  consent-gated, V1+ feature)* — if you enable a remote originality
  service, BooksForge sends a hash-based fingerprint of the passage
  you choose to check, never the raw text, to the service you
  configured. Disable in *Settings → Originality*. This feature is
  not enabled in the MVP.

That is the complete list of outbound network behaviour. There is no
analytics, no telemetry, no error reporting service, no usage
tracking, no advertising integration.

### 1.3 Local LLM (Ollama) traffic

BooksForge talks to Ollama over `http://127.0.0.1:11434`. This is
your machine's loopback interface; the traffic does not leave the
device. If you change the Ollama host to a non-loopback address (e.g.
a different machine on your local network), the app shows a blocking
consent dialog explaining that prompts and excerpts will then leave
your device, and you must explicitly accept before the change saves.

---

## 2. AI features and consent

AI capabilities are **off per project until you turn them on**. The
first time you trigger an AI-driven action (Outline Architect,
Copyedit, Continuity check, etc.) on a project, BooksForge prompts
you with a one-time consent dialog explaining:

1. that all model inference happens locally via Ollama,
2. that BooksForge will write the prompt envelope, the model's
   reply, and any applied edits to the project bundle (so you can
   review the audit ledger later), and
3. that you can revoke consent at any time in *Settings → AI*.

If consent is revoked or never granted, no AI feature can run on
that project — the runner returns a `ConsentRequired` error before
any prompt is constructed.

---

## 3. What we (the maintainers) receive

**Nothing automatic.** BooksForge does not phone home with usage
statistics, error reports, project metadata, or any other data
unless you take an explicit action.

If you choose to send us a bug report, security disclosure, or
crash dump, we receive only what you choose to attach. The diagnostic
bundle command (*Settings → Diagnostics → Save diagnostic bundle*)
generates a `.zip` you can review before sharing — it does **not**
auto-upload.

If you contact us at `support@booksforge.app` or
`security@booksforge.app`, we keep your message and any attachments
for as long as needed to resolve the issue and then delete them.

---

## 4. Third-party services

BooksForge bundles or invokes the following external software, each
with its own privacy posture:

| Component | What it is | Network behaviour |
|---|---|---|
| Ollama | Local LLM runtime, separate process | Loopback only by default. Pulls models from Ollama's official source on your explicit action. |
| Pandoc | DOCX/PDF export sidecar, separate process | None. Runs entirely offline. |
| EPUBCheck | EPUB validator sidecar, separate process | None. Runs entirely offline. |

The full attribution list lives in `THIRD_PARTY_LICENSES.md`.

---

## 5. Children's privacy

BooksForge is not directed at children under 13. We do not knowingly
collect personal information from children.

---

## 6. Your rights

Because BooksForge stores your data on your device, the conventional
GDPR/CCPA "right to access" / "right to delete" requests apply to
**you**, not to us — your data is in your project bundles, your
filesystem, and your Ollama model files. To delete it, delete those
files.

If you have contacted us via support or security channels, you may
ask us to delete that correspondence by writing to
`privacy@booksforge.app` *(provisioning pending)*.

---

## 7. Changes to this policy

We will update this document and bump the *Effective date* when
behaviour changes. Material changes (e.g. adding a new outbound call,
even an opt-in one) will be announced in the app's release notes
before they take effect.

---

## 8. Contact

- General privacy questions: `privacy@booksforge.app` *(pending)*
- Security / vulnerability reports: see `SECURITY.md` (preferred path
  is GitHub Security Advisories).

---

## 9. Implementation references (for auditors)

The behaviour described in this document is implemented in the
following components, each gated by automated CI tests:

| Behaviour | Implementation | CI test |
|---|---|---|
| No telemetry SDKs in workspace | `booksforge/deny.toml` `[bans]` | `cargo deny check bans` |
| Ollama default = loopback | `crates/booksforge-domain/src/settings.rs` | `tests/privacy_invariants.rs::ollama_default_endpoint_is_loopback` |
| AI off-by-default per project | `crates/booksforge-orchestrator/src/originality_provider.rs` | `tests/privacy_invariants.rs::default_originality_provider_is_local_only` |
| GPL crates not statically linked | `booksforge/deny.toml` `[licenses]` | `cargo deny check licenses` |
| No outbound network at startup | (gap — tracked as audit #7) | (planned) |
| No manuscript content over the wire | (gap — tracked as audit #8) | (planned) |
| Non-loopback Ollama requires consent | (UI gap — tracked as audit #10) | (planned) |

The "(planned)" rows are tracked in `EXTERNAL_AUDIT_BACKLOG.md`. They
must close before this policy goes into effect on a public release.

---

*This file is a draft. Before any public download is offered, it must
be reviewed by legal counsel, the contact emails must be provisioned,
and the planned CI tests above must be implemented and green.*
