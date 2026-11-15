//! # Write-ahead log — Phase 4
//!
//! Durability and crash recovery for page writes. The rule is simple and
//! absolute: **a dirty page may not reach the database file until the log record
//! describing that change is durable**. Recovery then replays committed records
//! over whatever the database file happens to contain.
//!
//! ## Why full-page images?
//!
//! Physical redo with full 4096-byte after-images is wasteful but trivially
//! correct, and correctness is the lesson. Logical logging ("apply this insert
//! again") requires the tree to be deterministic and idempotent; page images
//! only require that replay order matches log order. Start with page images.
//! Compress or switch to physiological logging later if you care about bytes.
//!
//! ## Record stream (suggested encoding)
//!
//! ```text
//! +--------+--------+--------+--------+--------+---------+----------+
//! | len u32| crc u32| lsn u64| txn u64| kind u8| payload |          |
//! +--------+--------+--------+--------+--------+---------+----------+
//!   4        4        8        8        1        len-21
//! ```
//!
//! `len` covers the whole record so a reader can skip forward. `crc` covers
//! everything after it (or everything including `len`; pick one and document
//! it). Payload for `PageWrite` is `page_id u64 || 4096 bytes`.
//!
//! ## Invariants
//!
//! 1. Records are appended in LSN order; LSNs are strictly increasing and never
//!    reused within one WAL file.
//! 2. A transaction's records are contiguous and bracketed by `Begin`/`Commit`
//!    (or `Abort`).
//! 3. `append` returns only after the record is fully written to the OS. `sync`
//!    is the durability barrier (`fsync`).
//! 4. A record that fails its checksum is only tolerated at the **tail** (a
//!    torn write from a crash). Corruption before the tail is
//!    [`crate::Error::WalCorrupt`].
//! 5. Replay is idempotent: replaying the same log twice produces the same
//!    database state. Full-page after-images give you this for free.
//! 6. Uncommitted (and aborted) transactions are never replayed.
//!
//! ## Recovery order (the part people get wrong)
//!
//! ```text
//! 1. scan the WAL from the start, validating checksums
//! 2. collect the set of committed txn ids
//! 3. replay PageWrite records for committed txns, in LSN order
//! 4. fsync the database file                <-- before touching the WAL
//! 5. truncate/delete the WAL (checkpoint)    <-- only now is it safe
//! ```
//!
//! If you truncate before step 4 and crash, the changes are lost and the log no
//! longer describes them.
//!
//! ## Not required in P4
//!
//! - Group commit, log shipping, or background checkpointing.
//! - Logical (row-level) records.
//! - `Abort` doing undo work; P4's `Abort` simply means "do not replay".
//!   Undo requires before-images, which arrive with real transactions (P7).

use std::fs::File;
use std::path::{Path, PathBuf};

use crate::error::Result;
use crate::storage::pager::PageId;
use crate::txn::TxnId;

/// Log sequence number. Monotonic, assigned by [`Wal::append`].
pub type Lsn = u64;

/// One entry in the write-ahead log.
///
/// Derives `PartialEq` so tests can compare a read-back stream with what was
/// written without inspecting bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WalRecord {
    /// Start of a transaction. Replay must not apply anything until a matching
    /// `Commit` is seen.
    Begin {
        /// Transaction this record belongs to.
        txn: TxnId,
    },

    /// Full after-image of one page. The page buffer is exactly
    /// [`crate::storage::pager::PAGE_SIZE`] bytes.
    ///
    /// `page_id` may be equal to the current page count: replay must extend the
    /// file in that case (the pager was allowed to allocate before flushing).
    PageWrite {
        /// Transaction this record belongs to.
        txn: TxnId,
        /// Page being replaced.
        page_id: PageId,
        /// Complete new contents of the page.
        page: Vec<u8>,
    },

    /// Transaction committed. Only committed transactions are replayed.
    Commit {
        /// Transaction this record belongs to.
        txn: TxnId,
    },

    /// Transaction rolled back. Treated exactly like "never committed".
    Abort {
        /// Transaction this record belongs to.
        txn: TxnId,
    },
}

impl WalRecord {
    /// Transaction id carried by this record.
    ///
    /// Implemented here (and not as a trait) because every record has one, and
    /// recovery groups records by it.
    pub fn txn(&self) -> TxnId {
        todo!("P4: match on self and return the txn field")
    }

    /// LSN of the record this one describes, if it describes a page write.
    ///
    /// Returns `None` for control records. Useful for diagnostics and for
    /// deduplicating page images during replay.
    pub fn page_id(&self) -> Option<PageId> {
        todo!("P4: PageWrite => Some(page_id), otherwise None")
    }
}

/// Append-only log file.
///
/// Holds its own file handle: it is intentionally separate from the database
/// file so that a WAL write and a data-page write can never be confused.
#[derive(Debug)]
pub struct Wal {
    path: PathBuf,
    file: File,
    next_lsn: Lsn,
}

impl Wal {
    /// Create a new, empty WAL, truncating any existing file at `path`.
    ///
    /// Truncation is correct here (unlike for the database file): a new WAL
    /// means the previous one was successfully checkpointed. Must fsync the
    /// empty file so a crash does not resurrect old bytes.
    pub fn create(path: impl AsRef<Path>) -> Result<Self> {
        todo!("P4: create/truncate the file, write nothing, next_lsn = 1")
    }

    /// Open an existing WAL for appending.
    ///
    /// Scans the file to determine the next LSN and the valid end offset. A
    /// torn record at the tail must be truncated here (or ignored by `records`
    /// and reported); it must not be treated as fatal corruption. Sets
    /// `next_lsn` to `last_valid_lsn + 1`, or 1 for an empty file.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        todo!("P4: open read/write, seek to the last valid record, set next_lsn")
    }

    /// Append a record and return the LSN assigned to it.
    ///
    /// Does not fsync; call [`Wal::sync`] before acknowledging a commit to the
    /// user. The returned LSN must be strictly greater than all previous ones.
    pub fn append(&mut self, record: &WalRecord) -> Result<Lsn> {
        todo!("P4: encode with checksum, write_all at EOF, bump next_lsn")
    }

    /// Make everything appended so far durable.
    ///
    /// This is the commit barrier: `append(Commit)` followed by `sync` is what
    /// makes a transaction durable.
    pub fn sync(&mut self) -> Result<()> {
        todo!("P4: self.file.sync_all()")
    }

    /// Read back the log from the beginning.
    ///
    /// Returns records with their LSNs in file order. Stops at the first torn
    /// record at the tail; any checksum failure *before* that is
    /// [`crate::Error::WalCorrupt`]. Aborted transactions are returned too;
    /// filtering them is recovery's job.
    pub fn records(&mut self) -> Result<Vec<(Lsn, WalRecord)>> {
        todo!("P4: sequential decode with a bounds-checked reader")
    }

    /// Discard the whole log after a successful checkpoint.
    ///
    /// Only legal once every replayed page and the database file itself are
    /// durable. Implementations may truncate the file to zero bytes and reset
    /// `next_lsn` to 1 (or delete and recreate it); document which.
    pub fn truncate(&mut self) -> Result<()> {
        todo!("P4: set_len(0), seek to start, reset next_lsn")
    }

    /// Path of the log file, for diagnostics and tests.
    pub fn path(&self) -> &Path {
        todo!("P4: &self.path")
    }
}

/// Outcome of [`recover`], for logging and tests.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RecoveryReport {
    /// Page writes applied to the database file.
    pub pages_replayed: u64,
    /// Committed transactions found in the log.
    pub transactions_committed: u64,
    /// Transactions discarded because they never committed.
    pub transactions_discarded: u64,
    /// True if a partial record was found at the end of the log and ignored.
    pub torn_tail: bool,
}

/// Bring `db_path` up to date using `wal_path`.
///
/// Contract:
/// - Scans the WAL, validates checksums, and ignores a torn tail.
/// - Applies `PageWrite` records only for transactions that reached `Commit`,
///   in LSN order.
/// - Extends the database file when a replayed `page_id` is the next new page.
/// - fsyncs the database file, and only then truncates the WAL.
/// - Is idempotent: running it twice, or on a clean shutdown, is a no-op.
///
/// Edge cases:
/// - Missing WAL file: nothing to recover; return a default report. (A missing
///   WAL after a clean shutdown is normal.)
/// - Missing database file but non-empty WAL: create the file and replay into
///   it; the log is the source of truth.
/// - A `PageWrite` for a page id far beyond `page_count + 1`: treat as
///   corruption; do not leave a hole.
/// - Never delete the WAL before the fsync in step 4 succeeds.
pub fn recover(db_path: impl AsRef<Path>, wal_path: impl AsRef<Path>) -> Result<RecoveryReport> {
    todo!("P4: scan -> filter committed -> replay -> fsync -> truncate")
}
