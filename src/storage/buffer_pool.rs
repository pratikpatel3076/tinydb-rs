//! # `BufferPool` — Phase 5
//!
//! A bounded cache of pages in front of [`crate::storage::pager::Pager`]. It is
//! the difference between "one syscall per page access" and "one syscall per
//! miss", and it is where dirty-page writeback and eviction live.
//!
//! ## Frame model
//!
//! ```text
//! frames (capacity = 3, LRU order shown by arrows)
//!
//!  +--------+   +--------+   +--------+
//!  | page 7 |<->| page 2 |<->| page 9 |
//!  | dirty  |   | clean  |   | clean  |
//!  | pin=1  |   | pin=0  |   | pin=0  |
//!  +--------+   +--------+   +--------+
//!    most recently used        least recently used
//! ```
//!
//! ## Invariants
//!
//! 1. `capacity > 0`. Constructing a pool with zero frames is a caller bug
//!    (panic or `assert!`, but do not return a silently useless pool).
//! 2. A page id appears in at most one frame.
//! 3. A frame with `pin_count > 0` is never evicted.
//! 4. Evicting a dirty frame writes it back through the pager *before* the
//!    frame is reused. A clean frame is dropped without I/O.
//! 5. `fetch` pins the returned frame. Every successful `fetch` must be paired
//!    with exactly one `unpin`. Leaking a pin eventually exhausts the pool.
//! 6. `allocate` returns an id that did not exist before the call; fetching it
//!    immediately afterwards must hit (the frame is already resident).
//!
//! ## Eviction policy
//!
//! Use LRU (or CLOCK; pick one and document it). The tests assert that an
//! unpinned least-recently-used page is preferred over a more recently used one,
//! so "random victim" is not acceptable. Determinism matters more than
//! sophistication here.
//!
//! ## Edge cases to get right
//!
//! - Every frame pinned and a miss arrives: return
//!   [`crate::Error::PoolExhausted`]. Never evict a pinned page, never grow
//!   silently.
//! - Re-fetching an already-pinned page: allowed; increment the pin count and
//!   count a hit. The caller must unpin as many times as it fetched.
//! - Unpinning a page that is not resident, or unpinning below zero: caller bug.
//!   Debug-assert it; do not corrupt the LRU list.
//! - `flush_all` must write back dirty frames but keep them resident. Dropping
//!   the pool should behave like `flush_all` (tests reopen the file after a
//!   scope ends), or document loudly that it does not.
//! - `page_count` must reflect allocated-but-not-yet-flushed pages.
#![allow(dead_code, unused_variables)]

use crate::error::Result;
use crate::storage::pager::{Page, PageId, Pager};

/// Default number of frames. Small enough to exercise eviction in tests.
pub const DEFAULT_CAPACITY: usize = 64;

/// Counters exposed for tests and (later) `EXPLAIN`-style diagnostics.
///
/// Counters are monotonic for the lifetime of the pool. They exist so that
/// eviction can be tested without guessing at internal state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PoolStats {
    /// `fetch` calls served from a resident frame (including re-pins).
    pub hits: u64,
    /// `fetch` calls that required a pager read.
    pub misses: u64,
    /// Frames reused for a different page. Includes clean and dirty victims.
    pub evictions: u64,
    /// Dirty frames written back to the pager (eviction or `flush_*`).
    pub writes: u64,
}

/// Bounded page cache in front of a [`Pager`].
///
/// Owns the pager: there is exactly one way into the file.
#[derive(Debug)]
pub struct BufferPool {
    pager: Pager,
    capacity: usize,
    stats: PoolStats,
    // TODO(owner): add frame table, LRU list, page->frame map, and pin counts.
}

impl BufferPool {
    /// Wrap `pager` in a pool with `capacity` frames.
    ///
    /// Panics if `capacity == 0`. The pool does not read anything at
    /// construction time; the first `fetch` does.
    pub fn new(pager: Pager, capacity: usize) -> Self {
        todo!("P5: assert capacity > 0 and initialise frames/LRU/map")
    }

    /// Number of frames the pool may use. Constant after construction.
    pub fn capacity(&self) -> usize {
        todo!("P5: return self.capacity")
    }

    /// Page count of the underlying file, including pages allocated by this
    /// pool but not yet flushed.
    pub fn page_count(&self) -> u64 {
        todo!("P5: report the pager's count plus any buffered allocations")
    }

    /// Snapshot of the hit/miss/eviction counters.
    pub fn stats(&self) -> PoolStats {
        todo!("P5: return self.stats")
    }

    /// Get a page, reading it in on a miss, and pin it.
    ///
    /// - On a miss, choose an unpinned victim (LRU), write it back if dirty,
    ///   read the requested page, then bump `misses`.
    /// - On a hit, bump `hits` and the frame's pin count.
    /// - Returns `Err(PoolExhausted)` if every frame is pinned.
    /// - The returned reference is valid until the next `&mut self` call; that
    ///   is enforced by the borrow checker, not by the pool.
    pub fn fetch(&mut self, id: PageId) -> Result<&mut Page> {
        todo!("P5: probe map, hit fast path, otherwise evict+read")
    }

    /// Allocate a fresh page and leave it resident (pinned once).
    ///
    /// On a full pool this may evict an unpinned page first. The returned id is
    /// already usable with [`BufferPool::fetch`] and must not be re-allocated.
    pub fn allocate(&mut self) -> Result<PageId> {
        todo!("P5: pager.allocate_page() then insert a pinned, dirty, zeroed frame")
    }

    /// Release one pin on `id`.
    ///
    /// The page stays resident (it becomes an eviction candidate). Pinning and
    /// unpinning must nest correctly; unpinning without a pin is a logic bug and
    /// should be caught by `debug_assert!`.
    pub fn unpin(&mut self, id: PageId) {
        todo!("P5: decrement the frame's pin count, debug_assert it stays >= 0")
    }

    /// Write `id` back to the pager if it is dirty, and mark it clean.
    ///
    /// No-op if the page is not resident or already clean. Does **not** fsync;
    /// durability is [`BufferPool::flush_all`] or the WAL's job.
    pub fn flush_page(&mut self, id: PageId) -> Result<()> {
        todo!("P5: locate frame, write_page if dirty, clear dirty bit")
    }

    /// Write back every dirty frame and fsync the pager.
    ///
    /// Frames stay resident and become clean. Safe to call with an empty pool.
    pub fn flush_all(&mut self) -> Result<()> {
        todo!("P5: iterate dirty frames, write them, then pager.flush()")
    }
}
