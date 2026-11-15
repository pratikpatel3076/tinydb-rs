//! # B+tree — Phase 3
//!
//! An ordered map from byte-string keys to byte-string values, stored in
//! 4096-byte pages. This is the first "real" data structure in the project and
//! the first place where page layout, fan-out, and splits matter.
//!
//! ## Shape
//!
//! ```text
//!                 [ internal: 30 | 70 ]            root
//!                /          |         \
//!      [10|20|30]      [40|50|70]      [80|90]      internal (leaves at depth 1)
//!       /   |   \       /   |   \       /    \
//!    leaves ...                                    leaf level (linked)
//!    +------+   +------+   +------+
//!    | 1..9 |-->|10..29|-->|30..39|--> ...
//!    +------+   +------+   +------+
//! ```
//!
//! B+tree, specifically: **values live only in leaves**, internal nodes hold
//! separator keys and child page ids, and leaves are linked left-to-right so a
//! range scan walks siblings instead of re-descending from the root.
//!
//! ## Invariants (your implementation must maintain all of these)
//!
//! 1. Every leaf is at the same depth. The tree is balanced after every write.
//! 2. Keys within a node are strictly ascending (duplicates are upserts).
//! 3. A separator key is the smallest key in the child to its **right**
//!    (choose "smallest in right child" or "largest in left child" and never
//!    mix the two; the scan and split code will punish inconsistency).
//! 4. Every non-root node is at least half full, except that P3 may leave
//!    underfull nodes after a delete. Document the relaxation instead of
//!    pretending.
//! 5. `root` in the header points at a valid page (leaf, or internal).
//! 6. Leaf sibling links form a null-terminated chain in ascending order:
//!    the last leaf has `next == None`.
//! 7. A node's free space accounting is exact: writing a node that does not fit
//!    is a corruption bug, not an excuse to split mid-write.
//!
//! ## Page layout (suggestion, not law)
//!
//! ```text
//! offset 0:  magic u16 | version u16 | flags u8 | (reserved)
//! offset 8:  slot_count u16 | free_start u16 | free_end u16 | next_page u64
//! offset 24: slot array (offset,len pairs, ascending by key)
//! ...        free space grows down from PAGE_SIZE
//! offset 4096: key/value bytes grow up from the end
//! ```
//!
//! You may choose anything that fits in 4096 bytes, but write the layout down
//! in this comment before you write the code. Header page (page 0) needs at
//! least: magic, format version, page size, root page id, leaf level, and the
//! key count if you want `key_count` to be O(1).
//!
//! ## Edge cases to get right
//!
//! - Empty tree: a single empty leaf. `scan` returns `[]`; `get` returns `None`.
//! - Split the root: create a new root, increment the tree height. Do not
//!   special-case "the root is a leaf" beyond that.
//! - Inserting a key larger than everything already present, and smaller than
//!   everything: both must land in the rightmost/leftmost leaf.
//! - Duplicate key: replace the value in place; do not add a second entry.
//! - Value too large for one page (after split, it still does not fit): return
//!   an error. Overflow pages are out of scope for P3.
//! - `delete`: for P3, removing from a leaf without rebalancing is acceptable;
//!   an empty leaf must still be reachable by scan, and the tree must stay
//!   searchable. Merging/redistribution is a stretch goal. Never leave a
//!   dangling sibling pointer.
//! - Reopening after a clean shutdown: `open` reads the header and trusts the
//!   root id; it must validate magic/version rather than panicking on garbage.
//!
//! ## Testing contract
//!
//! `tests/phase3_btree.rs` compares the tree against `std::collections::BTreeMap`
//! under randomized insert/delete sequences (proptest). Treat that test as the
//! specification: if the model and the tree disagree, the tree is wrong.
#![allow(dead_code, unused_variables)]

use crate::error::Result;
use crate::storage::buffer_pool::BufferPool;

/// Key type for the tree. Opaque bytes, unsigned-lexicographic order.
pub type Key = Vec<u8>;

/// Value type for the tree. Opaque bytes; stored inline in a leaf.
pub type Value = Vec<u8>;

/// An on-disk B+tree.
///
/// Owns the buffer pool, and therefore the pager and the file. Only one tree
/// may own a given file at a time.
#[derive(Debug)]
pub struct BTree {
    pool: BufferPool,
    // TODO(owner): cache the root page id, height, and key count from the header
    // so that reads do not need a header page fetch on every operation.
}

impl BTree {
    /// Create a new tree in an already-open (empty or reusable) database file.
    ///
    /// Writes page 0 (the header) and page 1 (an empty root leaf), then flushes
    /// so that a crash immediately after returns leaves a valid file. Must fail
    /// if the header page already contains a valid tree header (that would
    /// silently destroy data).
    ///
    /// Edge case: an empty pager (`page_count == 0`) is the expected input. If
    /// page 0 exists but has no valid header, return
    /// [`crate::Error::CorruptPage`] rather than overwriting it.
    pub fn create(pool: BufferPool) -> Result<Self> {
        todo!("P3: init page 0 header + page 1 empty leaf, flush, return BTree")
    }

    /// Open an existing tree.
    ///
    /// Validates the header's magic and format version and loads the root id and
    /// height. Does not walk the tree (a full validation pass is a nice
    /// `debug_assert`-only extra, not a P3 requirement).
    pub fn open(pool: BufferPool) -> Result<Self> {
        todo!("P3: read page 0, validate magic/version, cache root id")
    }

    /// Insert or replace a value.
    ///
    /// - Duplicate key: overwrite in place, keep one entry.
    /// - Leaf full: split, promote the separator, propagate to the parent, and
    ///   split the root if needed (height grows by one).
    /// - Empty key and empty value are legal.
    /// - Values that cannot fit in an empty leaf are an error (no overflow
    ///   pages yet).
    ///
    /// Invariant after return: all leaves are at the same depth and sibling
    /// links are intact.
    pub fn insert(&mut self, key: impl Into<Key>, value: impl Into<Value>) -> Result<()> {
        todo!("P3: descend to the leaf, insert/split, propagate separators upward")
    }

    /// Point lookup.
    ///
    /// Returns the value stored in a leaf, or `None`. Must not dirty any page.
    pub fn get(&mut self, key: &[u8]) -> Result<Option<Value>> {
        todo!("P3: descend internal nodes using separators, binary-search the leaf")
    }

    /// Delete a key.
    ///
    /// Returns `Ok(true)` if the key existed. P3 may leave underfull leaves; it
    /// must not leave dangling sibling pointers or unreachable live keys.
    ///
    /// Edge cases: deleting the only key leaves an empty root leaf (valid);
    /// deleting a missing key is `Ok(false)` and must not modify pages.
    pub fn delete(&mut self, key: &[u8]) -> Result<bool> {
        todo!("P3: locate leaf, remove entry, fix links if needed")
    }

    /// Range scan in ascending key order: `start` inclusive, `end` exclusive.
    ///
    /// Both bounds optional. Walk from the leaf containing `start` (or the
    /// leftmost leaf) and follow `next` pointers until `end` is reached.
    /// `start >= end` yields an empty vec.
    ///
    /// Edge case: a scan that starts before the first key must begin at the
    /// leftmost leaf, not the root's left child (they coincide today, but do not
    /// encode that assumption).
    pub fn scan(&mut self, start: Option<&[u8]>, end: Option<&[u8]>) -> Result<Vec<(Key, Value)>> {
        todo!("P3: seek to start leaf, iterate leaves, stop at end")
    }

    /// Number of live keys in the tree.
    ///
    /// Keep this O(1) if you can (counter in the header updated on insert and
    /// delete). If you choose O(n), say so here; the tests only care about
    /// correctness.
    pub fn key_count(&mut self) -> Result<u64> {
        todo!("P3: read the header counter, or count leaves if you chose O(n)")
    }

    /// Force dirty pages to disk and fsync.
    ///
    /// Delegates to [`BufferPool::flush_all`]. Tests call this before reopening
    /// the same file, so the tree must remain valid after a flush without any
    /// further writes.
    pub fn flush(&mut self) -> Result<()> {
        todo!("P3: self.pool.flush_all()")
    }
}
