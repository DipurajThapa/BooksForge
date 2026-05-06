# Phase 13 — Plugin write capabilities (V2.0 begins)

## Goal

Enable importer and exporter plugin types with stricter sandbox + capability model than read-only plugins. Ship the Zotero importer as the flagship.

## Pre-conditions

V1.5 GA shipped.

## Inputs

1. `../_deep/07-plugin-architecture.md` — sections 2 (types), 5 (capabilities).
2. `../_deep/02-FSD-functional-specifications.md` — section 10 FR-PLUG-003 (V1.5 entries).

## Deliverables

### 1. Importer plugin type

Adds capabilities `read-file-from-user`, `write-bibliography-entry`, `write-entity`, `write-suggestions`. Importer plugins receive content via host file picker (host gives content, not the path). Output is staged in a "review" UI before persisting.

### 2. Exporter plugin type

Adds `write-export-file` capability — host supplies the destination path; plugin produces bytes. Exporter plugins are wrapped by the export orchestrator and inherit the pre-export snapshot.

### 3. Zotero importer plugin

Imports a user's Zotero library into the project bibliography. Uses Zotero web API. Capability: `network-domain:api.zotero.org`. Stores user's API key in OS keyring (host-mediated; plugin never sees key directly).

### 4. Tests

- Importer review-UI flow: nothing persists without explicit user accept.
- Exporter sandboxed: cannot write outside the host-supplied path.
- Zotero integration tested with VCR cassettes.

## Guard-rails

**[GUARD-P13-1]** Importer never directly mutates the project bundle — output staged for user review.

**[GUARD-P13-2]** Exporter cannot write to arbitrary FS paths — only to the host-supplied destination.

**[GUARD-P13-3]** Network capabilities are still per-domain; new wildcard requests trigger user re-prompt.

## Acceptance criteria

1. Zotero importer ships and imports a real Zotero library end-to-end.
2. Adversarial exporter trying to write outside its destination is killed and reported.

## When you finish

PR title `Phase 13: Plugin write capabilities`.
