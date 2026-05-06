# Phase 11 — Encrypted cloud sync (Studio)

## Goal

Implement end-to-end-encrypted project sync between two devices for Studio-tier users. Server stores opaque ciphertext. Conflict detection on multi-device edits with three-way merge UI.

## Pre-conditions

Phase 09 (encryption) and Phase 10 (Studio account, payments) merged.

## Inputs

1. `../_deep/02-FSD-functional-specifications.md` — sections 11.3 (FR-SYNC).
2. `../_deep/06-security-privacy-compliance.md` — section 7.4 (cloud sync posture).

## Deliverables

### 1. Sync server

`booksforge-sync` service. Object-storage-backed (S3-compatible). Endpoints: upload object, list objects, download object, attach metadata, conflict detection. Server is **dumb** — it does not understand project structure; it stores opaque blobs keyed by content hash.

### 2. Client sync engine

Background sync task on the desktop. Watches snapshot objects (already content-addressed). Encrypts (already done if project encryption is on; if not, sync requires enabling project encryption first — make this explicit in UX). Uploads new objects, downloads new ones, reconciles tree references.

### 3. Conflict detection and three-way merge UI

Detect divergence by tree-hash. UI shows side-by-side: current local, current remote, common ancestor (last-synced). User picks, merges, or annotates per node.

### 4. Selective sync

Don't sync large assets / snapshots by default — user opts-in per-project. Saves bandwidth and storage.

### 5. Tests

- Two-device sync: edit on device A, sync, see on device B within ≤ 30 s.
- Conflict scenario: simultaneous edits on two devices, merge UI appears, both versions preserved.
- Server cannot decrypt: attempt server-side decryption with the wrong key fails.
- Data residency: EU users' ciphertext stays in EU bucket.
- Selective sync: large asset opt-out is honoured.

## Guard-rails

**[GUARD-P11-1]** Server never sees plaintext.

**[GUARD-P11-2]** Sync requires project encryption to be enabled.

**[GUARD-P11-3]** Conflict detection is honest — if we can't merge automatically, surface to UI; never silently pick.

**[GUARD-P11-4]** Server-side audit log records access patterns but not content.

## Acceptance criteria

1. Two-device sync of a 100k-word project with 50 assets in ≤ 30 s for incremental changes.
2. Conflict UI works on a contrived divergence.
3. Penetration test confirms server cannot decrypt.
4. EU data-residency test passes.

## Out of scope

- Real-time collaboration (Phase 12).
- Mobile companion apps.

## When you finish

PR title `Phase 11: Encrypted sync`.
