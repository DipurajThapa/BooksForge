# BooksForge — Business Model (Decision Pending)

> **Status: founder decision required.**  This document scaffolds
> the pricing / monetisation decision audit #49 calls for.  It is
> *not* a recommendation — it presents the realistic options + their
> tradeoffs so the founder can choose with eyes open.
>
> **Refs:** `EXTERNAL_AUDIT_BACKLOG.md #49`,
> `outputs/MVP_SCOPE.md §3` (multi-author / multi-machine licensing
> is out-of-MVP), `EULA.md §2` (per-user / per-machine grant),
> `docs/DISTRIBUTION.md`.

---

## 1. Constraints the model has to honour

| Constraint | Source | Implication |
|---|---|---|
| Local-first, manuscript never leaves the device | `outputs/SECURITY_PRIVACY.md` | Any licence-key flow must be **offline-verifiable**.  No "phone home for activation" loop. |
| Single-machine licence in MVP | `outputs/MVP_SCOPE.md §3` | Multi-machine and team / studio licensing are V1+; the MVP price-point should not promise team features. |
| GPL sidecars (Pandoc) invoked as separate processes | `THIRD_PARTY_LICENSES.md` | We can charge for BooksForge regardless of Pandoc's licence — the licence applies to BooksForge itself. |
| Privacy-first positioning | every doc | Pricing copy must not undermine the privacy story (e.g. no "track usage to improve experience" language). |
| Solo founder + small team | reality | Pricing model must be operable without a billing engineer + support team. |

---

## 2. Realistic options

### Option A — One-time purchase

| | |
|---|---|
| **Price (proposed)** | $79–$129 USD, one-time, single-machine perpetual |
| **What you get** | Current major version (e.g. v1.x); free patch + minor updates within the major. |
| **Major upgrades** | Discounted upgrade pricing (e.g. 50% off) for next major. |
| **Licence-key flow** | Ed25519-signed token issued by the founder's payment processor (Lemon Squeezy / Paddle / Gumroad).  Token is offline-verifiable using a public key bundled in the app.  No phone-home. |
| **Pros** | Simple.  Aligns with the privacy-first ethos (you bought it; you own it).  Predictable revenue per sale. |
| **Cons** | No recurring revenue.  Major-version cadence has to actually produce upgrade-worthy releases.  Hard to monetise heavy-AI users (their cost-of-goods is the same as light users — Ollama runs on their hardware). |
| **Comparable products** | Sublime Text, Tinderbox, Scrivener (Mac/Win $59). |

### Option B — Subscription

| | |
|---|---|
| **Price (proposed)** | $8–$14 USD/month, single-user, single-machine |
| **What you get** | All updates + cloud-side optional features (originality service, future cloud-LLM bridge if the team adds one — both opt-in). |
| **Licence-key flow** | Subscription token rotates every N days; offline grace period (e.g. 30 days unreachable → app keeps working but starts nagging). |
| **Pros** | Predictable recurring revenue.  Funds ongoing maintenance.  Lets you ship features continuously. |
| **Cons** | Privacy posture conflict — recurring billing inherently requires the user to talk to a payment processor.  Some target users (writers) will resist subscription pricing.  Operationally heavier. |
| **Comparable products** | Ulysses ($5.99/mo), iA Writer (one-time though), Sudowrite ($10/mo). |

### Option C — Hybrid (recommended for evaluation)

| | |
|---|---|
| **Free tier** | The full local app — every MVP feature, all 10 agents, all exports.  No artificial limits.  The privacy-first writing tool is fully functional offline forever, with no payment. |
| **Paid tier** | A "Pro" or "Studio" SKU at $X/year that unlocks: optional cloud-LLM bridge (when the team adds it), opt-in remote originality service (Copyleaks / PlagScan integrations), future team-collaboration features, priority email support.  None of these are MVP. |
| **Licence-key flow** | Pro key is a signed token unlocking the in-app feature flag.  Free tier never checks. |
| **Pros** | Aligns marketing ("the only privacy-first writing app — and it's free for what most writers need").  Subscription only triggers when the user opts into cloud features.  Drives adoption. |
| **Cons** | Revenue dependent on selling cloud-bound features whose value is uncertain pre-launch.  Risk of the free tier being too good to convert. |
| **Comparable products** | Obsidian (free + paid Sync), Joplin (free), Typora ($14.99 one-time). |

### Option D — Open-source (paid) commercial licence

| | |
|---|---|
| **Code licence** | Dual — MIT / Apache-2.0 for crates + binaries are *NOT* free for commercial use without a paid commercial licence. |
| **Free tier** | Source available; build-it-yourself works.  Pre-built signed binaries require purchase. |
| **Pros** | Strong community angle.  Aligns with privacy-first ethos.  Crates can be reused. |
| **Cons** | Hard to enforce "no commercial use" without legal action.  Sketchy business model — most writers want a binary, not a source tree. |

---

## 3. Recommended next steps

1. **Pick Option A, B, C, or D** — or describe a fifth.  The decision should be documented in this file with a date + rationale paragraph.
2. **Pick a payment processor.**  Lemon Squeezy and Paddle handle EU VAT automatically; Gumroad is simpler but doesn't.  All three support one-time purchases + subscriptions + signed-licence-key issuance.
3. **Once chosen, design the licence-key flow:**
   - Generate a long-lived Ed25519 keypair.  Public key embedded in the app at compile time.  Private key with the payment processor (or your offline backup).
   - On purchase, the processor calls a webhook that signs the licence payload and returns a `.bfkey` file the user saves into `~/.booksforge/licence.bfkey`.
   - On launch, the app reads the file, verifies the signature, and unlocks the appropriate tier.  No network call.
4. **Update `EULA.md` Section 2** to reflect the chosen scope (single-machine vs subscription terms).
5. **Update `PRIVACY_POLICY.md` §1.2** if the chosen model adds any new outbound network call (e.g. subscription rotation check).
6. **Wire the licence flow** in `crates/booksforge-domain/src/licence.rs` (NEW crate module — proposed) + a `LicencePanel` UI.  Implementation belongs to a post-MVP milestone.

---

## 4. What is decided (fill in)

> **Decision date:** *(YYYY-MM-DD when made)*
>
> **Selected option:** *(A / B / C / D / other)*
>
> **Rationale:** *(2–3 sentences explaining why this option fits the
> project's privacy posture, cadence ambition, and operational
> capacity.)*
>
> **Payment processor:** *(Lemon Squeezy / Paddle / Gumroad / other)*
>
> **Public-tier price:** *(USD)*
>
> **Pro-tier price (if hybrid):** *(USD/yr)*
>
> **EULA changes required:** *(yes / no — if yes, list the sections)*
>
> **Privacy Policy changes required:** *(yes / no)*

Until this section is filled, the launch checklist
(`docs/PRE_LAUNCH_CHECKLIST.md`) blocks at the **1.0 GA gate**.  The
MVP / closed-beta gates are **not** blocked by this decision —
those releases ship as free-to-use.

---

*Last updated 2026-05-09 (scaffolded).  Update on every change to
the decided model.*
