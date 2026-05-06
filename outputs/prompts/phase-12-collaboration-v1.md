# Phase 12 — Collaboration v1 (comments + suggestions live)

## Goal

Real-time collaboration scoped to comments and suggestions only. Two users in a project can see each other's comments and suggestions in near-real-time. Manuscript text remains last-writer-wins for V1.5; full-content CRDT is V2.0+.

## Pre-conditions

Phase 11 (sync transport) merged.

## Inputs

1. `../_deep/10-roadmap-and-phasing.md` — Phase 12.
2. `../_deep/01-BRD-business-requirements.md` — V1.5 scope.

## Deliverables

### 1. CRDT for comments and suggestions

Use `automerge-rs` (or equivalent) for the comments and suggestions documents only. Each comment/suggestion is a node in the CRDT; manuscript content is **not** in the CRDT in V1.5.

### 2. Collaboration transport

Reuse the sync server (Phase 11) with a websocket layer for low-latency CRDT updates. Encrypted at the application layer.

### 3. Presence

Awareness protocol: who's connected, where their cursor is (in coarse "scene" granularity, not character-precise — privacy and bandwidth).

### 4. Identity

BooksForge account identity (Phase 10). Display name + avatar. Per-project permissions: owner, editor, beta-reader (read-only + comment).

### 5. Tests

- Two-client comment sync within ≤ 1 s on the same scene.
- CRDT conflict resolution on simultaneous comment threads.
- Permission enforcement: beta-reader cannot edit manuscript.
- Disconnection / reconnection: comments queued offline reconcile on reconnect.

## Guard-rails

**[GUARD-P12-1]** Manuscript text edits remain locally authoritative; collaboration is comments + suggestions only.

**[GUARD-P12-2]** All collaboration traffic is encrypted client-side.

**[GUARD-P12-3]** Permissions enforced server-side; client-side checks are belt-and-braces.

**[GUARD-P12-4]** Awareness data minimal — no keystroke leakage to other users.

## Acceptance criteria

1. Two real users co-comment on a project; updates are visible in ≤ 1 s.
2. Beta-reader role can read and comment; cannot edit text.
3. Offline → online reconcile preserves comment order.

## Out of scope

- Real-time text co-editing (V2.0).
- Voice/video chat.

## When you finish

PR title `Phase 12: Collaboration v1`. After merge cut **V1.5 release**.
