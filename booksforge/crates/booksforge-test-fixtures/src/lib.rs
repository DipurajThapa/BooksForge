//! Shared test fixtures (dev-dependency only).
//!
//! Provides deterministic, seeded instances of all domain types so tests
//! don't repeat boilerplate construction.  ULID seeds are fixed so test
//! output is reproducible across runs.

#![forbid(unsafe_code)]
#![cfg(test)]
// BACKLOG §C4: this entire crate is test-only (`#![cfg(test)]`); waive the
// strict policy lints so fixture builders can use `.unwrap()` freely.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

pub mod mock_ollama;
pub mod nodes;
pub mod projects;
pub mod entities;
