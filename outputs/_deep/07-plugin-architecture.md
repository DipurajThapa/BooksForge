# Plugin Architecture — BooksForge

**Version:** 1.0.0-draft  •  **Status:** For review  •  **Date:** 2026-05-06

---

## 1. Goals

The plugin system exists to let small communities, editors, and presses ship vertical packs without forking the codebase. It must be (a) **safe**: a malicious or buggy plugin cannot exfiltrate manuscripts or destabilise the host; (b) **expressive**: plugin authors can ship templates, validators, AI prompts, importers, exporters, and UI panels; (c) **stable**: a plugin written for V1.0 keeps working through V1.x; (d) **distributable**: marketplace, sideload, and developer-mode all work with the same package format.

## 2. Plugin types

| Type | Purpose | Phase | Sandbox |
|------|---------|-------|---------|
| `template` | Project skeleton + style rules + preset prompts | 1.0 | Declarative only — no executable code |
| `validator` | Pure rule that inspects a project and reports issues | 1.0 | WASM (compute) |
| `prompt-pack` | A bundle of versioned AI prompt templates | 1.0 | Declarative only |
| `importer` | Read a foreign format → project tree | 1.5 | WASM with capability `read-file:<mime>` |
| `exporter` | Project → foreign format file | 1.5 | WASM with capability `write-export-file` |
| `ui-panel` | Side-panel UI; e.g., Zotero browser, beat-sheet view | 1.5 | Isolated WebView (UI sandbox) |
| `script` | Background task plugin (e.g., series consistency periodic check) | 2.0 | WASM with explicit capabilities |

A single plugin package may declare multiple types in one manifest.

## 3. Package format

A plugin is a directory bundle with extension `.booksforge-plugin` (zipped for distribution). Marketplace ships them zipped + signed. Developer mode loads them unzipped.

```
my-plugin.booksforge-plugin/
├── plugin.toml                # Manifest (mandatory)
├── README.md                  # Shown in marketplace listing
├── LICENSE
├── icon.png                   # 512×512
├── templates/                 # if type includes 'template'
│   └── romance-mass-market/
│       ├── template.toml
│       ├── styles.toml
│       └── scaffold.json
├── validators/                # if type includes 'validator'
│   └── romance-trope-check.wasm
├── prompts/                   # if type includes 'prompt-pack'
│   └── beta-reader.prompt.toml
├── ui/                        # if type includes 'ui-panel'
│   └── dist/                  # Built static assets (HTML/JS/CSS)
└── i18n/                      # locale strings
    ├── en.json
    └── es.json
```

## 4. `plugin.toml` manifest

```toml
[plugin]
id = "com.example.romance-pack"           # reverse-DNS unique
name = "Romance Mass Market Pack"
version = "1.2.0"
description = "Templates, validators, and beta-reader prompts for mass-market romance."
author = "Example Books LLC"
homepage = "https://example.com/romance-pack"
license = "MIT"
icon = "icon.png"

[compatibility]
booksforge_min = "1.0.0"
booksforge_max = "<2.0.0"                  # SemVer range

[contains]
templates = ["templates/romance-mass-market"]
validators = [
  { id = "trope.black-moment-present.v1", file = "validators/romance-trope-check.wasm" }
]
prompt_packs = ["prompts/beta-reader.prompt.toml"]
ui_panels = []

[capabilities]
# Each capability has rationale shown to user on install.
read-manuscript = { rationale = "Validators inspect chapter text to flag missing romance beats." }
read-bibliography = { rationale = "Not used by this pack." } # listed = false would omit
write-suggestions = { rationale = "Beta-reader prompt produces inline comments as suggestions." }

[signing]
# Filled by marketplace at signing time, or empty for sideload
publisher = "..."
signature = "..."
```

The manifest is parsed in a strict mode: unknown top-level keys fail the install with a clear error (forward compatibility for *plugins* opening on *older* hosts is not guaranteed; that's the purpose of `compatibility.booksforge_min/max`).

## 5. Capability model

A plugin gets nothing except by capability. The full V1.0 capability list:

**Read capabilities** — `read-manuscript`, `read-bibliography`, `read-entities`, `read-snapshot`, `read-asset:<mime-glob>`, `read-export-history`.

**Write capabilities** — `write-suggestions` (creates suggestion objects only; cannot directly mutate the manuscript), `write-comment` (creates comments), `write-bibliography-entry`, `write-validator-issue` (only for validators), `write-export-file` (only for exporters; writes to a host-supplied path, not arbitrary FS).

**Network capabilities** — `network-domain:<host>` (one capability per host; wildcard subdomains require explicit `*.host` form). The host proxies the request and applies CORS/CSP. `network-public-internet` is a "dangerous" superpower, never granted to marketplace plugins by default.

**System capabilities** — `read-file-from-user` (host shows a file picker; the plugin gets the contents not the path), `write-file-to-user` (host shows a save dialog).

**UI capabilities** — `ui-panel-side`, `ui-panel-modal`, `ui-shortcut:<scope>`.

**Special** — `ai-prompt-overlay` (inject prompt fragments into AI calls; rate-limited), `template-provider`, `validator-engine`.

The user sees the requested set on install, with the plugin-supplied rationale per capability. Granted capabilities are stored in `plugin_installs.capabilities_granted` and re-prompted on update if the requested set grows.

## 6. Runtime

### 6.1 Compute plugins (WASM)

Loaded by `wasmtime` with WASI-preview2. Default-deny everything: no FS, no network, no clocks, no env, no random. The host injects a small, capability-gated host API the plugin imports:

```wit
interface booksforge:plugin {
  // Runtime info
  get-booksforge-version: func() -> string;
  get-plugin-version: func() -> string;

  // Capability-gated reads
  read-manuscript-scene: func(node-id: string) -> result<scene-data, error>;
  read-bibliography-entry: func(csl-key: string) -> result<csl-json, error>;
  // ... etc

  // Capability-gated writes
  emit-validator-issue: func(issue: issue) -> result<unit, error>;
  emit-suggestion: func(suggestion: suggestion) -> result<unit, error>;

  // Network (proxied)
  http-fetch: func(req: http-request) -> result<http-response, error>;
}
```

The host validates every call against the granted capability set. Resource caps: 256 MB memory, 30 s wall-clock, 10 s CPU. Violations kill the plugin task and surface a typed error.

### 6.2 UI plugins (WebView)

Each UI plugin gets an isolated WebView with a `booksforge://plugin/<id>/` origin. CSP is strict by default. The plugin has access to a JS SDK (`@booksforge/plugin-sdk`) that wraps host IPC. All host calls go through the same capability checks as compute plugins.

UI plugins **cannot** directly access the editor's ProseMirror state. They register declarative side-panels and event subscriptions; for editor manipulation, they emit suggestions or comments.

### 6.3 Templates and prompt-packs

Pure-data plugins. The host loads and validates the TOML/JSON; no code runs. Templates contribute to the new-project picker; prompt-packs contribute to the AI preset menu.

## 7. Plugin lifecycle

```
Install → (signature verify if signed) → manifest parse → capability prompt
       → user approves → copy to data dir → register
On open project → check `plugins/enabled.toml` → lazy-load on first use
On update available → diff capability set → re-prompt if expanded
On disable → unload runtime; data preserved
On uninstall → unload, remove files, remove from `plugin_installs`
```

Per-project enable: even if installed, plugins are off in a new project until the user enables them per-project. (Exception: ones without sensitive capabilities can be marked "auto-enable" by the user globally.)

## 8. Plugin SDK

We ship two SDKs: **TypeScript** (for UI plugins and prompt-pack/template authoring tools) and **Rust** (compiled to WASM for compute plugins). The Rust SDK provides typed wrappers around the WIT interface and a testing harness.

The SDK contract is what we promise to keep stable for 6 months minimum. Breaking changes are versioned in the SDK and gated by `compatibility.booksforge_min`.

A `plugin-cli` tool scaffolds new plugins: `booksforge plugin new --type validator --name my-rule`. It also runs `plugin lint`, `plugin test` (against fixture projects), `plugin pack` (create the bundle), and `plugin publish` (upload to marketplace draft).

## 9. Marketplace

V1.5 deliverable. Web-side: list, search, ratings, install. Server-side: plugin submission flow, signing, virus scan, basic security review (manifest sanity, capability sanity, code-review for popular ones). Revenue: 80/20 split to publisher; payouts via Stripe Connect.

The marketplace is **not the only distribution channel**. Sideload remains supported. The freedom to install an unsigned plugin from a friend (with the warning) is part of the product's privacy/independence story.

## 10. Compatibility and deprecation

The plugin SDK follows SemVer. Breaking changes bump the major version of `booksforge_max` requirement, giving plugin authors warning. Deprecations are announced one minor version before removal. The host loads only plugins whose `compatibility` range matches the running version. Out-of-range plugins are listed but disabled with a "needs update" badge.

## 11. Plugin security review checklist

Before signing a marketplace plugin we check (mostly automated): signature of upload matches publisher's registered key; manifest validates; capabilities requested match what the README claims; WASM modules pass static analysis (no suspicious imports beyond declared); no embedded credentials; UI plugin CSP is strict; declared rationale per capability is present and non-empty; size budget respected (≤20 MB unpacked); on-install behaviour does not request capabilities at runtime that weren't declared at install time.

## 12. Anti-abuse posture

The marketplace operator (BooksForge company) reserves the right to revoke a plugin's signature for confirmed abuse (data exfiltration, malware, ToS violations). Revocation propagates to all installs on next online check; users see a notification and the plugin is disabled. Offline users continue running until the next online check. We accept the offline-revocation gap as the price of offline-first.

## 13. Telemetry on plugins

Per project: which plugins are loaded, time spent in their host calls, error counts, capability use counts. **No content** of plugin operations is logged centrally. This supports plugin authors via aggregate stats (with their consent). Plugin authors get a free dashboard in their marketplace account.

## 14. Examples (as a sanity check)

A **Romance Trope validator plugin** declares `read-manuscript` and `write-validator-issue`. It scans for missing common beats. No network. Marketplace-signed. Average user grants on install with one prompt.

A **Zotero importer plugin** (V1.5) declares `read-bibliography`, `write-bibliography-entry`, and `network-domain:api.zotero.org`. The user enters their Zotero API key in the plugin's settings; the plugin imports references. Network access is logged per call.

A **Save the Cat beat-sheet UI panel** declares `ui-panel-side`, `read-manuscript` (for word-count rollups against beats), `write-suggestions`. It draws a 15-beat overlay onto the outline view.
