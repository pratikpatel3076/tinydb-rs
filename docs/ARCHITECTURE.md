# Architecture

tinydb-rs is a layered embedded database. Each layer has one job, a narrow public
API, and a phase in which it is built. Higher layers must not reach around lower
ones: SQL talks to transactions, transactions talk to the btree, the btree talks
to the buffer pool, the buffer pool talks to the pager. Only the pager touches a
database file.

## Layered view

```text
        SQL text
           |
           v
+-----------------------------------------------------------------------------------+
|  sql/                                                                      P6     |
|  lexer  ->  parser  ->  planner  ->  executor                                      |
|  text       AST          typed Plan      rows / effects                            |
+------------------------------------+----------------------------------------------+
                                     | put / get / delete / scan
+------------------------------------v----------------------------------------------+
|  txn/                                                                     P7      |
|  begin / commit / abort, visibility rules, single-writer -> MVCC                  |
+------------------------------------+----------------------------------------------+
                                     | ordered KV operations
+------------------------------------v----------------------------------------------+
|  index/btree.rs  P3                        memtable.rs  P1                        |
|  ordered KV stored in pages,              ordered KV in RAM, no durability         |
|  splits, leaf links, persistence                                                   |
+------------------------------------+----------------------------------------------+
                                     | fetch / unpin / flush
+------------------------------------v----------------------------------------------+
|  storage/buffer_pool.rs                                                    P5     |
|  bounded frame cache, pins, LRU eviction, dirty writeback                          |
+------------------------------------+----------------------------------------------+
                                     | read_page / write_page / allocate_page
+------------------------------------v----------------------------------------------+
|  storage/pager.rs                                                          P2     |
|  one file = array of 4096-byte pages, addressed by PageId                          |
+------------------------------------+----------------------------------------------+
                                     |
                              database file
```

Durability is a side-channel, not a layer:

```text
   commit ──> wal/ append(PageWrite/Commit) ──> sync (fsync WAL)
                                                   |
                                            (only now ack)
                                                   |
   writeback ──> pager.write_page ─────────> flush (fsync data file)

   restart ──> wal/recover()
                 1. scan + validate checksums
                 2. keep committed txns only
                 3. replay full-page images in LSN order
                 4. fsync data file
                 5. truncate WAL
```

`memtable` (P1) and `btree` (P3) are two implementations of the same contract:
`put`/`get`/`delete`/`scan` with half-open ranges. The btree replaces the
memtable; it does not sit on top of it in P3 (an LSM-style memtable + SSTable
design is a different, valid path — out of scope here).

## Module responsibilities

| Module | Phase | Owns | Must not |
|---|---|---|---|
| `src/error.rs` | 1 | The single `Error` enum and `Result<T>` alias | Panic on user input; leak `io::Error` semantics without a variant |
| `src/memtable.rs` | 1 | Sorted in-memory KV, range semantics | Touch the filesystem, or promise durability |
| `src/storage/pager.rs` | 2 | File handle, page count, 4096-byte reads/writes, fsync | Cache pages; interpret page contents |
| `src/storage/buffer_pool.rs` | 5 | Frame cache, pin counts, eviction, dirty writeback | Interpret page contents; read/write files directly |
| `src/index/btree.rs` | 3 | Page layout for tree nodes, splits, leaf links, tree header | Know about SQL, transactions, or WAL |
| `src/wal/mod.rs` | 4 | Append-only log, checksums, replay/checkpoint | Decide transaction visibility policy (it reports, `txn` decides) |
| `src/txn/mod.rs` | 7 | Transaction lifecycle, isolation, commit ordering | Format pages, parse SQL |
| `src/sql/lexer.rs` | 6 | Tokens, byte offsets | Resolve names, evaluate expressions |
| `src/sql/parser.rs` | 6 | AST, grammar errors | Look at the catalog or storage |
| `src/sql/planner.rs` | 6 | Name/type resolution, `Plan` | Read or write rows |
| `src/sql/executor.rs` | 6 | Running a `Plan`, `Database` façade | Parse SQL, guess at schema |
| `src/bin/tinydb.rs` | 1+ | REPL I/O only | Contain engine logic |

## Data flows

### Point read (`get`)

```text
btree.get(key)
  -> buffer_pool.fetch(root_page)        // may read from disk (miss)
  -> binary-search separators
  -> buffer_pool.fetch(child_page)
  -> ... until leaf
  -> buffer_pool.unpin(each page)        // exactly once per fetch
  -> return value
```

The pager never sees a key. The btree never sees a file offset.

### Write and durability

```text
btree.insert(key, value)
  -> buffer_pool.fetch(leaf)             // pinned
  -> modify bytes in the frame           // frame marked dirty
  -> split if needed (allocate pages via pool)
  -> unpin
  -> (P4) wal.append(PageWrite after-image); wal.sync()
  -> (later) buffer_pool flush -> pager.write_page -> fsync
```

The order "log durable before data page durable" is what makes recovery work. If
you fsync the data page first, the WAL has nothing to add; if you ack the user
before the WAL fsyncs, a crash loses a committed transaction.

### Crash recovery

See `wal::recover`. The only safe order is: replay, fsync data, *then* truncate
the log. Truncating first and crashing loses both copies.

## On-disk formats (proposals, not law)

### Database file

```text
+----------------------+   page 0: btree header (owned by P3)
| page 0 (4096 bytes)  |     magic, version, page_size, root_page, leaf_level,
+----------------------+     key_count, free_page (future)
| page 1               |   internal/leaf nodes, self-describing via a node header
+----------------------+
| page 2               |
+----------------------+
| ...                  |
```

The pager imposes no page header. The btree chooses one; write it down in
`src/index/btree.rs` before implementing splits.

### WAL file

```text
record := len:u32 | crc:u32 | lsn:u64 | txn:u64 | kind:u8 | payload
payload(PageWrite) := page_id:u64 | page[4096]
```

`len` allows skipping, `crc` detects torn writes, `lsn` orders replay. Record
encoding details belong to P4; the invariant is that a reader can always tell
"valid record", "torn tail", or "corrupt middle".

## Error model

All fallible APIs return `tinydb_rs::Result<T>` = `Result<T, tinydb_rs::Error>`.

| Failure | Variant | Recoverable? |
|---|---|---|
| OS I/O | `Io` | Sometimes (retry, different path) |
| Page id past EOF | `PageOutOfRange` | Caller bug |
| Bad page/record structure | `CorruptPage`, `WalCorrupt` | No; report and stop |
| All frames pinned | `PoolExhausted` | Yes; unpin more or grow pool |
| Torn WAL tail | reported via `RecoveryReport`, not an error | Yes; truncate |
| Bad transaction state | `TxnState`, `TxnConflict` | Usually yes (retry) |
| SQL syntax/semantics | `Sql`, `Type` | Yes; fix the query |
| Phase not built | `NotImplemented` | Only from the REPL |

Rules:

- Corruption is never silently repaired.
- Errors carry ids/offsets so a message is actionable without a backtrace.
- Tests match on variants, not on message strings.

## Ownership and concurrency

- `Pager` owns one file handle. Not `Sync`.
- `BufferPool` owns the `Pager`. `&mut self` serializes access.
- `BTree` owns the `BufferPool`.
- `Database` (SQL) owns the catalog and table storage.
- The whole engine is single-process, single-threaded at the API level. A
  `Mutex<Database>` is the intended multi-threaded story once it exists; MVCC
  (P7) is about logical isolation, not about lock-free data structures.
- There is no `unsafe` (`#![forbid(unsafe_code)]`). If a later phase needs
  `mmap`, that is a deliberate, documented change to the crate attribute.

## Testing strategy

| Layer | Test style | File |
|---|---|---|
| MemTable | Example-based, edge cases | `tests/phase1_memtable.rs` |
| Pager | Roundtrip + reopen + bounds errors | `tests/phase2_pager.rs` |
| B+tree | Examples + proptest vs `BTreeMap` | `tests/phase3_btree.rs` |
| WAL | Stream roundtrip + crash/redo + torn tail | `tests/phase4_wal.rs` |
| Buffer pool | Counter-based eviction assertions | `tests/phase5_buffer_pool.rs` |
| SQL | Lexer/parser AST + end-to-end queries | `tests/phase6_sql.rs` |
| Transactions | Visibility matrix | `tests/phase7_txn.rs` |

The proptest comparison against `std::collections::BTreeMap` is the highest-value
test in the repo: it catches separator-direction mistakes, split bugs, and scan
boundary errors that example tests miss.

## Deliberately deferred decisions

Decide these when the phase needs them, and record the choice in the module
header:

- Page 0 layout and whether the catalog lives inside it (P3/P6).
- Separator convention: smallest key in the right child vs largest in the left.
- Delete rebalancing: none in P3, merge/redistribute later.
- Buffer pool replacement: strict LRU vs CLOCK (pick one, make it deterministic).
- WAL record checksum algorithm (CRC32 vs xxhash-style; `std`-only, so hand-roll
  or use a simple additive/FNV checksum).
- Catalog persistence format (pages vs a side file) in P6.
- MVCC version storage and garbage-collection policy in P7.

## Non-goals

No networking, no server mode, no replication, no query optimizer beyond filter
pushdown, no joins/aggregates, no floats, no date/time types, no `UPDATE`, no
concurrent multi-process access. Add them only by writing a new phase in
`docs/ROADMAP.md` first.
