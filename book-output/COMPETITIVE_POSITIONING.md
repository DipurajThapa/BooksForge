# BooksForge — Competitive Positioning & Feature Recommendations

**Date:** 2026-05-10
**Companion to:** `ARCHITECTURE_RECOMMENDATIONS.md`
**Scope:** what the market does that BooksForge doesn't, what BooksForge does that the market doesn't, and the one-line positioning that follows from both

---

## 1. The 2026 landscape (six tools, one-line description, gap to BooksForge)

| Tool | Model strategy | Privacy | Pricing | Distinctive feature | Direct competitor? |
|---|---|---|---|---|---|
| **Sudowrite** | Closed; cloud only | Cloud (uses your text) | $19–29/mo, credit metered | Story Bible 2 + Canvas + Outline Unlimited + Visualize + Brainstorm | No — different audience (hobbyist craft) |
| **NovelCrafter** | BYOK (Claude / GPT-4o / Gemini / Llama / local) | Cloud, but BYOK lets you point at local | Subscription (~$20/mo) | Codex with auto-reference detection in prose | Adjacent — same structural philosophy |
| **NovelMage** | BYOK + local via Ollama / LM Studio | **Local-first** (same as us) | **Lifetime license** | Codex + offline | **Yes — head-on competitor** |
| **NovelAI** | Own model (Kayra) | Cloud | Subscription | Lorebook + minimal content filters (horror/dark) | Niche overlap |
| **Squibler** | Cloud LLM | Cloud | Subscription | Template-driven walkthrough; corkboard scene management | Adjacent (better first-mile) |
| **Plottr** | Outline only, no prose | Cloud | Subscription | Visual timeline + plot board (no draft generation) | Complementary, not competing |

**The one tool BooksForge actually competes with for the same buyer is NovelMage.** Same local-first thesis, same Ollama backend, same privacy promise. Everyone else is solving an adjacent problem (cloud convenience, niche content, outlining).

---

## 2. What BooksForge already does better than every competitor

These are not hypothetical strengths. They exist in the codebase today. The competitive analysis confirms none of the six tools above ship them.

| Capability | BooksForge | Sudowrite | NovelCrafter | NovelMage | NovelAI |
|---|---|---|---|---|---|
| Bounded swarm of agents (≤8 calls, audit ledger) | ✅ | ❌ | ❌ | ❌ | ❌ |
| Pre-edit snapshot before every accepted change | ✅ | partial | ❌ | ❌ | ❌ |
| Differentiated polish stack (voice / metaphor / dialogue / tension) | ✅ | one-shot rewrite | one-shot rewrite | ❌ | ❌ |
| Anti-AI-tells detector (PUBLISHABLE / NEEDS_REVISION / AI_SMELL_HIGH) | ✅ | ❌ | ❌ | ❌ | ❌ |
| Validator pipeline (Schema / Redaction / Length / EntitySanity / Originality) | ✅ | ❌ | ❌ | ❌ | ❌ |
| EPUBCheck-validated export end-to-end | ✅ | hand-off to Atticus/Vellum | hand-off | hand-off | hand-off |
| Cancel a long-running job mid-flight (job_id + cancel) | ✅ | ❌ | ❌ | ❌ | ❌ |
| Reproducible deterministic prompt input hashing | ✅ | ❌ | ❌ | ❌ | ❌ |

The BooksForge moat, when articulated, is: **"the only book platform engineered like an editorial production line."** Every other tool is a single LLM call wrapped in a smarter prompt. That sounds technical but it cashes out as: writers who care about *finishing* a publishable book — not just generating fun chapters — are underserved by every other tool. That's the buyer.

---

## 3. Five features to copy (ranked by ROI for BooksForge's buyer)

These are confirmed market-validated. Not speculative. Each one is shipping in at least two competitors and is the source of their best reviews.

### 3.1 Codex-style auto-reference detection (NovelCrafter / Sudowrite Story Bible 2 / NovelAI Lorebook)

**What it is.** When the writer types "Elara walked into the keyhole," the editor recognises "Elara" as a Codex entity, auto-injects her character bible into the next AI call, and offers a tooltip showing her current state.

**Why it matters for BooksForge.** Today the orchestrator loads bibles by full pass-through. Auto-reference detection lets the polish stack and any future inline-rewrite use *only* the relevant bible chunks — cuts prompt size by 60–80%, cuts latency proportionally, and improves consistency by surfacing entity drift to the writer in real time.

**Effort.** Tractable. Tokenise prose → fuzzy-match against entity names in the project's bibles → inject matched bibles into context. ~1 sprint. Wire into the existing TipTap editor as a decoration plugin.

### 3.2 Inline rewrite with custom instruction (Sudowrite's killer feature)

**What it is.** Select 1–3 paragraphs in the editor. Right-click → "Rewrite with instruction…". Type "make this more tense and cut the dialogue tags." AI returns a candidate; accept / reject / re-spin.

**Why it matters for BooksForge.** This is the single feature most consistently called out as the reason writers pay for Sudowrite. BooksForge has all the parts: polish stack, snapshot, audit ledger. It does not have the in-editor surface to invoke them on a selection with a freeform instruction. The architectural cost is one new IPC command, one new agent (`inline-rewrite`) that wraps the existing polish stack with a user-supplied instruction string, and one editor menu.

**Effort.** Small. ~1 week. Highest perceived-value-per-line-of-code on this list.

### 3.3 BYOK multi-model routing (NovelCrafter / NovelMage)

**What it is.** Settings: per-agent model selector. "Use Claude Opus for `scene-drafter-fic`, qwen3.5:9b for `scene-critic`, GPT-4o for `polish:dialogue`." User pays the cloud bill if they want cloud; defaults stay local.

**Why it matters for BooksForge.** Two reasons. (1) The architecture already supports it — `model_preference` is per-agent. We just don't expose a non-Ollama provider. (2) It removes the largest objection from prospective buyers: "but local models aren't as good as Claude for prose." Let them have both. Privacy stays the default; quality is one click away.

**Effort.** Medium. New `LlmProvider` trait abstracting Ollama / Anthropic / OpenAI. Provider-key storage in OS keychain. UI for per-agent model selection. ~2 weeks.

### 3.4 Genre-pack templates (Squibler's structural play)

**What it is.** Curated, pre-authored bibles + voice profiles + scene-card templates for the eight genres that account for ~80% of indie publishing: cozy mystery, romantasy, regency romance, dark academia, military SF, urban fantasy, thriller, literary fiction. New project → "Start from genre pack" → bibles are 70% complete on day one.

**Why it matters for BooksForge.** The Run #5 → Run #11 finding in `quality-review.md` is exactly this: a generic bible produces less interesting prose than a well-crafted one. Genre packs are pre-crafted bibles. They also dramatically improve first-mile onboarding — Sudowrite and Squibler both win new users with pre-built starting points; BooksForge starts you with a blank canvas, which is the worst possible UX for a writer who doesn't know what a "scene card" is.

**Effort.** Mostly content authoring, not engineering. Schema is shipped; need 8 well-written packs. ~3 weeks of ghostwriter time + 3 days of engineering to wire the import path.

### 3.5 Brainstorm mode (Sudowrite's exploratory ideation)

**What it is.** Non-committal generation. "Give me 20 names for a coastal Maine lighthouse town." "Suggest 10 ways the inheritance subplot could complicate." "What if Arthur isn't dead?" Returns short, scannable lists. Doesn't write to the manuscript.

**Why it matters for BooksForge.** The bounded-agent architecture is great for *production* but terrible for *play*. Writers spend ~30% of their time exploring, and BooksForge currently has zero surface for it. Brainstorm sits outside the bounded-workflow constraint by design — it's the one place where the audit ledger is silent and the user can ask the model anything.

**Effort.** Small. New agent `brainstorm` with `UserGate::None` and `WhenToRun::OnDemand`. ~3 days.

---

## 4. The differentiation play — what to lean into, what to ignore

**Lean into:**

- **Reliability.** Audit ledger, snapshot before every change, cancel mid-run, deterministic input hashing. Nobody else has this. Make it visible: a "production trail" panel showing every agent call, every model, every accept/reject. Pitch it as the *first AI book tool a professional ghostwriter can defend in a re-print contract dispute.*
- **Output quality engineering.** Polish stack + anti-tells detector + (after the architecture recs ship) numeric voice profile + rhythm polish + adaptive planner. This is the technical edge that the next 11 runs will compound. Sudowrite is one big LLM; BooksForge is a production line. Lean into the line.
- **Publication grade.** EPUBCheck validation, DOCX/PDF/EPUB-3 in one app. Sudowrite et al. punt to Atticus/Vellum. BooksForge can be the only tool a writer needs from idea to .epub upload.

**Ignore (do not chase):**

- **Image generation.** Sudowrite has Visualize, Squibler has AI Visualize. They're decoration. Writers don't pay for them. Skip until V2.0.
- **Community / shared prompts marketplace.** Real, but requires a network effect we won't have for 18 months. Don't build it now.
- **Storyteller-mode adventure / roleplay (NovelAI).** Different buyer. Don't dilute the brand.
- **Visual story canvas (Sudowrite Canvas, Plottr).** Tempting because it demos well. But it's a UI toy that doesn't improve the manuscript. The Run #11 monotony wasn't caused by lack of a canvas; it was caused by underconstrained voice. Spend the engineering hours on voice, not canvas.

---

## 5. Pricing recommendation

**Lifetime license, single tier, ~$199 one-time.**

Reasoning:

- NovelMage already proved the lifetime-license model works for the privacy-first audience. They charge ~$179.
- BooksForge has *no marginal cost per generated word* (local Ollama). Subscription pricing is dishonest when the vendor doesn't bear ongoing inference cost.
- Subscription churn is the dominant problem at Sudowrite/NovelCrafter — writers who pay $25/mo while *not* writing for two months cancel.
- A $199 one-time payment converts the buyer once and keeps them paying attention to BooksForge updates rather than monitoring monthly value.

**Optional add-on: "Studio Sync" — $4/mo for sync-between-machines + cloud backup of project bundles + early access to new agents.** Pure recurring revenue from power users without compromising the one-time-license thesis for everyone else.

---

## 6. The one-line positioning

> **BooksForge is the only local-first AI book platform engineered like a publishing pipeline. Your manuscript stays on your machine. Every change is versioned, every AI call is auditable, and the export is publication-grade.**

That sentence does the work that ten landing-page paragraphs cannot. It names exactly four things (local-first, pipeline, audit, publication-grade), each of which is a feature competitors cannot easily replicate without rewriting their stack:

- **Local-first** — Sudowrite, NovelCrafter, NovelAI, Squibler are SaaS by architecture. They cannot offer this without a year of replatforming.
- **Pipeline** — every competitor is one LLM call. Becoming a pipeline requires an orchestrator they don't have.
- **Audit** — requires a ledger schema and write-discipline that nobody else has built.
- **Publication-grade export** — Sudowrite punts to Atticus. NovelCrafter exports DOCX but not EPUBCheck-validated EPUB-3. We already have it.

Two of those four are durable moats (local-first architecture, audit-grade orchestration). Two are catch-up features for everyone else (pipeline, publication export). That's defensible.

---

## 7. Sequencing — the 90-day plan that combines this doc with `ARCHITECTURE_RECOMMENDATIONS.md`

**Days 1–14 (foundation):**
- Numeric VoiceProfile crate (architecture-rec bet 1)
- Schema-aware JSON repair (architecture-rec bet 4)
- Five anti-tell regex guards (architecture-rec §3)

**Days 15–35 (visible product wins):**
- Inline rewrite with custom instruction (this doc §3.2)
- Codex-style auto-reference detection (this doc §3.1)
- Brainstorm mode (this doc §3.5)

**Days 36–60 (the structural moves):**
- Streaming + parallel pipeline (architecture-rec bet 2)
- Adaptive planner (architecture-rec bet 5)
- BYOK multi-model routing (this doc §3.3)

**Days 61–90 (compounding and reach):**
- Self-learning exemplar memory (architecture-rec bet 3)
- Eight genre packs (this doc §3.4)
- Lifetime-license storefront

In this sequence, by day 35 BooksForge ships features that match Sudowrite's headline UX (inline rewrite, codex detection, brainstorm) on top of an architecture nobody else has. By day 60 the speed problem is solved and the planner is making the polish stack adaptive. By day 90 the moat is structural: every successful run makes the next one better, and there are eight genre packs ready out of the box.

The 11-run quality campaign was the proof that the technical foundation is real. The next 90 days turn it into a product writers will pay $199 to own.
