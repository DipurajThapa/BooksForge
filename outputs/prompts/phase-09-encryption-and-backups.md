# Phase 09 — Encryption and advanced backups

## Goal

Ship per-project encryption (Argon2id KDF, AES-256-GCM, SQLCipher), enable pre-AI and pre-migration auto-snapshots, implement snapshot retention policy with content-addressed dedupe, and integrate OS keyring for optional passphrase storage.

## Pre-conditions

Phases 01–08 merged.

## Inputs

1. `../_deep/06-security-privacy-compliance.md` — sections 4.1, 4.2.
2. `../_deep/04-data-model-and-project-format.md` — section 10 (encryption), section 7 (snapshot storage).
3. `../_deep/02-FSD-functional-specifications.md` — FR-PROJ-012, FR-SNAP-001…006.

## Deliverables

### 1. SQLCipher integration

Replace `rusqlite`/`sqlx` SQLite plain with SQLCipher when project encryption is enabled. Master key derived via Argon2id from passphrase. Salt and KDF params in `manifest.toml`.

### 2. Asset and snapshot encryption

When project encryption is enabled, every asset blob and snapshot object is encrypted with AES-256-GCM, master key + random nonce. Nonce stored alongside ciphertext. Authentication tag verified on read.

### 3. Passphrase UX

Enable encryption: dialog warns about passphrase loss = data loss; offers OS-keyring storage (DPAPI / Keychain / Secret Service). Passphrase strength meter (zxcvbn). Project-open: passphrase prompt; OS-keyring auto-fill if available; "Forgot passphrase" → "data is unrecoverable" message (we do not back-door).

### 4. Auto-snapshot policies

Per FR-SNAP-002: hourly during active sessions. Pre-AI-edit (FR-SNAP-003): wired in Phase 03 — verify here. Pre-migration: wired in Phase 01 hook — fully implemented now. Pre-validator-fix (Phase 05) — wired here. Retention policy (FR-SNAP-005): keep all manual; last 30 auto; monthly archives; rest GC'd.

### 5. Storage compaction

Periodic GC of orphaned content-addressed objects (assets and snapshot objects) under user control.

### 6. "Save Self-Contained Copy"

Bundle ZIP with all referenced assets included; portable across devices.

### 7. Tests

- Encryption round-trip: encrypt project, close, reopen with passphrase, content matches.
- Wrong passphrase: refuse to open; never partially decrypt.
- OS-keyring path: store, retrieve, delete keychain entry.
- Snapshot dedupe: change one scene; new snapshot reuses unchanged objects (storage growth ≈ size of changed scene only).
- Compaction: orphaned object removed; referenced object preserved.
- Self-contained ZIP exports and re-imports cleanly on a different machine.

## Guard-rails

**[GUARD-P9-1]** Encryption is **opt-in per project**, never global default — making the privacy story honest and recoverable.

**[GUARD-P9-2]** No back-door key escrow. We cannot recover a forgotten passphrase. UI states this clearly.

**[GUARD-P9-3]** Master key never persisted to disk in plaintext. OS-keyring is the only acceptable persistent store; otherwise re-enter on open.

**[GUARD-P9-4]** Auth tag verification on every encrypted read; tampered ciphertext → typed error → no partial decryption surfaces.

**[GUARD-P9-5]** Pre-AI snapshot must already exist (Phase 03); this phase verifies + adds pre-validator-fix and pre-migration snapshots.

## Acceptance criteria

1. Encrypt a 100k-word project; reopen with passphrase; full content present.
2. Forgotten-passphrase test: app refuses, no partial reads.
3. Wrong passphrase: error after 3 attempts, no rate-limit-bypass.
4. Keychain integration on each OS verified.
5. Pre-migration snapshot taken automatically when a future schema change runs.
6. Snapshot storage growth on a 10-edit session is ≤ 2× the changed-scene size.

## Review gate

- Cryptographic primitives use vetted crates (`aes-gcm`, `argon2`, `rand_core`); no rolled-our-own crypto.
- Constant-time tag comparison.
- Salt is random and persisted; KDF params support upgrade in V1.x without re-encryption hell.

## Out of scope

- Cloud sync (Phase 11) — depends on this.
- Sharing of encrypted projects between users (requires asymmetric crypto; later).

## When you finish

PR title `Phase 09: Encryption and snapshots`. After merge, **V1.0 release branch** opens for stabilisation.
