# tinydb-rs

An embedded database engine in Rust, built from scratch as a learning project:
pages, a B+tree, a write-ahead log, a buffer pool, a tiny SQL front end, and
transactions. No SQL parser crates, no storage engines, no `unsafe`.

**Status: scaffold.** Every function body is `todo!("P<n>: ...")`. The
integration tests are written against the finished API and **fail on purpose**.
`cargo test` being red is the expected state until Phase 1 lands.

## Why

Most "build your own database" resources either hand you a working toy or gloss
over the parts that make databases hard: page layout, split rules, durability
ordering, and isolation. This repo is the skeleton plus the specification —
tests, invariants, and an ordered plan — so the implementation is the learning.

## Quick start

```bash
cargo build            # compiles: all bodies are todo!()
cargo test             # fails by design (todo! panics)
cargo run --bin tinydb # REPL stub: banner, .help, .quit
```

Then open [`src/memtable.rs`](src/memtable.rs) and start Phase 1.

> Windows note: if `cargo` is not found, add `%USERPROFILE%\.cargo\bin` to
> `PATH` (or use the full path) before running the commands above.

## Layout

```text
tinydb-rs/
├── Cargo.toml              # edition 2021, zero runtime deps
├── src/
│   ├── lib.rs              # crate docs, module tree, lint policy
│   ├── error.rs            # one Error enum + Result alias        (P1)
│   ├── memtable.rs         # sorted in-memory KV store            (P1)
│   ├── storage/
│   │   ├── pager.rs        # 4096-byte pages, read/write by id    (P2)
│   │   └── buffer_pool.rs  # frame cache, pins, LRU eviction      (P5)
│   ├── index/
│   │   └── btree.rs        # B+tree over pages, splits, leaf links(P3)
│   ├── wal/
│   │   └── mod.rs          # redo log + crash recovery            (P4)
│   ├── txn/
│   │   └── mod.rs          # transactions, single-writer -> MVCC  (P7)
│   ├── sql/
│   │   ├── lexer.rs        # text -> tokens                       (P6)
│   │   ├── parser.rs       # tokens -> AST                        (P6)
│   │   ├── planner.rs      # AST -> typed Plan                    (P6)
│   │   └── executor.rs     # Plan -> rows, Database façade        (P6)
│   └── bin/
│       └── tinydb.rs       # REPL stub (I/O only)
├── tests/                  # one file per phase, all currently failing
├── docs/
│   ├── ARCHITECTURE.md     # layers, invariants, on-disk formats
│   ├── ROADMAP.md          # P1..P7 with "done when" checklists
│   └── LEARNING_LOG.md     # empty template for session notes
└── .github/workflows/ci.yml
```

## The phases

| Phase | Theme | Start in |
|---|---|---|
| P1 | In-memory KV: `get`/`put`/`delete`/`scan` | `src/memtable.rs` |
| P2 | On-disk pager, persistence across restarts | `src/storage/pager.rs` |
| P3 | B+tree with page splits | `src/index/btree.rs` |
| P4 | WAL + crash recovery | `src/wal/mod.rs` |
| P5 | Buffer pool with eviction | `src/storage/buffer_pool.rs` |
| P6 | SQL: `CREATE TABLE`, `INSERT`, `SELECT ... WHERE` | `src/sql/lexer.rs` |
| P7 | Transactions: single-writer, then MVCC | `src/txn/mod.rs` |

Full checklists live in [`docs/ROADMAP.md`](docs/ROADMAP.md).

## How the scaffold is organised

- Every engine function is `todo!("P<n>: <what to do>")` and carries a doc
  comment with the contract, invariants, and edge cases.
- Signatures and types are frozen so tests can be written first. If a signature
  is genuinely wrong, change it *and* the tests together.
- Only two pieces of real code exist, both plumbing rather than engine logic:
  the `Error` trait impls (`Display`/`source`/`From<io::Error>`) and the REPL's
  line loop.
- `#![warn(missing_docs)]` and `#![forbid(unsafe_code)]` are on. Keep them on.
- The crate-level `allow(dead_code, unused_variables)` exists only because every
  body is `todo!()`. Remove it module by module as you implement.

## Conventions

- Public fallible API: `fn foo(...) -> tinydb_rs::Result<T>`.
- Bytes are `Vec<u8>` at rest and `&[u8]` when borrowed. No `String` keys.
- Ranges are half-open: start inclusive, end exclusive.
- Corruption is reported, never silently repaired.
- Durability order: WAL record fsynced **before** the data page, data page
  fsynced **before** the WAL is truncated.
- No runtime dependencies. Dev-dependencies are limited to `tempfile` and
  `proptest`.

## Tests

```bash
cargo test --test phase1_memtable   # one phase
cargo test                          # everything (currently fails everywhere)
```

`tests/phase3_btree.rs` contains a proptest that compares the tree against
`std::collections::BTreeMap`; treat it as the specification. `tests/phase4_wal.rs`
contains the crash-recovery test: a committed page image exists only in the log,
and recovery must replay it.

## CI

`.github/workflows/ci.yml` runs `fmt --check`, `clippy -D warnings`, `build`, and
compiles all tests. The `cargo test` step is deliberately `continue-on-error`
until Phase 1 is green — flip it to blocking as soon as the first phase passes.

## Docs

- [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) — layers, data flow, page and
  WAL layouts, error model, deferred decisions.
- [`docs/ROADMAP.md`](docs/ROADMAP.md) — ordered plan with done-when checklists.
- [`docs/LEARNING_LOG.md`](docs/LEARNING_LOG.md) — template for session notes.

## License

MIT. See [LICENSE](LICENSE).
