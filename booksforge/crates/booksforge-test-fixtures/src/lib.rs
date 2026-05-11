//! Shared test fixtures (dev-dependency only).
//!
//! Provides deterministic, seeded instances of all domain types so tests
//! don't repeat boilerplate construction.  ULID seeds are fixed so test
//! output is reproducible across runs.

#![forbid(unsafe_code)]
// BACKLOG §C4 — this crate is a dev-dependency-only fixture library
// (declared under `[dev-dependencies]` everywhere it is used; never
// shipped in any binary). The `#![cfg(test)]` that previously gated
// the crate root made it invisible to OTHER crates' integration tests
// (which is precisely what the fixtures are for). Allow the policy
// lints unconditionally so fixture builders can use `.unwrap()`
// freely without forcing every dependent test crate to whitelist them.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

pub mod entities;
pub mod mock_ollama;
pub mod nodes;
pub mod projects;
