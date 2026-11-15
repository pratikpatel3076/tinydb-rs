//! # Transactions — Phase 7
//!
//! All-or-nothing groups of reads and writes. This module is intentionally
//! staged:
//!
//! 1. **Single-writer** (the P7 target): at most one write transaction may be
//!    active; readers are still allowed to read the last committed state.
//! 2. **MVCC** (the stretch goal): writers build private versions; readers see a
//!    snapshot taken at `begin`. Conflicts are detected at commit.
//!
//! Do not skip stage 1. Getting commit/abort visibility right with one writer is
//! a day's work; MVCC is a week. The API below is designed so stage 2 can keep
//! the same signatures.
//!
//! ## Visibility rules (the contract)
//!
//! - A transaction sees its own uncommitted writes.
//! - A transaction never sees another transaction's uncommitted writes.
//! - `commit` makes all of a transaction's writes visible atomically.
//! - `abort` (and `Drop` without commit, if you implement it) discards all of
//!   them; no partial state is ever visible.
//! - A committed transaction's writes survive a crash only if the commit record
//!   was fsynced in the WAL first (P4). Durability is not this module's job, but
//!   the commit path must call the WAL, not the pager, first.
//!
//! ## What "single-writer" means precisely
//!
//! - [`TransactionManager::begin`] returns [`crate::Error::TxnConflict`] when
//!   another *write* transaction is active.
//! - Read-only transactions may or may not be modeled in stage 1; if they are,
//!   they must still be isolated from the writer's uncommitted changes.
//! - `active_txn` exists so tests (and the REPL) can assert that a failed
//!   `begin` did not leak a transaction id.
//!
//! ## Edge cases to get right
//!
//! - Commit with no writes: legal, and still consumes the transaction.
//! - Abort with no writes: legal, idempotent from the caller's perspective
//!   because the transaction is consumed.
//! - Using a transaction after commit/abort: impossible by construction
//!   (`commit`/`abort` take `self` by value), but `TxnState` is still exposed
//!   for diagnostics. Do not add `&mut Transaction` versions that would allow it.
//! - Deleting a key that does not exist inside a transaction: `Ok(false)`, no
//!   write recorded.
//! - Re-inserting a key twice in one transaction: the last write wins, and only
//!   that write should appear at commit.
//! - Aborting must not decrement the visible key count for keys that never
//!   existed.
//!
//! ## MVCC notes for the stretch goal
//!
//! Wrap each value in `(TxnId creating, TxnId deleting, value)`. A reader with
//! snapshot `s` sees the version whose creator is committed and `<= s` and whose
//! deleter is not committed before `s`. Garbage-collect versions that no live
//! snapshot can see. Read [`TransactionManager`]'s fields: a version chain per
//! key is the natural upgrade from the current `BTreeMap<Key, Value>` shape.
#![allow(dead_code, unused_variables)]

use crate::error::Result;

/// Monotonic transaction identifier. Also used by the WAL to bracket records.
pub type TxnId = u64;

/// How much isolation the manager must provide.
///
/// Start by implementing only [`IsolationLevel::SingleWriter`]. The variant is
/// here so `TransactionManager::new` does not need to change later.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsolationLevel {
    /// One writer at a time; readers see the last committed state. Cheapest to
    /// implement and the P7 target.
    SingleWriter,
    /// Snapshot isolation via multi-version concurrency control.
    SnapshotIsolation,
}

/// Lifecycle state of a transaction.
///
/// `Active` is the only state in which reads and writes are legal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TxnState {
    /// Begun and not yet committed or aborted.
    Active,
    /// Effects are visible to later transactions.
    Committed,
    /// Effects were discarded.
    Aborted,
}

/// A handle to one transaction.
///
/// Holds no data of its own beyond its identity and (for MVCC) its snapshot.
/// Consumed by [`TransactionManager::commit`] / [`TransactionManager::abort`].
#[derive(Debug)]
pub struct Transaction {
    id: TxnId,
    state: TxnState,
    // TODO(owner): snapshot id (MVCC), write buffer, or pinned key set.
}

impl Transaction {
    /// Transaction id, as assigned by `begin`.
    pub fn id(&self) -> TxnId {
        todo!("P7: return self.id")
    }

    /// Current lifecycle state. `Active` until consumed by commit/abort.
    pub fn state(&self) -> TxnState {
        todo!("P7: return self.state")
    }
}

/// Owns the committed state plus any active transaction.
///
/// The storage engine (`BTree` or `MemTable`) sits inside this manager: callers
/// never touch the store directly once transactions exist, or isolation is a
/// lie.
#[derive(Debug)]
pub struct TransactionManager {
    isolation: IsolationLevel,
    // TODO(owner): store, active txn, next txn id, snapshot bookkeeping.
}

impl TransactionManager {
    /// Create a manager over an empty store.
    ///
    /// The first id handed out should be 1 (0 is reserved for "no
    /// transaction"/"snapshot of nothing"). For `SingleWriter`, no other setup
    /// is required.
    pub fn new(isolation: IsolationLevel) -> Self {
        todo!("P7: initialise the store, next_txn_id = 1, active = None")
    }

    /// Start a transaction.
    ///
    /// - `SingleWriter`: fails with [`crate::Error::TxnConflict`] if a
    ///   transaction is already active. A failed `begin` must not allocate a
    ///   visible id or change `active_txn`.
    /// - `SnapshotIsolation`: always succeeds; the transaction records the
    ///   current commit horizon.
    ///
    /// Edge case: ids must not be reused after abort, even though they are not
    /// visible in the data.
    pub fn begin(&mut self) -> Result<Transaction> {
        todo!("P7: allocate id, set state Active, record snapshot, register as active")
    }

    /// Read a key inside a transaction.
    ///
    /// Sees the transaction's own uncommitted writes first, then the committed
    /// state visible to its snapshot. Returns `None` for a key that was deleted
    /// or never existed.
    pub fn get(&self, txn: &Transaction, key: &[u8]) -> Result<Option<Vec<u8>>> {
        todo!("P7: overlay write buffer on committed store using snapshot visibility")
    }

    /// Upsert a key inside a transaction.
    ///
    /// Buffered until commit (single-writer may still mutate an in-memory map and
    /// apply it at commit; MVCC must not mutate shared state).
    ///
    /// Invariant: no live/committed state changes before `commit` returns `Ok`.
    pub fn put(
        &mut self,
        txn: &mut Transaction,
        key: impl Into<Vec<u8>>,
        value: impl Into<Vec<u8>>,
    ) -> Result<()> {
        todo!("P7: validate state, record the write in the txn's write buffer")
    }

    /// Delete a key inside a transaction.
    ///
    /// Returns whether the key was visible to the transaction before the delete.
    /// Deleting a missing key records nothing and returns `Ok(false)`.
    pub fn delete(&mut self, txn: &mut Transaction, key: &[u8]) -> Result<bool> {
        todo!("P7: check visibility, record a tombstone, return prior existence")
    }

    /// Commit a transaction, making all of its writes visible.
    ///
    /// Order matters for durability: append `Commit` to the WAL and `sync` it,
    /// *then* publish to the store and release the write lock. If publishing
    /// fails, the manager must still be in a state where a retry or abort is
    /// possible; document the choice.
    ///
    /// Consumes the transaction; there is no way to commit twice by accident.
    pub fn commit(&mut self, txn: Transaction) -> Result<()> {
        todo!("P7: validate Active, WAL commit+sync, apply writes, clear active")
    }

    /// Abort a transaction and discard its writes.
    ///
    /// Aborting an empty or already-finished transaction is not reachable
    /// through the public API (the handle is consumed), but the state machine
    /// should still reject nonsense with [`crate::Error::TxnState`].
    pub fn abort(&mut self, txn: Transaction) -> Result<()> {
        todo!("P7: validate Active, drop write buffer, clear active")
    }

    /// Id of the currently active write transaction, if any.
    ///
    /// Used by tests to prove that a rejected `begin` did not leak a slot.
    pub fn active_txn(&self) -> Option<TxnId> {
        todo!("P7: return the active txn id")
    }

    /// Isolation level this manager was built with.
    pub fn isolation(&self) -> IsolationLevel {
        todo!("P7: return self.isolation")
    }
}
