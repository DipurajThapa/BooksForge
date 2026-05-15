#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Drift check: the TS-side `BOILERPLATE_KINDS` array in
//! `Stage13_14_Export.tsx` must list every variant of the Rust
//! `BoilerplateKind` enum with the matching `front` flag.
//!
//! Why this exists: the UI's "add page" buttons read from the local
//! TS array. A new Rust variant (or a flipped `is_front_matter`)
//! must show up in the UI, but TypeScript can't enforce parity with
//! a Rust enum at compile time. This test closes that loop at CI
//! time so the writer can never end up unable to add a new boilerplate
//! kind that the export pipeline supports.
//!
//! The test reads Stage13's source file as text and parses out the
//! `id` + `front` columns; no JS runtime required.

use booksforge_domain::BoilerplateKind;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

const STAGE13_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../apps/desktop/src-ui/src/stages/Stage13_14_Export.tsx",
);

/// Parse the `BOILERPLATE_KINDS` array literal out of Stage13's
/// source. Returns `id → front` mappings. Cheap text parser — the
/// array literal format is stable and unique in the file.
fn parse_stage13_kinds() -> HashMap<String, bool> {
    let path = PathBuf::from(STAGE13_PATH);
    let src = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));

    // Locate the array literal.
    let array_start = src
        .find("const BOILERPLATE_KINDS")
        .expect("Stage13 has no BOILERPLATE_KINDS constant");
    let body_start = src[array_start..]
        .find('[')
        .map(|i| array_start + i)
        .expect("BOILERPLATE_KINDS has no opening `[`");
    let body_end = src[body_start..]
        .find("];")
        .map(|i| body_start + i)
        .expect("BOILERPLATE_KINDS has no closing `];`");
    let body = &src[body_start..=body_end];

    // Each entry: `{ id: "snake_case", label: "…", front: true|false }`.
    let mut out = HashMap::new();
    for line in body.lines() {
        let line = line.trim();
        let id_start = match line.find("id: \"") {
            Some(i) => i + "id: \"".len(),
            None => continue,
        };
        let id_end = line[id_start..]
            .find('"')
            .map(|i| id_start + i)
            .expect("unterminated id string");
        let id = line[id_start..id_end].to_owned();

        let front = if line.contains("front: true") {
            true
        } else if line.contains("front: false") {
            false
        } else {
            panic!("entry {id:?} has no `front` flag in Stage13 array");
        };
        out.insert(id, front);
    }
    out
}

#[test]
fn every_rust_variant_is_listed_in_stage13() {
    let ts = parse_stage13_kinds();
    let mut missing = Vec::new();
    for &kind in BoilerplateKind::ALL {
        if !ts.contains_key(kind.id()) {
            missing.push(kind.id());
        }
    }
    assert!(
        missing.is_empty(),
        "BOILERPLATE_KINDS DRIFT: Rust BoilerplateKind has variants the UI doesn't show. \
         Add these entries to `apps/desktop/src-ui/src/stages/Stage13_14_Export.tsx::BOILERPLATE_KINDS`:\n  {}",
        missing.join("\n  "),
    );
}

#[test]
fn stage13_does_not_list_phantom_variants() {
    let ts = parse_stage13_kinds();
    let known: std::collections::HashSet<&str> =
        BoilerplateKind::ALL.iter().map(|k| k.id()).collect();
    let mut phantom: Vec<&str> = ts
        .keys()
        .map(|s| s.as_str())
        .filter(|id| !known.contains(*id))
        .collect();
    phantom.sort_unstable();
    assert!(
        phantom.is_empty(),
        "BOILERPLATE_KINDS DRIFT: Stage13 lists ids that don't match any Rust variant \
         (probably a typo or a renamed enum). Remove or fix:\n  {}",
        phantom.join("\n  "),
    );
}

#[test]
fn front_matter_flag_matches_rust() {
    let ts = parse_stage13_kinds();
    let mut diffs = Vec::new();
    for &kind in BoilerplateKind::ALL {
        let ts_front = match ts.get(kind.id()) {
            Some(b) => *b,
            None => continue, // covered by the missing-variant test
        };
        let rust_front = kind.is_front_matter();
        if ts_front != rust_front {
            diffs.push(format!(
                "{}: Stage13 says front={ts_front}, Rust says is_front_matter()={rust_front}",
                kind.id()
            ));
        }
    }
    assert!(
        diffs.is_empty(),
        "BOILERPLATE_KINDS FRONT-MATTER DRIFT:\n  {}",
        diffs.join("\n  "),
    );
}
