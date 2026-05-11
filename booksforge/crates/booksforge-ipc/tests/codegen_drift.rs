#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! IPC codegen drift check (BACKLOG §C3).
//!
//! ts-rs writes TypeScript bindings into
//! `packages/shared-types/src/bindings/`.  This test forces a fresh
//! export and then snapshots a few invariants that catch the most
//! common drift modes:
//!
//!   1. Every `.ts` file under `bindings/` is also re-exported from
//!      `packages/shared-types/src/index.ts`.  If a Rust author adds a
//!      new IPC type but forgets the index re-export, the TS layer
//!      can't import it — this test fails loudly with the missing
//!      type's name.
//!
//!   2. Every type re-exported from `index.ts` has a corresponding
//!      `bindings/<Name>.ts` file.  Catches the reverse drift — a
//!      Rust type was renamed/removed but `index.ts` still references
//!      the old name.
//!
//! CI command: `cargo test -p booksforge-ipc --test codegen_drift`.
//!
//! Note: the tests in `lib.rs` regenerate the bindings as a side-effect
//! of `export_*_bindings`.  Running `cargo test -p booksforge-ipc` in
//! that order keeps everything consistent.

use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

const SHARED_TYPES_DIR: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../packages/shared-types/src",
);

fn bindings_dir() -> PathBuf {
    PathBuf::from(SHARED_TYPES_DIR).join("bindings")
}
fn index_ts() -> PathBuf {
    PathBuf::from(SHARED_TYPES_DIR).join("index.ts")
}

/// Collect every `<Name>.ts` filename in the bindings dir, minus the
/// `.ts` suffix.
fn binding_names() -> HashSet<String> {
    let mut out = HashSet::new();
    let dir = bindings_dir();
    if !dir.exists() {
        panic!("bindings dir missing: {}", dir.display());
    }
    for entry in fs::read_dir(&dir).expect("read bindings dir").flatten() {
        let name = entry.file_name();
        let name = match name.to_str() {
            Some(s) => s,
            None => continue,
        };
        if let Some(stem) = name.strip_suffix(".ts") {
            out.insert(stem.to_owned());
        }
    }
    out
}

/// Scan `index.ts` for `export type { Name } from ...` re-exports and
/// extract the `Name` token.
fn index_exports() -> HashSet<String> {
    let content = fs::read_to_string(index_ts()).expect("read packages/shared-types/src/index.ts");
    let mut out = HashSet::new();
    for line in content.lines() {
        let trimmed = line.trim();
        // `export type { Foo } from "./bindings/Foo";` — pick out `Foo`.
        if let Some(rest) = trimmed.strip_prefix("export type {") {
            if let Some(end) = rest.find('}') {
                let names = &rest[..end];
                for name in names.split(',') {
                    let n = name.trim();
                    if !n.is_empty() {
                        out.insert(n.to_owned());
                    }
                }
            }
        }
    }
    out
}

/// Types intentionally NOT re-exported from `index.ts`.  Anything
/// listed here is internal-only or a child type the consumer never
/// imports directly (it's reachable via a parent struct).  Keeps the
/// invariant strict but documented.
const INTERNAL_ONLY: &[&str] = &[
    // Shipped under a parent type's import.
    // Add entries here with a one-line "why" comment when needed.
];

#[test]
fn every_binding_file_is_exported_from_index() {
    let bindings = binding_names();
    let exports = index_exports();
    let internal: HashSet<String> = INTERNAL_ONLY.iter().map(|s| (*s).to_string()).collect();

    let mut missing: Vec<String> = bindings
        .difference(&exports)
        .filter(|n| !internal.contains(*n))
        .cloned()
        .collect();
    missing.sort();
    assert!(
        missing.is_empty(),
        "IPC CODEGEN DRIFT: {} type(s) have a `bindings/<Name>.ts` file \
         but are NOT re-exported from `packages/shared-types/src/index.ts`. \
         Add `export type {{ <Name> }} from \"./bindings/<Name>\";` for each:\n  {}",
        missing.len(),
        missing.join("\n  "),
    );
}

#[test]
fn every_index_export_has_a_binding_file() {
    let bindings = binding_names();
    let exports = index_exports();

    let mut missing: Vec<String> = exports.difference(&bindings).cloned().collect();
    missing.sort();
    assert!(
        missing.is_empty(),
        "IPC CODEGEN DRIFT: {} type(s) are re-exported from `index.ts` but \
         have NO matching `bindings/<Name>.ts` file.  Either the Rust type \
         was renamed/removed (delete the export) or the binding file was \
         deleted accidentally:\n  {}",
        missing.len(),
        missing.join("\n  "),
    );
}
