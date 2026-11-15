# Learning log

One entry per working session. Short and honest beats polished and vague. The
goal is to be able to explain, months later, *why* the code looks the way it
does.

Rules:

- Write the entry **after** the tests run, even if they are red.
- Quote the actual error message. Paraphrases lose the lesson.
- "What I learned" must be about databases or Rust, not about the weather.
- Link the commit hash when you commit.

---

## 2026-11-15 — P1: MemTable, or why nine green tests lie

**Phase:** P1

**What I built**
- `src/memtable.rs`: `MemTable` over `BTreeMap<Vec<u8>, Vec<u8>>` — `new`, `len`,
  `is_empty`, `get`, `put`, `delete`, `scan`, and `Default`. `put` is an upsert,
  `delete` reports whether the key existed, and keys compare as unsigned bytes.
- `scan` builds `Bound::Included` / `Bound::Excluded` bounds for the documented
  half-open contract and clones rows out, so no iterator borrow escapes.

**What broke**
- A reversed range in `scan` panicked instead of returning an empty `Vec`.
  - Error: `thread 'main' panicked at ...: range start is greater than range end in BTreeMap`
  - Cause: `BTreeMap::range` asserts `start <= end` rather than treating a
    reversed range as empty. My contract says `start >= end` must be an empty
    result, so the wrapper has to enforce it before calling `range`.
  - Fix: guard `if s >= e { return Ok(Vec::new()) }` before constructing the
    bounds. It covers both the reversed case (which would panic) and the equal
    case (which is legal for `BTreeMap` but still empty by contract).
- The editor auto-imported an SQL type into `memtable.rs` while I was typing.
  - Error:
    ```
    error: unused import: `crate::sql::Statement::Insert`
      = note: `-D unused-imports` implied by `-D warnings`
    ```
  - Cause: an auto-import from the unbuilt P6 module. Nothing in P1 uses
    `Statement`, so the import was noise from the editor, not from the code.
  - Fix: delete the import. CI runs `cargo clippy --all-targets -- -D warnings`,
    so this was a build failure, not a suggestion.
- `cargo test --test phase1_memtable` reported `9 passed; 0 failed` while
  `impl Default for MemTable` was still `todo!()`.
  - Cause: none of the nine provided tests call `default()`. They all start at
    `MemTable::new()`, so the `todo!()` was never executed. `todo!()` compiles;
    it only panics when called.
  - Fix: added a `default_is_empty` unit test in `memtable.rs`. Now `Default` has
    at least one caller in the test suite.

**What I learned**
- **Green tests only prove the paths they execute.** Nine passing acceptance
  tests said nothing about `Default` because none of them touched it. The
  provided suite is a floor, not a completeness proof. When an item has no
  test naming it, assume it is untested no matter what the pass count says.
- **`BTreeMap::range` is not a total function.** Reversed bounds panic instead
  of yielding an empty iterator. Any wrapper that promises "empty on
  `start >= end`" must enforce that itself; forwarding a bad range to `std`
  trades a documented empty result for a panic.
- **A crate-level allow hides more than the scaffold.** `#![allow(dead_code,
  unused_variables)]` in `lib.rs` covered every module, including the implemented
  `memtable`, so dead or unused code there could never be reported. Moving the
  allow into each unbuilt module (`index`, `sql`, `storage`, `txn`, `wal`) keeps
  the `todo!()` stubs quiet while putting `memtable` back under the lints.
  Scope allows to the code that needs them, and delete each one as its phase
  lands.
- **Clippy with `-D warnings` is a cheap layering check.** The unused-import
  error was the only signal that the editor had reached across a module
  boundary. Compiler lints caught what review by eye did not.
- One subtlety worth keeping: the reversed range did not panic on an empty map,
  because there was no root to search. The bug would have hidden in an
  empty-table test and only surfaced once the table had data — the exact case
  the acceptance test creates.

**Next**
- P2: the pager. Open questions: page-0 layout, where fsync belongs, how to
  handle short reads/writes, and how to inject crash-like failures with `std`
  only and no `unsafe`.
- Keep the `dead_code` allow local to each stub and delete the P2 one once the
  pager is real.

**Commits:** `36587b0` (P1 implementation), `dd1df3c` (move the allow into the
unbuilt modules)

<!--
Copy this block for each entry and delete the comment markers.

## YYYY-MM-DD — short title

**Phase:** P?

**What I built**
-

**What broke**
-
- Error:
- Cause:
- Fix:

**What I learned**
-

**Next**
-

**Commit:** `0000000`
-->

## Template

**Phase:** P?

**What I built**
-

**What broke**
-

**What I learned**
-

**Next**
-
