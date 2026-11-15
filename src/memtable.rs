//! # `MemTable` — Phase 1
//!
//! An ordered, in-memory key/value map. This is the warm-up phase: no files, no
//! pages, no WAL — just a sorted map with a database-shaped API.
//!
//! Why start here? Because everything above this line has the same shape:
//! `put`, `get`, `delete`, range `scan`. Getting the semantics right (ordering,
//! bounds, overwrite behaviour, what `delete` returns) before adding durability
//! keeps P2–P7 from rediscovering the same edge cases.
//!
//! ## Semantics (the contract all later layers must preserve)
//!
//! - **Keys are opaque byte strings** ordered lexicographically by unsigned
//!   byte value (`[1, 0] < [1, 1] < [2]`; `[0xFF]` sorts last, unlike signed
//!   `i8` comparison).
//! - `put` is an upsert: inserting an existing key replaces the old value and
//!   does not create a duplicate entry.
//! - `delete` returns whether the key existed before the call. Deleting a
//!   missing key is not an error.
//! - `scan(start, end)` returns entries in ascending key order with
//!   `start` **inclusive** and `end` **exclusive**, like `BTreeMap::range`.
//!   `None` means unbounded on that side. `start >= end` yields an empty vec,
//!   not an error.
//! - Empty keys and empty values are legal. `get` of a missing key is `None`,
//!   not an error.
//!
//! ## Why does everything return `Result`?
//!
//! P1 cannot fail. P2+ can (I/O, corruption). Freezing the fallible signature
//! now means the B+tree can be swapped in behind the same API without touching
//! call sites.
//!
//! ## Not required in P1
//!
//! - Tombstones, versioning, or visibility rules (P7).
//! - Persistence (P2/P4).
//! - Concurrency (P7). `&mut self` is the whole story.
//! - Memory accounting or a size limit.

use std::collections::BTreeMap;

use crate::error::Result;

/// Opaque key bytes. Ordering is unsigned-lexicographic.
pub type Key = Vec<u8>;

/// Opaque value bytes. No encoding is imposed at this layer.
pub type Value = Vec<u8>;

/// Sorted in-memory key/value store.
///
/// A thin, intent-revealing wrapper around [`BTreeMap`]. Do not leak the map:
/// the point of the wrapper is to pin down the semantics above and to give P3
/// a migration target.
#[derive(Debug)]
pub struct MemTable {
    entries: BTreeMap<Key, Value>,
}

impl MemTable {
    /// Create an empty table.
    ///
    /// Should not allocate until the first `put` (or let `BTreeMap` do what it
    /// wants; this is not a performance-critical decision).
    pub fn new() -> Self {
        todo!("P1: create the backing BTreeMap and return Self")
    }

    /// Number of live entries.
    ///
    /// Overwrites do not increase this; deletes do decrease it.
    pub fn len(&self) -> usize {
        todo!("P1: self.entries.len()")
    }

    /// True when there are no live entries.
    ///
    /// Exists alongside [`MemTable::len`] because Clippy (rightly) expects it.
    pub fn is_empty(&self) -> bool {
        todo!("P1: self.entries.is_empty()")
    }

    /// Look up a key.
    ///
    /// `None` for a missing key. Must not mutate or insert anything: a read of a
    /// missing key must not change [`MemTable::len`].
    pub fn get(&self, key: &[u8]) -> Result<Option<Value>> {
        todo!("P1: self.entries.get(key).cloned()")
    }

    /// Insert or replace a value.
    ///
    /// Accepts anything convertible into a `Vec<u8>`, so `&[u8]`, `&str`,
    /// `Vec<u8>`, and `b"literal"` all work at call sites.
    ///
    /// Edge case: replacing an existing key must keep exactly one entry and
    /// must make the new value immediately visible to `get` and `scan`.
    pub fn put(&mut self, key: impl Into<Key>, value: impl Into<Value>) -> Result<()> {
        todo!("P1: self.entries.insert(key.into(), value.into())")
    }

    /// Remove a key.
    ///
    /// Returns `Ok(true)` if the key was present, `Ok(false)` if it was already
    /// absent. A missing key is not an error.
    pub fn delete(&mut self, key: &[u8]) -> Result<bool> {
        todo!("P1: self.entries.remove(key).is_some()")
    }

    /// Range scan in ascending key order.
    ///
    /// `start` is inclusive, `end` is exclusive. Both are optional. Results own
    /// their bytes so the caller can mutate the table afterwards.
    ///
    /// Edge cases:
    /// - `start >= end` → empty result.
    /// - `start == end` → empty result.
    /// - Bounds that do not match any key are still valid; return the keys in
    ///   between.
    /// - The scan must see the table as of the call (no iterator borrow escapes).
    pub fn scan(&self, start: Option<&[u8]>, end: Option<&[u8]>) -> Result<Vec<(Key, Value)>> {
        todo!("P1: build a half-open RangeBounds and clone the matching entries")
    }
}

impl Default for MemTable {
    /// Equivalent to [`MemTable::new`].
    fn default() -> Self {
        todo!("P1: Self::new()")
    }
}
