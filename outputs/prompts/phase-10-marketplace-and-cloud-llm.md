# Phase 10 — Marketplace + Cloud LLM (V1.5 begins)

## Goal

Stand up the plugin marketplace (web frontend + backend + signing infra + Stripe Connect payouts) and ship cloud LLM provider support with both BYO-key and Studio-credits modes.

## Pre-conditions

V1.0 GA shipped. Production traffic flowing on the auto-update path.

## Inputs

1. `../_deep/07-plugin-architecture.md` — sections 9, 10 (marketplace).
2. `../_deep/08-ai-integration.md` — sections 7 (cloud providers).
3. `../_deep/01-BRD-business-requirements.md` — section 7 (pricing).
4. `../_deep/06-security-privacy-compliance.md` — section 11 (data retention).

## Deliverables

### 1. Marketplace backend

A new server-side service (out of the desktop repo; new repo `booksforge-marketplace`). Stack: Rust `axum` + Postgres. Endpoints: plugin submit, signing, list, search, install token, ratings, payments. Stripe Connect for publisher payouts. Manual review queue for first 6 months.

### 2. Marketplace web frontend

`booksforge-marketplace-web` repo, Next.js. Browse, search, install (deep-link `booksforge://install/<plugin-id>`).

### 3. Desktop marketplace UI

In-app marketplace browser. List, search, install via deep-link. Install pulls a signed package, verifies, runs through capability prompt (Phase 07).

### 4. Cloud LLM providers

Implement `LlmProvider::Cloud(Provider)` for: Anthropic, OpenAI, OpenRouter. Mistral and Cohere as stretch. Each behind a feature flag.

### 5. BYO key mode

Settings panel: enter API key per provider. Stored in OS keyring. Calls go directly client → provider with retries and rate-limit handling.

### 6. Studio credits mode

BooksForge backend service `booksforge-llm-proxy` (auth, usage metering, forwarding to provider with our enterprise key). Studio subscription includes a monthly credit pool. Per-call cost estimates shown to user. Hard daily budget cap configurable.

### 7. Cost UI

Pre-call estimate. Per-day spend tracker. Per-project AI spend in audit log.

### 8. Tests

- Marketplace install flow E2E.
- Plugin signing pipeline test (plugin signed by marketplace key verifies in desktop).
- BYO key path test using VCR cassettes for each provider.
- Studio credits path test against a mock proxy.
- Rate-limit handling per provider.
- Network-unreachable fallback to local model with user prompt.

## Guard-rails

**[GUARD-P10-1]** Marketplace plugin signature is verified before install — never bypass.

**[GUARD-P10-2]** Cloud provider DPAs include no-train terms; enforced by using enterprise endpoints. CI grep guard ensures we don't accidentally hit non-enterprise endpoints.

**[GUARD-P10-3]** Cost estimate is shown before every cloud call. Audit row written.

**[GUARD-P10-4]** Studio proxy never logs prompt content; only usage metadata. Penetration-tested before launch.

**[GUARD-P10-5]** Plugin marketplace search results are ranked by quality not by sponsorship. No paid placement in V1.5.

## Acceptance criteria

1. Five marketplace plugins listed and installable from desktop.
2. First paying plugin sale processed end-to-end (Stripe).
3. Anthropic and OpenAI providers verified end-to-end (BYO key + Studio credits).
4. Cost estimates within 15% of actual on test cassettes.
5. Network-unreachable test falls back gracefully to local model.

## Out of scope

- Sync (Phase 11).
- Collaboration (Phase 12).
- Plugin write capabilities for importer/exporter (Phase 13).

## When you finish

PR title `Phase 10: Marketplace + Cloud LLM`.
