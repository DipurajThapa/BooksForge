# Business Requirements Document — BooksForge

**Version:** 1.0.0-draft  •  **Status:** For review  •  **Date:** 2026-05-06

---

## 1. Executive summary

BooksForge is a **local-first, AI-assisted, cross-platform book authoring and publishing platform** that takes a writer from blank page to store-ready DOCX, PDF, and EPUB without forcing their manuscript onto a vendor cloud. It targets three audiences with one platform — indie self-publishers, professional/trade authors working with editors, and academic/technical writers — through a shared core and mode-specific templates, validators, and AI prompt sets.

The platform differentiates on four axes the incumbents do not combine: **(1)** privacy-grade local-first storage with optional opt-in cloud, **(2)** offline-capable AI via embedded llama.cpp / external Ollama with cloud LLM as a premium opt-in, **(3)** a Pandoc-grade export pipeline configured for KDP, IngramSpark, Apple Books, and academic press requirements out of the box, and **(4)** a plugin/extension architecture so genre communities, editors, and small presses can ship their own templates, validators, and AI prompt packs.

## 2. Problem statement

Authors today juggle a fragmented toolchain: Scrivener or Word for drafting, Vellum or Atticus for formatting, Sudowrite or NovelCrafter for AI assistance, Grammarly or ProWritingAid for editing, Plottr for outlining, Calibre for EPUB QA. Each tool is good at one thing and mediocre at the others. Worse, the AI-forward tools are SaaS-only — manuscripts must be uploaded, raising IP, privacy, and cost concerns, especially for authors with NDAs, journalists, ghostwriters, or anyone writing under contract.

Three concrete pains:

The **fragmentation tax**: authors lose hours per book copy-pasting between tools, re-doing formatting, and reconciling versions. Style consistency degrades. Manuscripts get corrupted on the seam between tools.

The **privacy tax**: a SaaS AI tool wants the manuscript on its servers. For paid ghostwriters, journalists, academics under embargo, or authors whose advance contracts forbid third-party processing, this is a non-starter — yet AI assistance is now table-stakes for productivity.

The **publishing-readiness tax**: getting a manuscript to actually pass KDP, IngramSpark, or an academic press validator the first time is a dark art. Authors waste days on rejected uploads over font embedding, bleed, ISBN placement, or EPUB-3 schema.

BooksForge collapses the toolchain, removes the privacy tax by defaulting to local AI, and removes the publishing-readiness tax through rule-based validators tied to each store's published requirements.

## 3. Vision and product goals

**Vision.** The default authoring environment for any writer who values their manuscript's privacy, owns the words they produce, and wants to ship a publication-ready book without a cloud subscription.

**Product goals (18-month horizon).**

The platform must let a writer go from empty project to a store-validated EPUB in a single afternoon, with AI assistance that runs offline on a 2024-class laptop. It must support fiction, non-fiction, and academic workflows without requiring three different tools. It must be extensible enough that a romance author community could ship a "romance pack" of templates, beta-reader prompts, and trope validators without forking the codebase.

**Non-goals.**

BooksForge is not a publisher — we do not host, sell, or distribute books. It is not a real-time collaborative editor in V1 (collaboration is on the V1.5 roadmap, with a deliberately conservative model). It is not a DRM tool. It is not a marketing automation platform; marketing helpers are scoped to metadata, blurb, and back-cover copy generation only.

## 4. Target users and personas

The product serves three primary personas. Each persona's needs map to a "mode" in the application — Fiction, Non-Fiction/Trade, and Academic. The shared core is the same; templates, validators, and AI prompts differ.

### 4.1 Persona A — Indie Anya

Anya is a 34-year-old self-publishing romance author with eleven titles on KDP. She writes 4,000 words a day, works on a Windows laptop in a café, has a moderate AI suspicion (uses ChatGPT for brainstorming but won't paste her manuscript), and obsesses over series consistency across 100k-word books. Her current toolchain is Scrivener plus Vellum plus a spreadsheet for series bible. She loses two days per book to formatting and another day to KDP rejection back-and-forth. She would pay $99 once or $9/month for a tool that makes her ship faster without sending her words to a server.

What she needs from BooksForge: distraction-free drafting, a series bible that auto-extracts characters and locations from her manuscripts, AI revision suggestions that run on her laptop, one-click KDP and IngramSpark export profiles, and trope/genre-convention validators ("did you set up the Black Moment beat?") drawn from a romance plugin pack.

### 4.2 Persona B — Trade-author Theo

Theo is a 52-year-old non-fiction author with a Big-Five publisher contract. His editor sends Word docs with tracked changes; his agent reads PDFs; his publisher demands a specific manuscript format with title page, copyright page, ToC, endnotes, and a 2-inch left margin. He's contractually forbidden from putting the draft on a third-party cloud. He uses Word and a folder of revision PDFs and is exhausted by it. He owns a MacBook Pro with 32 GB RAM.

What he needs from BooksForge: import/export of tracked-changes DOCX with full fidelity, a manuscript format profile that matches his publisher's spec sheet, snapshot/revision history independent of git, the ability to leave inline notes for his editor that round-trip through Word, and AI assistance that never leaves his machine.

### 4.3 Persona C — Academic Aisha

Aisha is a 41-year-old historian writing a 140k-word university-press monograph with 800 footnotes, 24 figures, an index, and a bibliography in Chicago author-date. She uses Zotero, LaTeX, and Word in an unhappy rotation. Her press demands a specific TeX template for the typeset book and a clean DOCX for the copyedit. She runs Linux.

What she needs from BooksForge: first-class footnotes and endnotes, BibTeX/CSL citation integration with Zotero, equation support (LaTeX math), figure numbering and cross-references, an index generator, export to LaTeX as well as DOCX/PDF, and the discipline of a structured outline.

### 4.4 Secondary personas

**Editor Emma** uses BooksForge to receive manuscripts from authors, run validators, leave inline comments, and round-trip changes. **Beta-reader Ben** uses a read-only viewer mode with a comment overlay. **Plugin-author Priya** ships a Cozy Mystery template pack and earns revenue or reputation. These are V1.5+ priorities but inform the V1.0 architecture.

## 5. Market and competitive analysis

The book-authoring tools market is fragmented across drafting, formatting, AI assistance, and validation. The top tools each occupy one quadrant well:

**Scrivener** dominates serious drafting (corkboard, binder, snapshots) but the formatter ("Compile") is famously brittle, there is no AI, and the UI is dated. Cross-platform but with feature drift between Windows and Mac.

**Vellum** is the gold standard for fiction formatting on Mac — beautiful PDF and EPUB output — but Mac-only, no drafting, no AI, no academic features. Roughly $250 perpetual.

**Atticus** is the cross-platform answer to Vellum, web-based with Electron desktop, formatting plus a basic editor, no AI. Subscription.

**Reedsy Studio** is a free, web-based editor and formatter with a marketplace of paid editors. No offline. No AI assistance for writing.

**Sudowrite, NovelCrafter, Plotdrive** are AI-first writing tools, web-only, SaaS subscriptions, manuscripts on their servers. Excellent generation features, brittle formatting/export, no privacy posture.

**Ulysses, iA Writer** are minimalist writing apps — beautiful, narrow scope, no formatting or AI.

**Plottr** is outlining-only.

**Pandoc + LaTeX + Zotero** is the academic stack — powerful, free, but a multi-week onboarding curve and no GUI.

**BooksForge's white-space:** the only product that combines (a) Scrivener-class drafting, (b) Vellum-class formatting and export, (c) AI assistance comparable to Sudowrite for routine tasks, (d) publishing validators per store, (e) plugin extensibility, **and** (f) local-first privacy. We do not need to beat any of these on its single axis; we need to be the only product that does not force the user to leave for the others.

## 6. Success metrics

Product success is measured against four pillars. Targets are stated for the 12-month-post-V1.0 horizon.

**Adoption.** 25,000 monthly active writers; 8,000 paying customers; net revenue $1.2M ARR; <3% monthly churn on annual plans.

**Productivity.** Median time from "new project" to "first store-validated export" under 45 minutes for a fiction MVP project (measured via opt-in telemetry). Median manuscript writes per week per active user ≥3 sessions of ≥30 minutes.

**Quality.** Crash-free session rate ≥99.5%. KDP/IngramSpark rejection rate on first upload ≤5% for users who run the validator. AI-suggestion acceptance rate ≥30% (measured per accepted vs shown).

**Ecosystem health.** ≥40 third-party plugin packs published in the marketplace; ≥3 community-maintained translations; documented public API with ≥6 months of backwards-compatibility commitment.

## 7. Business model and pricing

**Free tier.** Local writing, local AI (user provides hardware), all export formats, one project at a time, basic templates. The free tier must be genuinely usable — it is the privacy story.

**Pro (one-time license, $129; or $7/month).** Unlimited projects, all genre templates, all validators, plugin marketplace access, snapshot history, premium export profiles, priority support. License includes all V1.x updates.

**Studio (subscription, $19/month).** Everything in Pro plus optional cloud LLM credits (BYO API key supported) for premium editing/research, encrypted cloud sync between devices, real-time collaboration when it ships in V1.5.

**Plugin marketplace** takes a 20% cut on paid plugin sales. Free plugins are free.

**Decision posture.** Both perpetual license and subscription are offered to respect the persona-A indie-author who hates subscriptions and the persona-B/C professional who would expense a subscription. **[DECISION-001]** confirmed; revisit annually.

## 8. Compliance, legal, and IP posture

The platform must be defensible on four legal axes. **GDPR/UK-GDPR/CCPA**: no PII leaves the device by default; opt-in telemetry is granular, anonymous by design, and revocable; cloud sync (Studio tier) operates on a documented data-processing agreement. **AI training**: by default, no user content is used for training, including for the cloud LLM; this is contractually flowed down to LLM vendors via dedicated enterprise endpoints (OpenAI/Anthropic/etc. enterprise terms). **Pandoc licensing**: Pandoc is GPLv2+; BooksForge ships it as a sidecar process invoked over a documented IPC, not as a statically linked library, preserving the freedom to license the host application as we choose. See risk **R-07**. **Plugin liability**: plugins run sandboxed with explicit capabilities; the marketplace requires plugin authors to accept publisher terms; user-installed plugins ("sideload") show a capability prompt similar to mobile app permissions.

## 9. Constraints

The platform must run fully offline for all core flows including AI. It must run on Windows 10+, macOS 12+, and Ubuntu 22.04+ (and Debian-derived distros) with a single codebase via Tauri v2. It must respect a documented hardware floor: 8 GB RAM minimum to run a 3B-parameter quantised model, 16 GB recommended for a 7B, 32 GB for 13B; CPU-only fallback supported but flagged as slow. It must protect manuscripts at rest with optional per-project encryption. It must be code-signed on all three platforms. It must support keyboard-only operation and meet WCAG 2.2 AA.

## 10. Assumptions

We assume Tauri v2 reaches stable status with a multi-window, sidecar-friendly API by the time MVP development begins (currently it does). We assume llama.cpp continues to support GGUF models and provides usable Rust bindings (currently `llama-cpp-rs`/`llm` ecosystem). We assume Pandoc remains under GPLv2+ and continues to support DOCX, EPUB-3, PDF (via LaTeX or wkhtmltopdf or Typst), and LaTeX. We assume the team can secure code-signing certificates for Microsoft, Apple, and one Linux package format (DEB/RPM/AppImage/Flatpak; we will pick at packaging time).

## 11. High-level scope by phase

The full roadmap is in `10-roadmap-and-phasing.md`. At the BRD level:

**MVP (months 0–4).** Single-user, fiction mode, drafting, basic formatting, local AI via embedded llama.cpp, export to DOCX/PDF/EPUB, KDP validator, three genre templates. Windows + macOS only.

**V1.0 (months 5–8).** Add Non-Fiction/Trade and Academic modes. Tracked changes round-trip. Snapshot history. Plugin runtime (read-only capabilities). Citation integration. Linux build. Code signing on all three. Opt-in encrypted local backup.

**V1.5 (months 9–14).** Real-time collaboration (CRDT-based, peer-to-peer with optional relay). Plugin marketplace. Cloud LLM integration (BYO key + Studio-managed credits). Mobile companion app (read-only + comment).

**V2.0 (months 15–18).** Plugin write-capabilities (with stricter sandbox). Voice dictation, voice playback for self-edit. Advanced AI features (multi-document context, character-voice models). Translator pack with terminology preservation. Audiobook export pipeline.

## 12. Stakeholders and roles

| Role | Responsibility |
|------|----------------|
| Product owner | BRD ownership, persona research, roadmap |
| Tech lead | TAD ownership, ADRs, architecture review gate |
| Frontend lead | Tauri shell, React UI, editor, design system |
| Backend/sidecar lead | Rust sidecar, SQLite, IPC, AI runtime, export |
| Security lead | Threat model, plugin sandbox, encryption |
| QA lead | Test strategy, validator suites, accessibility |
| Designer | UX, design tokens, accessibility, illustration |
| Tech writer | In-app help, plugin SDK docs |

For a small team, one engineer can wear two hats; the responsibilities themselves remain.

## 13. Open business questions

These need answers before V1.0 launch and are tracked in the decision log:

Whether to register a marketplace under the company entity or as a separate legal vehicle to compartmentalise plugin liability. Whether the academic mode justifies a separate "BooksForge Academic" SKU and price point. Whether to build official integrations with Zotero/Mendeley or rely on standard CSL/BibTeX import only. Whether the cloud LLM credit pool in Studio is sold as credits or all-you-can-eat with rate limiting.

## 14. Out of scope (explicit non-requirements)

To prevent scope creep during development, the following are explicitly **not** part of any planned phase. Adding any of these requires a BRD revision and roadmap re-baselining: hosted publication or distribution, payment processing for end-readers, DRM application, audio/video editing, AI image generation for cover art (cover image *placement* and template support is in scope; generation is not), social network or community features beyond plugin marketplace listings, in-app marketing automation, real-time voice chat collaboration.
