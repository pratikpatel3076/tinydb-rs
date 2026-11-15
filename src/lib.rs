//! # tinydb-rs
//!
//! A small embedded database engine, written from scratch as a learning project.
//!
//! The crate is deliberately organised in layers that can be built and tested in
//! isolation. Each module's header names the phase that brings it to life; every
//! function body in the scaffold is `todo!("P<n>: ...")`.
//!
//! ## Layers (bottom-up)
//!
//! ```text
//! sql      P6  lexer -> parser -> planner -> executor
//! txn      P7  transaction manager, isolation
//! index    P3  B+tree over pages
//! storage  P2  pager (4096-byte pages) + P5 buffer pool
//! wal      P4  redo log + crash recovery
//! memtable P1  in-memory ordered key/value store (the warm-up)
//! ```
//!
//! ## Ground rules
//!
//! - No runtime dependencies. `std` only.
//! - No `unsafe`.
//! - Every public item is documented (`#![warn(missing_docs)]`).
//!
//! See `docs/ARCHITECTURE.md` for responsibilities and invariants, and
//! `docs/ROADMAP.md` for the phase-by-phase plan.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
// Scaffold-only allows: every body is `todo!()`, so parameters and fields are
// intentionally unused until the owning phase lands. Delete these two lines as
// the corresponding modules are implemented (CI will keep you honest).
#![allow(dead_code, unused_variables)]

pub mod error;
pub mod storage;

pub use error::{Error, Result};
