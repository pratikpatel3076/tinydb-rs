//! # `error` — Phase 1
//!
//! One error type for the whole crate, plus the crate-wide [`Result`] alias.
//!
//! Why a single enum instead of `Box<dyn Error>`? Because the layers of a
//! database fail in very different ways (I/O, corruption, conflicts, SQL
//! syntax), and callers should be able to *match* on the failure mode without
//! downcasting. `Error` is `Debug + Display + std::error::Error`, so `?` and
//! `anyhow`-style reporting both work at the edges.
//!
//! ## Conventions
//!
//! - Errors carry enough context to debug without a stack trace: page ids,
//!   WAL offsets, transaction ids, byte positions in SQL input.
//! - `&'static str` for `detail` fields keeps the enum small; use it for a
//!   short, stable reason ("not a btree leaf"), not free-form prose.
//! - Do not add a variant until a phase needs it. An unused variant is a
//!   design smell, not foresight.

use std::fmt;
use std::io;

use crate::storage::pager::PageId;

/// Crate-wide result type. Every fallible public API returns this.
pub type Result<T> = std::result::Result<T, Error>;

/// Everything that can go wrong in tinydb-rs.
///
/// The variants are grouped by layer in the order they were introduced, which
/// mirrors `docs/ROADMAP.md`.
#[derive(Debug)]
pub enum Error {
    /// An operating-system I/O failure (open, read, write, fsync, seek).
    ///
    /// The underlying [`io::Error`] is preserved so callers can inspect
    /// `ErrorKind` (for example, to distinguish `NotFound` from `PermissionDenied`).
    Io(io::Error),

    /// A page id was outside `0..page_count`.
    ///
    /// Reading or writing past the end of the file is always a bug: the caller
    /// must call `allocate_page` first. `page_count` is included so the message
    /// can explain *how far* out of range the request was.
    PageOutOfRange {
        /// The page id that was requested.
        id: PageId,
        /// Number of pages the pager currently believes exist.
        page_count: u64,
    },

    /// A page was structurally invalid for the layer that read it (bad magic,
    /// bad checksum, impossible slot offsets, and so on).
    ///
    /// Corruption is not the same as I/O failure and must never be silently
    /// repaired; surface it so recovery code can decide what to do.
    CorruptPage {
        /// The page that failed validation.
        id: PageId,
        /// Short, stable reason, e.g. `"bad magic"` or `"slot offset out of range"`.
        detail: &'static str,
    },

    /// The buffer pool has no evictable frame: every frame is pinned.
    ///
    /// This is a resource-exhaustion signal, not corruption. Either unpin some
    /// pages or size the pool larger.
    PoolExhausted {
        /// Configured number of frames.
        capacity: usize,
    },

    /// The write-ahead log is malformed before its valid end.
    ///
    /// A bad checksum at the *tail* is a torn write and should be handled by
    /// truncation, not reported with this variant. This variant means the log
    /// is untrustworthy in the middle.
    WalCorrupt {
        /// Byte offset of the first record that failed validation.
        offset: u64,
        /// Short, stable reason, e.g. `"bad record checksum"`.
        detail: &'static str,
    },

    /// A transaction tried to do something its state does not allow (write after
    /// commit, commit twice, read from an aborted transaction, ...).
    TxnState {
        /// Transaction id involved.
        txn: u64,
        /// Short, stable reason, e.g. `"already committed"`.
        detail: &'static str,
    },

    /// A write transaction conflicted with a concurrent one.
    ///
    /// Only produced once MVCC/optimistic concurrency exists (P7 stretch). The
    /// single-writer implementation should return it when a second writer is
    /// refused, so callers can retry.
    TxnConflict {
        /// Transaction that was asked to do work.
        txn: u64,
        /// Short, stable reason, e.g. `"another writer is active"`.
        detail: &'static str,
    },

    /// SQL text, or a statement derived from it, is invalid.
    ///
    /// `offset` is a byte offset into the original SQL string (0 when the
    /// problem is semantic rather than lexical). `message` is human-facing.
    Sql {
        /// Byte offset into the input where the problem starts.
        offset: usize,
        /// Human-readable explanation, e.g. `"expected FROM, found WHERE"`.
        message: String,
    },

    /// A value does not match the declared column type, or a column referenced
    /// by a query does not exist.
    ///
    /// Kept separate from [`Error::Sql`] so the executor and tests can tell
    /// "your SQL is malformed" apart from "your data/query doesn't type-check".
    Type {
        /// What the schema (or expression) required, e.g. `"INTEGER"`.
        expected: &'static str,
        /// What was actually found, e.g. `"TEXT"`.
        found: &'static str,
    },

    /// A feature that the current phase has not built yet.
    ///
    /// Used by the REPL and by phase-gated code paths where `todo!()` would
    /// abort the process. Library stubs should still use `todo!()` so tests
    /// fail loudly.
    NotImplemented(&'static str),
}

// --- Boilerplate plumbing ---------------------------------------------------
// This is the only implemented logic in the scaffold: the `Error` trait contract
// (Display/source/From) is not engine behaviour. Everything else is `todo!()`.

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "i/o error: {err}"),
            Self::PageOutOfRange { id, page_count } => {
                write!(f, "page {id} is out of range (file has {page_count} pages)")
            }
            Self::CorruptPage { id, detail } => write!(f, "page {id} is corrupt: {detail}"),
            Self::PoolExhausted { capacity } => {
                write!(f, "buffer pool exhausted: all {capacity} frames are pinned")
            }
            Self::WalCorrupt { offset, detail } => {
                write!(f, "write-ahead log is corrupt at offset {offset}: {detail}")
            }
            Self::TxnState { txn, detail } => write!(f, "transaction {txn}: {detail}"),
            Self::TxnConflict { txn, detail } => write!(f, "transaction {txn} conflicts: {detail}"),
            Self::Sql { offset, message } => write!(f, "sql error at byte {offset}: {message}"),
            Self::Type { expected, found } => {
                write!(f, "type error: expected {expected}, found {found}")
            }
            Self::NotImplemented(what) => write!(f, "not implemented yet: {what}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}
