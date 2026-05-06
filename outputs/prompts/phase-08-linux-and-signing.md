# Phase 08 — Linux build and code signing

## Goal

Add the Linux build to the matrix, ship code-signed installers on all three OSes, and verify that auto-update only applies signed packages. After this phase the V1.0 release is operationally ready.

## Pre-conditions

Phase 07 merged. Signing certificates procured (Microsoft EV, Apple Developer ID, Linux GPG).

## Inputs

1. `../_deep/03-TAD-technical-architecture.md` — section 17 (build/packaging/signing).
2. `../_deep/06-security-privacy-compliance.md` — sections 4.4, 4.5.
3. `../_deep/10-roadmap-and-phasing.md` — Phase 8 entries.

## Deliverables

### 1. Linux Tauri build

Ubuntu 22.04 baseline target. Dependencies installed in CI: `webkit2gtk-4.1`, `libsoup-3.0`, `libssl`, etc. AppImage and Flatpak packaging via `tauri-bundler`.

### 2. Code signing

- Windows: SignTool with EV cert (HSM-backed via Azure Key Vault or 1Password CLI). MSI and EXE both signed.
- macOS: Developer ID signing + notarization (notarytool) + stapling. Universal binary (arm64 + x86_64).
- Linux: GPG-signed AppImage (using `gpg --detach-sign`); Flatpak signing via Flatpak's own mechanism.

### 3. Auto-update verification

Tauri updater configured with the public key matching the signing key. Negative test in CI: a tampered update package is rejected.

### 4. Installer UX

Windows: MSI wizard or NSIS. macOS: DMG with drag-to-Applications. Linux: AppImage runs directly; Flatpak via flathub-style metadata.

### 5. Distribution channels

Stable, beta, nightly. Release pipeline tags select the channel. Auto-update respects the user's channel.

### 6. Tests

- Build matrix produces signed artifacts on all three OSes.
- Smoke install + launch in clean VMs (CI ephemeral runners).
- Update signature verification negative test.
- Post-install file integrity (manifest hashes) verified.

## Guard-rails

**[GUARD-P8-1]** Signing keys never on developer laptops. Use HSM or platform-native key storage.

**[GUARD-P8-2]** Unsigned builds may still be produced for development but are never uploaded to a release channel.

**[GUARD-P8-3]** macOS notarization is non-skippable for release.

## Acceptance criteria

1. Signed installers on all three OSes published to staging.
2. Auto-update from version N to N+1 verified on each OS.
3. Tampered update rejected.
4. Linux AppImage runs on Ubuntu 22.04 LTS, Fedora 39, Arch (latest).

## Review gate

- Signing runbook (`docs/runbooks/signing.md`) is current.
- Release pipeline secrets are OIDC-scoped, not static.
- `licenses/` directory ships with all third-party licenses.

## Out of scope

- Snap, deb, rpm — community demand only.
- Microsoft Store / Mac App Store submissions (later).

## When you finish

PR title `Phase 08: Linux + signing`. Update `STATUS.md`.
