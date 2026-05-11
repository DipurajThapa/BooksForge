//! Round-trip tests for `TemplateManifest`.
//!
//! Closes part of EXTERNAL_AUDIT_BACKLOG.md #21 (Rust unit-test
//! backfill on the L3 `booksforge-template` crate).
//!
//! These tests are deliberately small but exercise the contract the
//! template crate is meant to enforce: the `TemplateManifest` type
//! survives serde-json + toml round-trips with all its fields
//! preserved, and `TemplateError` carries enough information for
//! upstream layers to surface useful messages.

// Integration tests live in a separate `tests/` crate, so the workspace's
// `cfg(test) allow expect_used` does not apply automatically. Opt in here.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use booksforge_template::{TemplateError, TemplateId, TemplateManifest};

fn sample() -> TemplateManifest {
    TemplateManifest {
        id: TemplateId("fiction-literary".to_string()),
        version: "1.0.0".to_string(),
        display_name: "Fiction — Literary".to_string(),
        mode: "fiction".to_string(),
        description: "Three-act / character-arc / sensory passes.".to_string(),
    }
}

#[test]
fn template_manifest_round_trips_through_json() {
    let m = sample();
    let s = serde_json::to_string(&m).expect("encode JSON");
    let back: TemplateManifest = serde_json::from_str(&s).expect("decode JSON");
    assert_eq!(back.id.0, "fiction-literary");
    assert_eq!(back.version, "1.0.0");
    assert_eq!(back.display_name, "Fiction — Literary");
    assert_eq!(back.mode, "fiction");
    assert_eq!(
        back.description,
        "Three-act / character-arc / sensory passes."
    );
}

#[test]
fn template_manifest_round_trips_through_toml() {
    let m = sample();
    let s = toml::to_string(&m).expect("encode TOML");
    let back: TemplateManifest = toml::from_str(&s).expect("decode TOML");
    assert_eq!(back.id.0, m.id.0);
    assert_eq!(back.version, m.version);
    assert_eq!(back.display_name, m.display_name);
    assert_eq!(back.mode, m.mode);
    assert_eq!(back.description, m.description);
}

#[test]
fn template_id_equality_is_string_based() {
    let a = TemplateId("fiction-generic-novel".to_string());
    let b = TemplateId("fiction-generic-novel".to_string());
    let c = TemplateId("fiction-thriller".to_string());
    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn template_id_implements_hash() {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    set.insert(TemplateId("a".to_string()));
    set.insert(TemplateId("a".to_string()));
    set.insert(TemplateId("b".to_string()));
    assert_eq!(set.len(), 2, "TemplateId should hash by inner string");
}

#[test]
fn template_error_messages_include_the_id() {
    let err = TemplateError::NotFound {
        id: "missing-template".to_string(),
    };
    let s = err.to_string();
    assert!(
        s.contains("missing-template"),
        "TemplateError::NotFound display should mention the id, got: {s}",
    );
}

#[test]
fn template_error_parse_carries_message() {
    let err = TemplateError::Parse {
        message: "expected `[manifest]` table at line 1".to_string(),
    };
    let s = err.to_string();
    assert!(s.contains("expected `[manifest]`"));
    assert!(s.contains("line 1"));
}
