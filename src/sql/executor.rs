//! # Executor — Phase 6
//!
//! Runs a [`super::planner::Plan`] and returns rows or an effect count. This is
//! the only SQL stage that touches storage, and the only one allowed to fail for
//! non-SQL reasons.
//!
//! ## P6 storage strategy (deliberately embarrassing)
//!
//! Keep rows in a `Vec<Vec<Value>>` per table and scan linearly. That is enough
//! to build and test `CREATE TABLE`/`INSERT`/`SELECT ... WHERE`, and it keeps
//! SQL progress independent from B+tree maturity. Swapping the heap for a
//! B+tree-backed table is the natural P6 stretch goal; the public API below is
//! designed so only this module changes.
//!
//! ```text
//!   sql text
//!      |
//!      v
//!   lexer -> parser -> planner -----> Plan
//!                                      |
//!                                      v
//!                              +---------------+
//!                              |   Database    |
//!                              |  catalog      |
//!                              |  rows/tables  |
//!                              +---------------+
//! ```
//!
//! ## Semantics
//!
//! - Statements execute in source order. A failure aborts the whole
//!   [`Database::execute`] call; earlier statements in the same call are **not**
//!   rolled back (P6 has no transactions). Document this, and mention that P7 is
//!   where atomic multi-statement execution arrives.
//! - `INSERT` without a column list requires exactly one value per column, in
//!   declaration order.
//! - `SELECT` preserves insertion order of rows (no `ORDER BY` in P6) and
//!   projection order from the statement.
//! - `WHERE` uses boolean logic with `NULL` treated as "not true", so a `NULL`
//!   comparison never matches. State this explicitly in the module docs and the
//!   README; it is a common source of surprise.
//! - Empty table: `SELECT` returns `Rows { columns, rows: [] }` with the
//!   projected column names, not an error.
//! - `Execute` of an empty program returns an empty result vec, not an error.
//!
//! ## Edge cases to get right
//!
//! - `INSERT` into a column list in a different order than the declaration
//!   (`INSERT INTO t (b, a) VALUES (1, 2)`).
//! - Integer/boolean coercion: decide whether `WHERE flag = 1` is legal. If not,
//!   return [`crate::Error::Type`] rather than silently comparing.
//! - Duplicate table name: error, do not overwrite the existing table.
//! - Very large result sets: no streaming in P6; that is a known limitation.
//! - An error mid-way through a multi-row `INSERT`: decide whether earlier rows
//!   in the same statement are visible. The clean answer is "validate all rows,
//!   then append", which is also what a transaction would do.
#![allow(dead_code, unused_variables)]

use std::collections::HashMap;
use std::path::PathBuf;

use crate::error::Result;

use super::parser::Value;
use super::planner::{Catalog, TableSchema};

/// Result of one executed statement.
///
/// Derives `PartialEq`/`Eq` so tests can compare whole results instead of
/// unwrapping field by field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryResult {
    /// `CREATE TABLE` succeeded.
    CreatedTable {
        /// Name of the created table.
        name: String,
    },
    /// `INSERT` succeeded.
    Inserted {
        /// Number of rows appended.
        count: usize,
    },
    /// `SELECT` succeeded.
    Rows {
        /// Column names in projection order.
        columns: Vec<String>,
        /// Row-major values, each inner vec matching `columns`.
        rows: Vec<Vec<Value>>,
    },
}

/// A single-process database handle: catalog plus (P6) in-memory rows.
///
/// Not `Sync`. One handle per database file at a time; the pager does not
/// coordinate concurrent processes.
#[derive(Debug)]
pub struct Database {
    path: Option<PathBuf>,
    catalog: Catalog,
    rows: HashMap<String, Vec<Vec<Value>>>,
    // TODO(owner): replace `rows` with B+tree-backed tables once P3 is stable.
}

impl Database {
    /// Open (or create) a persistent database at `path`.
    ///
    /// P6 may return [`crate::Error::NotImplemented`] here while the executor is
    /// still heap-only; wiring the catalog and tables onto pages is the stretch
    /// goal. If you do implement it, the catalog must be recoverable from disk
    /// and `open` must be idempotent.
    pub fn open(path: impl Into<PathBuf>) -> Result<Self> {
        todo!("P6: open the pager/btree, load the catalog, then read tables on demand")
    }

    /// Create an empty, in-memory database. Nothing is written to disk.
    ///
    /// This is the P6 target: it makes the SQL tests fast and hermetic.
    pub fn open_in_memory() -> Result<Self> {
        todo!("P6: initialise an in-memory Database with empty catalog and table maps")
    }

    /// Parse, plan, and execute every statement in `sql`, in order.
    ///
    /// Returns one [`QueryResult`] per statement (an empty program yields an
    /// empty vec). On the first error, returns immediately; prior statements in
    /// the same call have already taken effect (no implicit transaction in P6).
    ///
    /// Errors: lexer/parser/planner errors are SQL errors; storage failures come
    /// from the table implementation; duplicate table and type errors come from
    /// the planner.
    pub fn execute(&mut self, sql: &str) -> Result<Vec<QueryResult>> {
        todo!("P6: lexer::tokenize -> parser::parse -> planner::plan -> execute_plan")
    }

    /// Look up a table's schema.
    ///
    /// `Ok(None)` for an unknown table; this is a query, not an error.
    pub fn schema(&self, table: &str) -> Result<Option<TableSchema>> {
        todo!("P6: self.catalog.get(&table.to_ascii_lowercase()).cloned()")
    }

    /// Names of all tables, sorted for deterministic output in tests and the
    /// REPL. Empty when the catalog is empty.
    pub fn table_names(&self) -> Result<Vec<String>> {
        todo!("P6: collect catalog keys and sort")
    }

    /// Path this database was opened from, if any. In-memory databases return
    /// `None`.
    pub fn path(&self) -> Option<&std::path::Path> {
        todo!("P6: self.path.as_deref()")
    }
}
