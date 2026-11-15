# Roadmap

Seven phases, each one a working milestone. Do them in order: every phase except
P1 assumes the previous one is green.

## How to use this file

1. Pick the lowest phase with unchecked boxes.
2. Read the module header comment for invariants and edge cases.
3. Implement the `todo!("P<n>: ...")` bodies listed under **Build**.
4. Tick the boxes only when the phase's tests pass *and* `cargo clippy -- -D
   warnings` is clean.
5. Remove the crate-level `allow(dead_code, unused_variables)` once no module in
   that phase still needs it.
6. Add a `docs/LEARNING_LOG.md` entry.
7. One commit per phase (or per logical slice within a phase). Do not push.

Status legend: `[ ]` not started, `[x]` done. Everything is currently `[ ]`.

| Phase | Theme | Primary modules | Test file |
|---|---|---|---|
| P1 | In-memory key/value store | `src/memtable.rs` | `tests/phase1_memtable.rs` |
| P2 | On-disk pager | `src/storage/pager.rs` | `tests/phase2_pager.rs` |
| P3 | B+tree with splits | `src/index/btree.rs` | `tests/phase3_btree.rs` |
| P4 | WAL + crash recovery | `src/wal/mod.rs` | `tests/phase4_wal.rs` |
| P5 | Buffer pool + eviction | `src/storage/buffer_pool.rs` | `tests/phase5_buffer_pool.rs` |
| P6 | SQL subset | `src/sql/*.rs` | `tests/phase6_sql.rs` |
| P7 | Transactions | `src/txn/mod.rs` | `tests/phase7_txn.rs` |

> Note: P3's tests construct a `BufferPool` (P5) because the btree must go through
> the cache. If P5 is not built yet, add a temporary pass-through pool that holds
> one frame per page and never evicts, then replace it in P5. Do not let the
> btree call the pager directly — that shortcut is hard to undo.

---

## P1 — In-memory key/value store

**Goal.** A sorted map with a database-shaped API. No files, no pages, no
durability.

**Build.** Every `todo!()` in `src/memtable.rs`:

- [ ] `MemTable::new` / `Default`
- [ ] `put` as upsert
- [ ] `get` returns `Option<Value>`
- [ ] `delete` returns whether the key existed
- [ ] `scan(start, end)` with start-inclusive/end-exclusive bounds
- [ ] `len` / `is_empty`

**Done when**

- [ ] `cargo test --test phase1_memtable` passes all 9 tests.
- [ ] Keys are ordered by unsigned byte value (the `[0x7F] < [0x80] < [0xFF]` test passes).
- [ ] Overwrites keep exactly one entry; `len` does not grow.
- [ ] Empty keys and empty values are stored, and an empty value is distinguishable from a missing key.
- [ ] `scan(Some(x), Some(x))` and inverted bounds return an empty vec, not an error.
- [ ] `cargo clippy --all-targets -- -D warnings` is clean.
- [ ] Nothing in this phase touches `std::fs`.

**Pitfalls.** Using a `HashMap` and sorting on every scan works, but the `Vec`
scan must still be stable and correct; `BTreeMap` is the intended implementation.

---

## P2 — On-disk pager

**Goal.** Persist an array of fixed 4096-byte pages, addressable by id, across
process restarts.

**Build.** Every `todo!()` in `src/storage/pager.rs`:

- [ ] `Pager::create` (fail if the file exists)
- [ ] `Pager::open` (derive page count from length)
- [ ] `page_count`
- [ ] `read_page` with range check
- [ ] `write_page` with range check
- [ ] `allocate_page` (append zero-filled)
- [ ] `flush` (`sync_all`)
- [ ] `path`
- [ ] `Page::zeroed` / `as_bytes` / `as_bytes_mut`

**Done when**

- [ ] `cargo test --test phase2_pager` passes all 9 tests.
- [ ] Page ids are sequential from 0 and never reused.
- [ ] Reading or writing `id == page_count` returns `Error::PageOutOfRange`, not EOF.
- [ ] `flush` then drop then `open` returns identical bytes.
- [ ] `create` on an existing path fails and leaves the file intact.
- [ ] A file whose length is not a multiple of 4096 is rejected on `open` (add a test for this; it is the most likely real-world corruption).
- [ ] `cargo clippy --all-targets -- -D warnings` is clean.

**Pitfalls.** `seek` + `write_all` must not assume the cursor is where you left
it. On Windows, `sync_all` maps to `FlushFileBuffers`; still call it. Do not zero
the file on `open`.

---

## P3 — B+tree with page splits

**Goal.** An ordered KV map stored in pages, with correct splits and range scans.

**Build.** Every `todo!()` in `src/index/btree.rs`:

- [ ] Header page (magic, version, root, height, key count)
- [ ] Node encoding: internal separators and leaf entries
- [ ] `BTree::create` / `open`
- [ ] `insert` with leaf split + parent propagation + root split
- [ ] `get` with binary search at each level
- [ ] `scan` by walking sibling leaf links
- [ ] `delete` (rebalancing optional for P3)
- [ ] `key_count`
- [ ] `flush`

**Done when**

- [ ] `cargo test --test phase3_btree` passes all example tests.
- [ ] The proptest `btree_matches_std_btreemap` passes (48 cases × up to 150 ops).
- [ ] 1 000 ascending inserts succeed; every key is retrievable; scan is ordered.
- [ ] ~910-byte values force many splits and still roundtrip.
- [ ] Splitting the root increases height by one and keeps every leaf at the same depth.
- [ ] `flush` + drop + `open` preserves all entries.
- [ ] A value that cannot fit in an empty leaf returns an error (add a test).
- [ ] Inserting an existing key overwrites and does not change `key_count`.
- [ ] `cargo clippy --all-targets -- -D warnings` is clean.

**Pitfalls.** Pick one separator convention and stick to it. Search and scan must
agree on it. Off-by-one in leaf capacity is the classic split bug: compute usable
space from the real header size, not `PAGE_SIZE`.

---

## P4 — WAL + crash recovery

**Goal.** Survive a crash between "commit acknowledged" and "data page written".

**Build.** Every `todo!()` in `src/wal/mod.rs`:

- [ ] Record framing with length + checksum + LSN + txn + kind
- [ ] `Wal::create` / `open` (scan to valid end, set next LSN)
- [ ] `append` (write to OS, return LSN)
- [ ] `sync` (fsync — the commit barrier)
- [ ] `records` (decode, stop at torn tail)
- [ ] `truncate` (post-checkpoint reset)
- [ ] `WalRecord::txn` / `page_id`
- [ ] `recover` (committed-only replay, extend file, fsync, then truncate)

**Done when**

- [ ] `cargo test --test phase4_wal` passes all 8 tests, including `crash_recovery_replays_committed_page_writes`.
- [ ] LSNs are strictly increasing, including after `Wal::open`.
- [ ] Uncommitted transactions are not replayed.
- [ ] A garbage suffix on the log sets `torn_tail = true` and still replays the valid prefix.
- [ ] `recover` twice in a row is a no-op the second time.
- [ ] A missing WAL is a no-op, and a missing data file is recreated from the log (add a test).
- [ ] `recover` fsyncs the data file **before** truncating the WAL (assert by ordering in code review; a test can `Drop` a wrapper file handle to observe).
- [ ] `cargo clippy --all-targets -- -D warnings` is clean.

**Pitfalls.** Hand-rolled CRC must cover `kind` and payload. Keep the scanner
bounds-checked; a malformed length must never panic. Do not treat a mid-file
checksum failure as a torn tail.

---

## P5 — Buffer pool with eviction

**Goal.** Stop paying one syscall per page access, without serving stale pages.

**Build.** Every `todo!()` in `src/storage/buffer_pool.rs`:

- [ ] Frame table + page→frame map + pin counts
- [ ] LRU (or CLOCK) ordering
- [ ] `new` (reject capacity 0)
- [ ] `fetch` with hit/miss paths
- [ ] `allocate`
- [ ] `unpin`
- [ ] `flush_page` / `flush_all`
- [ ] `capacity`, `page_count`, `stats`

**Done when**

- [ ] `cargo test --test phase5_buffer_pool` passes all 6 tests.
- [ ] A second `fetch` of a resident page is a hit and does not touch the pager.
- [ ] With capacity 2 and both pages unpinned, allocating a third evicts the least recently used one and writes it back if dirty.
- [ ] With capacity 1 and the only frame pinned, the next `allocate` returns `Error::PoolExhausted`.
- [ ] `flush_all` persists pages allocated but never written individually.
- [ ] `page_count` includes buffered allocations.
- [ ] A `Pager` re-opened after `flush_all` sees the same bytes.
- [ ] `cargo clippy --all-targets -- -D warnings` is clean.

**Pitfalls.** Eviction while holding a `&mut Page` (borrow checker will stop
you — good). Forgetting to mark an allocated page dirty. `unpin` underflow.

---

## P6 — SQL subset

**Goal.** `CREATE TABLE`, `INSERT`, and `SELECT ... WHERE`, end to end.

**Build.** Every `todo!()` in `src/sql/`:

- [ ] `lexer`: identifiers/keywords/ints/strings/operators, byte offsets, comments
- [ ] `parser`: grammar above, multi-statement programs, recovery-free errors
- [ ] `planner`: catalog lookup, `*` expansion, insert column mapping, type checks, literal folding
- [ ] `executor`: `Database` catalog + heap tables, filter/projection, result types
- [ ] `Database::execute` returns one `QueryResult` per statement

**Done when**

- [ ] `cargo test --test phase6_sql` passes all 13 tests.
- [ ] `SELECT *` expands in declaration order, and rows come back in insertion order.
- [ ] An empty table returns `Rows { columns, rows: [] }`, not an error.
- [ ] Unknown table/column produces `Error::Sql`; a wrong literal type produces `Error::Type`.
- [ ] Duplicate `CREATE TABLE` fails without replacing the table.
- [ ] `INSERT INTO t (b, a) VALUES (1, 2)` maps into declaration order (add a test).
- [ ] The REPL executes SQL through `Database` (stretch), or clearly says it is not wired up.
- [ ] `cargo clippy --all-targets -- -D warnings` is clean.

**Pitfalls.** `NULL` three-valued logic. Trailing commas. Case folding. Deciding
whether `INSERT` validates all rows before appending (do validate first).

---

## P7 — Transactions

**Goal.** All-or-nothing writes with a clear isolation story.

Stage 1 (required): single writer.

- [ ] `TransactionManager::new` with `IsolationLevel::SingleWriter`
- [ ] `begin` rejects a second concurrent writer with `Error::TxnConflict`
- [ ] `put`/`delete` buffered until commit; own writes visible to the txn
- [ ] `commit` publishes atomically; `abort` discards everything
- [ ] `active_txn` reflects the live writer; failed `begin` does not leak a slot
- [ ] WAL ordering: `Commit` record + `sync` before publishing (once P4 is wired)

Stage 2 (stretch): MVCC.

- [ ] `IsolationLevel::SnapshotIsolation` with a per-transaction snapshot
- [ ] Readers never block writers and vice versa
- [ ] First-committer-wins or conflict detection at commit, documented
- [ ] `abort`/`commit` garbage-collects unreachable versions

**Done when**

- [ ] `cargo test --test phase7_txn` passes all 7 tests (stage 1 tests all green).
- [ ] `snapshot_isolation_does_not_see_later_commits` passes (stage 2).
- [ ] A transaction sees its own uncommitted write and a later transaction does not.
- [ ] Deleting a missing key returns `false` and records nothing.
- [ ] Two puts to the same key in one transaction commit the last value only.
- [ ] `cargo clippy --all-targets -- -D warnings` is clean.

**Pitfalls.** Publishing before the WAL fsync. Aborting a transaction that
already wrote pages directly to the store (it must not). Leaking the active
writer slot on error paths.

---

## Cross-cutting checklist (every phase)

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo build --all-targets`
- [ ] `cargo test` (all green from the end of the phase onward)
- [ ] No new runtime dependencies in `[dependencies]`
- [ ] Public items documented; `#![warn(missing_docs)]` still on
- [ ] Module header updated if an invariant or on-disk format changed
- [ ] `docs/ARCHITECTURE.md` updated if a layer boundary moved
- [ ] `docs/LEARNING_LOG.md` entry added
- [ ] One commit, message prefixed with the phase (`p3: implement leaf splits`)

## After P7 (ideas, not committed)

- Index selection and `EXPLAIN` (use the btree for `WHERE pk = ?`).
- `UPDATE` / `DELETE` / `ORDER BY` / `LIMIT`.
- Catalog persistence on pages.
- A real page checksum and `PRAGMA integrity_check`-style verifier.
- Concurrent readers behind `RwLock`, then a thread-per-connection front end.
- Crash fuzzing: kill the process at random points and assert recovery.
