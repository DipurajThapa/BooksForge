# Toolchain — BooksForge

**Version:** 1.0.0  •  **Date:** 2026-05-06  •  **Authoritative for all tool version pins.**

This file is the single source of truth for exact tool versions. When `rust-toolchain.toml`, `.nvmrc`, `package.json`, or any CI matrix references a version, it must match what is here. Update this file first, then update the derived files.

---

## 1. Rust

| Property | Value |
|----------|-------|
| Stable channel | `1.82.0` |
| Edition | `2021` |
| MSRV (minimum supported) | `1.82.0` |
| Toolchain file | `booksforge/rust-toolchain.toml` |
| Profile | `minimal` + components: `rustfmt`, `clippy` |

`rust-toolchain.toml` content (seed, copy to `booksforge/` at MZ-01):

```toml
[toolchain]
channel = "1.82.0"
components = ["rustfmt", "clippy"]
profile = "minimal"
```

Rationale: Edition 2021 for `let-else`, `array::IntoIter`, and `std::future` improvements. 1.82 is the baseline that ships stable `async fn` in traits, needed by `booksforge-ollama`'s `OllamaClient` trait.

---

## 2. Node.js

| Property | Value |
|----------|-------|
| Version | `22.11.0` (LTS "Jod") |
| Version file | `booksforge/.nvmrc` |
| Engine constraint | `"node": ">=22.0.0"` in root `package.json` |

`.nvmrc` content:

```
22.11.0
```

Rationale: Node 22 LTS ships V8 12.4, which has native `Array.fromAsync`, better `--experimental-vm-modules` support for Vitest, and a stable `fetch` global.

---

## 3. pnpm

| Property | Value |
|----------|-------|
| Version | `9.12.3` |
| Pinned via | `packageManager` field in root `package.json` |
| Workspace protocol | `pnpm-workspace.yaml` |

Root `package.json` snippet:

```json
{
  "packageManager": "pnpm@9.12.3",
  "engines": {
    "node": ">=22.0.0",
    "pnpm": ">=9.0.0"
  }
}
```

---

## 4. Tauri

| Property | Value |
|----------|-------|
| Version | `2.2.3` |
| CLI version | `2.2.3` (must match `tauri-build` and `tauri` crate) |
| Minimum WebView | macOS 13+ (WebKit); Windows 10+ (WebView2) |
| Pinned in | `apps/desktop/Cargo.toml` + `apps/desktop/package.json` |

`Cargo.toml` snippet:

```toml
[dependencies]
tauri = { version = "=2.2.3", features = ["protocol-asset"] }
tauri-build = { version = "=2.2.3" }
```

`package.json` snippet:

```json
{
  "devDependencies": {
    "@tauri-apps/cli": "2.2.3"
  },
  "dependencies": {
    "@tauri-apps/api": "2.2.3"
  }
}
```

Pinned to an exact patch to prevent silent behavioural changes in the Tauri IPC layer. Upgrade requires an ADR entry and a full CI pass.

---

## 5. TypeScript and front-end tooling

| Tool | Version | Note |
|------|---------|------|
| TypeScript | `5.6.3` | `strict: true`; `noUncheckedIndexedAccess` on |
| Vite | `5.4.x` | Latest 5.4 patch |
| Vitest | `2.1.x` | Unit tests for TS / React |
| React | `18.3.x` | React 19 deferred — Tauri 2.2 WebView compat confirmed for 18.x |
| ts-rs | `10.0.x` | Codegen Rust → TS; drift fails CI |

`tsconfig.json` base (seed, copy to `apps/desktop/src-ui/` at MZ-01):

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "strict": true,
    "noImplicitAny": true,
    "strictNullChecks": true,
    "noUncheckedIndexedAccess": true,
    "noEmit": true,
    "jsx": "react-jsx",
    "baseUrl": ".",
    "paths": {
      "@booksforge/*": ["../../packages/*/src"]
    }
  }
}
```

---

## 6. Testing tools

| Tool | Version | Used for |
|------|---------|----------|
| Playwright | `1.47.x` | E2E against the built Tauri app |
| `cargo-tarpaulin` | `0.31.x` | Rust line-coverage reporting (CI) |
| `proptest` | `1.5.x` | Property-based tests in Rust domain crates |

---

## 7. CI matrix

| Runner | OS | Architecture | Role |
|--------|----|--------------|------|
| `macos-14` | macOS 14 Sonoma | Apple Silicon (M1) | Gating |
| `macos-13` | macOS 13 Ventura | x64 | Gating |
| `windows-2022` | Windows Server 2022 | x64 | Gating |
| `ubuntu-22.04` | Ubuntu 22.04 | x64 | Non-gating smoke |

The Ubuntu job runs `cargo build` and `cargo test` only — it catches Linux drift without blocking PRs.

---

## 8. Sidecar binaries (bundled with installer)

| Binary | Version | Source | Note |
|--------|---------|--------|------|
| Pandoc | `3.5` | pandoc.org | GPL; sidecar process only — never statically linked |
| EPUBCheck | `5.3.0` | w3c/epubcheck | Java-based; bundled JRE TBD in M5. Pin bumped from 5.1.0 → 5.3.0 on 2026-05-09 (BACKLOG §A12); BF-E2E test confirmed 0 errors / 0 warnings on the 8-chapter test EPUB |
| Typst | `0.14.x` | typst/typst | Apache-2.0; sidecar process for PDF interior generation. Single ~30 MB binary, no LaTeX dependency. Replaces the unspecified PDF engine that pandoc previously needed. Wrapped by `booksforge-export-typst` (BACKLOG §A11) |

Pandoc, EPUBCheck, and Typst versions are pinned in their respective sidecar TOML files (`booksforge-export-pandoc/sidecar.toml`, `booksforge-epubcheck/sidecar.toml`, `booksforge-export-typst/sidecar.toml`). The SHA-256 of each binary is recorded and verified at startup.

---

## 9. How to update a version

1. Update the version in this file.
2. Update every derived config file (`rust-toolchain.toml`, `.nvmrc`, `package.json`, `Cargo.toml`, CI YAML).
3. Append an entry to `ARCHITECTURE_DECISIONS.md` if the change is a major version bump or affects the CI matrix.
4. Open a PR with the `[toolchain]` tag in the title; CI must be fully green on the new version before merge.
