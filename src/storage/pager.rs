//! # `Pager` — Phase 2
//!
//! The bottom of the storage stack: a single database file viewed as an array
//! of **fixed 4096-byte pages**, addressed by `PageId`. Nothing above this
//! module knows that files, offsets, or `seek` exist.
//!
//! ## File layout you are building toward
//!
//! ```text
//! offset 0            offset 4096         offset 8192
//! +-------------------+-------------------+-------------------+
//! | page 0            | page 1            | page 2            | ...
//! | 4096 bytes        | 4096 bytes        | 4096 bytes        |
//! +-------------------+-------------------+-------------------+
//! ```
//!
//! The pager does **not** reserve page 0. Higher layers may use it for a header
//! (the B+tree will); the pager treats every page uniformly.
//!
//! ## Invariants
//!
//! 1. `PAGE_SIZE` is exactly 4096 and never changes at runtime.
//! 2. A page's byte offset is `page_id * PAGE_SIZE`; there is no per-page header
//!    at this layer.
//! 3. `page_count()` is the number of pages that are readable. `allocate_page`
//!    is the only operation that increases it.
//! 4. The file length is always a multiple of `PAGE_SIZE`. A file whose length is
//!    not a multiple of 4096 (or is zero bytes but claims pages) is corrupt;
//!    `open` should report, not repair.
//! 5. `write_page` never writes past `page_count - 1`. To add a page, allocate
//!    first; to overwrite, the page must already exist.
//! 6. After `flush` returns `Ok`, all prior writes are durable (`fsync`-ed), not
//!    just in the OS page cache.
//!
//! ## Edge cases to get right
//!
//! - Empty file: `page_count == 0`; the first `allocate_page` returns 0.
//! - Reading id `== page_count` is [`crate::Error::PageOutOfRange`], not EOF.
//! - A partial final page means the file was truncated mid-write: `open` must
//!   fail rather than zero-pad silently.
//! - `flush` on a clean pager is a no-op (and cheap).
//! - Two `Pager`s on the same path are *not* coordinated. Enforce a lock file or
//!   document single-process ownership; do not pretend otherwise.
//!
//! ## Not required in P2 (deferred)
//!
//! - Free-page list / page reuse. `allocate_page` appends forever.
//! - Sparse files or `fallocate`-style preallocation.
//! - Checksums. Those belong to the B+tree node header in P3 and to the WAL in
//!   P4, not to raw page I/O.

use std::fs::File;
use std::path::{Path, PathBuf};

use crate::error::Result;

/// Size of every page, in bytes. Fixed for the lifetime of a database file.
pub const PAGE_SIZE: usize = 4096;

/// Identifier of a page inside a database file. Page `n` starts at byte
/// `n * PAGE_SIZE`.
pub type PageId = u64;

/// A single 4096-byte page held in memory.
///
/// The bytes are boxed so that moving a `Page` moves a pointer instead of
/// copying 4 KiB onto the stack. `Page` owns its bytes; it is not a view into
/// the pager's cache.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Page {
    bytes: Box<[u8; PAGE_SIZE]>,
}

impl Page {
    /// Create an all-zero page.
    ///
    /// Needed by callers that build a page before writing it (a fresh B+tree
    /// leaf, a SQL catalog page). Must not read the file.
    ///
    /// Invariant: every byte is zero.
    pub fn zeroed() -> Self {
        todo!("P2: allocate a zero-filled Box<[u8; PAGE_SIZE]>")
    }

    /// Borrow the full page as a fixed-size byte slice.
    ///
    /// Length is always exactly [`PAGE_SIZE`]; callers index into it with
    /// their own page format. Panics are impossible here, which is the point of
    /// returning a fixed-size array reference.
    pub fn as_bytes(&self) -> &[u8; PAGE_SIZE] {
        todo!("P2: return &self.bytes")
    }

    /// Mutably borrow the full page.
    ///
    /// Any write through this slice marks the page dirty at the buffer-pool
    /// layer (P5), not here. The pager does not track dirtiness.
    pub fn as_bytes_mut(&mut self) -> &mut [u8; PAGE_SIZE] {
        todo!("P2: return &mut self.bytes")
    }
}

/// Owns the open database file and translates page ids into file offsets.
///
/// One `Pager` == one file handle == one process's view of the file. It is not
/// `Sync`; higher layers must serialize access (the buffer pool takes `&mut
/// self`, which is the intended story).
#[derive(Debug)]
pub struct Pager {
    path: PathBuf,
    file: File,
    page_count: u64,
}

impl Pager {
    /// Create a brand-new, empty database file at `path`.
    ///
    /// - Fails if the file already exists (use [`Pager::open`] for that).
    /// - Fails if the parent directory does not exist.
    /// - On success the file is zero bytes long and `page_count() == 0`.
    ///
    /// Edge cases: do not truncate an existing file, because that destroys data
    /// on a mistyped path. Do not create parent directories implicitly.
    pub fn create(path: impl AsRef<Path>) -> Result<Self> {
        todo!("P2: OpenOptions::new().read(true).write(true).create_new(true) then build Pager")
    }

    /// Open an existing database file for reading and writing.
    ///
    /// Must derive `page_count` from the file length, and must validate that
    /// the length is a multiple of [`PAGE_SIZE`]. A zero-byte file is valid and
    /// yields `page_count == 0`.
    ///
    /// Invariant: the returned pager never reports a page it could not read.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        todo!("P2: open the file, read its length, reject a partial trailing page")
    }

    /// Number of pages currently in the file.
    ///
    /// This is the *committed* size of the file, not the number of buffered
    /// writes: the pager is unbuffered in P2. The buffer pool adds its own
    /// dirty-frame accounting on top (P5).
    pub fn page_count(&self) -> u64 {
        todo!("P2: return self.page_count")
    }

    /// Read one page by id.
    ///
    /// - `id < page_count()` is required; otherwise return
    ///   [`crate::Error::PageOutOfRange`] with both the requested id and the
    ///   current count.
    /// - Reads exactly [`PAGE_SIZE`] bytes; a short read means the file changed
    ///   underneath us and is an I/O error.
    /// - Must not cache: every call hits the file. Caching is P5's job.
    pub fn read_page(&mut self, id: PageId) -> Result<Page> {
        todo!("P2: bounds-check, seek to id * PAGE_SIZE, read_exact into a zeroed Page")
    }

    /// Overwrite an existing page.
    ///
    /// - The page must already exist (`id < page_count()`), otherwise
    ///   [`crate::Error::PageOutOfRange`]. Use [`Pager::allocate_page`] for new
    ///   pages.
    /// - Writes exactly [`PAGE_SIZE`] bytes at `id * PAGE_SIZE`.
    /// - Does **not** fsync. Batch durability into [`Pager::flush`].
    ///
    /// Edge case: writing the same page twice without a flush must leave the
    /// last write visible to a subsequent `read_page` in the same process.
    pub fn write_page(&mut self, id: PageId, page: &Page) -> Result<()> {
        todo!("P2: bounds-check, seek, write_all(page.as_bytes())")
    }

    /// Append a new zero-filled page and return its id.
    ///
    /// The new file length must become `(old_count + 1) * PAGE_SIZE`; the
    /// returned id is `old_count`. Should the page be zero-filled on disk, or
    /// only on first write? Either is fine, but write it down: the B+tree
    /// assumes a freshly allocated page reads back as zeros.
    ///
    /// Edge cases:
    /// - Must not reuse ids; no free list in P2.
    /// - If the write fails halfway, the file length may be wrong. On the next
    ///   `open` that surfaces as a partial-page error; document the behaviour
    ///   you choose (truncate on open vs. report corruption). Silent repair is
    ///   not allowed.
    pub fn allocate_page(&mut self) -> Result<PageId> {
        todo!("P2: extend the file by one page, zero it, bump page_count")
    }

    /// Make every prior write durable.
    ///
    /// "Durable" means `fsync`/`FlushFileBuffers`, not `flush()` on a
    /// `BufWriter`. After this returns `Ok`, a power loss must not lose the
    /// writes. On a clean pager this should be cheap.
    pub fn flush(&mut self) -> Result<()> {
        todo!("P2: self.file.sync_all() (and sync the directory once on Unix if you care)")
    }

    /// Path this pager was opened from. Useful for error messages and tests.
    pub fn path(&self) -> &Path {
        todo!("P2: return &self.path")
    }
}
