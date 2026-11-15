//! # `index` — Phase 3
//!
//! Ordered on-disk indexes. The B+tree is the first (and, for a long time, the
//! only) implementation. It sits above `storage::buffer_pool` and below the SQL
//! executor.
//!
//! Why a B+tree and not a binary search tree? Because a database reads/writes
//! in pages. A high fan-out tree keeps the height at 3–4 for millions of keys,
//! so a point lookup costs a handful of page reads instead of hundreds of
//! pointer chases.

pub mod btree;
