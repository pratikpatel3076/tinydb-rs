//! # `storage` — Phase 2 (pager) and Phase 5 (buffer pool)
//!
//! Two layers, deliberately separated:
//!
//! - [`pager`]: the only code in the crate that talks to a database file. It
//!   moves fixed-size, 4096-byte pages in and out by page id.
//! - [`buffer_pool`]: a bounded cache of pages in front of the pager. It owns
//!   pinning, eviction, and dirty-page writeback.
//!
//! Rule of thumb: the pager is "correct but slow"; the buffer pool is "fast but
//! only correct if pin/unpin discipline is respected". Higher layers (btree,
//! wal recovery, sql) should depend on `buffer_pool::BufferPool`, not
//! `pager::Pager` directly.

pub mod buffer_pool;
pub mod pager;
