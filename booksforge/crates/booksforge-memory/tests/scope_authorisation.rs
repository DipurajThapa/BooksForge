//! Scope-authorisation integration tests at the `booksforge-memory`
//! crate boundary.
//!
//! Closes part of EXTERNAL_AUDIT_BACKLOG.md #21.
//!
//! `booksforge-memory` is a thin re-export facade over
//! `booksforge_domain::memory`.  The domain crate has unit tests
//! inline; this file adds a cross-agent scope-authorisation matrix
//! that exercises the re-exports through the public surface
//! `booksforge_memory::*` so the facade contract is itself tested
//! (a regression that loses an item from the `pub use` list breaks
//! these tests).

use booksforge_memory::{
    allowed_write_scopes, authorise_write, MemoryError, MemoryScope,
};

/// Every agent that's named in AGENTS.md, plus a fictional one to
/// confirm the deny-by-default fallback.
const AGENT_IDS: &[&str] = &[
    "memory-curator",
    "vocab-dictionary",
    "continuity",
    "copyeditor",
    "outline-architect",
    "intake",
    "chapter-drafter",
    "dev-editor",
    "humanization",
    "final-review-editor",
    "proposal-validator",
    "unknown-agent-9001",
];

const ALL_SCOPES: &[MemoryScope] = &[
    MemoryScope::Book,
    MemoryScope::Chapter,
    MemoryScope::Entity,
    MemoryScope::Style,
];

#[test]
fn allowed_write_scopes_is_disjoint_per_agent_per_audit() {
    // Spot-check the per-agent allowlist matches the table in
    // AGENTS.md §3 (and outputs/MEMORY_SYSTEM.md).  Each entry below
    // is a `(agent, expected_scopes)` pair that, if changed, must
    // also be reflected in the spec doc.
    let cases: &[(&str, &[MemoryScope])] = &[
        ("memory-curator",    &[MemoryScope::Book, MemoryScope::Chapter, MemoryScope::Entity]),
        ("vocab-dictionary",  &[MemoryScope::Style, MemoryScope::Entity]),
        ("continuity",        &[MemoryScope::Entity]),
        ("copyeditor",        &[MemoryScope::Style]),
        ("outline-architect", &[MemoryScope::Book]),
        // Read-only agents:
        ("intake",            &[]),
        ("chapter-drafter",   &[]),
        ("dev-editor",        &[]),
        ("humanization",      &[]),
        ("final-review-editor", &[]),
        ("proposal-validator", &[]),
        // Unknown agents: deny-by-default:
        ("unknown-agent-9001", &[]),
    ];

    for (agent, expected) in cases {
        let actual = allowed_write_scopes(agent);
        assert_eq!(
            actual, *expected,
            "scope mismatch for '{agent}': expected {expected:?}, got {actual:?}",
        );
    }
}

#[test]
fn authorise_write_accepts_only_listed_scopes() {
    for agent in AGENT_IDS {
        let allowed = allowed_write_scopes(agent);
        for &scope in ALL_SCOPES {
            let res = authorise_write(agent, scope);
            if allowed.contains(&scope) {
                assert!(
                    res.is_ok(),
                    "{agent} should be allowed to write {scope:?}, got {res:?}",
                );
            } else {
                let err = res.expect_err(&format!(
                    "{agent} should be denied write to {scope:?}",
                ));
                assert!(
                    matches!(err, MemoryError::OutOfScopeWrite { .. }),
                    "expected OutOfScopeWrite, got {err:?}",
                );
            }
        }
    }
}

#[test]
fn out_of_scope_write_error_carries_agent_and_scope() {
    let err = authorise_write("intake", MemoryScope::Book)
        .expect_err("intake must NOT write to book scope");
    let s = err.to_string();
    assert!(s.contains("intake"), "error should mention agent: {s}");
    assert!(s.contains("Book"), "error should mention scope: {s}");
}

#[test]
fn unknown_agent_is_denied_for_all_scopes() {
    for &scope in ALL_SCOPES {
        let res = authorise_write("not-a-real-agent", scope);
        assert!(
            matches!(res, Err(MemoryError::OutOfScopeWrite { .. })),
            "unknown agent must be denied for {scope:?}, got {res:?}",
        );
    }
}

#[test]
fn empty_string_agent_is_denied() {
    for &scope in ALL_SCOPES {
        assert!(authorise_write("", scope).is_err());
    }
}

#[test]
fn memory_scope_string_round_trip() {
    for &s in ALL_SCOPES {
        let serialised = s.as_str();
        let back = MemoryScope::from_str(serialised);
        assert_eq!(back, Some(s));
    }
    assert_eq!(MemoryScope::from_str("garbage"), None);
}
