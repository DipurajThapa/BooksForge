//! Memory subsystem facade.
//!
//! The canonical types live in `booksforge-domain::memory`; this crate
//! re-exports them so anything that wants to reach the memory layer has a
//! single import path.  The crate is kept as a separate compilation unit so
//! later non-trivial logic (e.g. value-set builders, contradiction
//! detectors) can land here without bloating the domain crate.

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

pub use booksforge_domain::memory::{
    allowed_write_scopes, authorise_write, MemoryEntry, MemoryError, MemoryScope,
};
