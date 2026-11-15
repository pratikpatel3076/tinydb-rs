//! # Planner — Phase 6
//!
//! Turns a parsed [`Statement`] into an executable [`Plan`], resolving names and
//! types against a catalog. This is where "syntax" becomes "semantics".
//!
//! ## What the planner does
//!
//! 1. Looks up the target table in the catalog.
//! 2. Expands `SELECT *` into the table's columns, in declaration order.
//! 3. Rejects unknown columns in projections and in `WHERE`.
//! 4. For `INSERT`: maps an explicit column list into table order, fills
//!    omitted nullable columns with `NULL`, rejects omitted `NOT NULL` columns,
//!    and type-checks every literal.
//! 5. Folds literal-only expressions (`WHERE 1 = 1` → true) so the executor
//!    never evaluates a constant tree per row.
//!
//! ## What the planner must NOT do
//!
//! - Read or write any rows.
//! - Choose an index. P6 is a full sequential scan; index selection is a later
//!   exercise once the B+tree is stable.
//! - Mutate the catalog.
//!
//! ## Errors
//!
//! All semantic failures use [`crate::Error::Sql`] (unknown table, unknown
//! column, wrong arity) or [`crate::Error::Type`] (literal not assignable to the
//! column type). The planner is the right place for both; the executor should
//! only fail for storage reasons or constraint violations.

use std::collections::HashMap;

use crate::error::Result;

use super::parser::{DataType, Expr, Statement};

/// Immutable snapshot of the schema, keyed by lowercased table name.
///
/// For P6 an in-memory `HashMap` is enough. When the catalog moves onto pages,
/// this alias becomes a `Catalog` struct with the same lookup methods.
pub type Catalog = HashMap<String, TableSchema>;

/// A table's declared shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableSchema {
    /// Lowercased table name.
    pub name: String,
    /// Columns in declaration order. This order is the row order.
    pub columns: Vec<ColumnSchema>,
}

/// One column of a table, resolved and numbered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnSchema {
    /// Lowercased column name.
    pub name: String,
    /// Declared type.
    pub data_type: DataType,
    /// Whether `NULL` is allowed.
    pub nullable: bool,
    /// Zero-based position in the row.
    pub ordinal: usize,
}

/// A statement after name resolution and type checking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Plan {
    /// Register a new table in the catalog.
    CreateTable(TableSchema),
    /// Append already-typed rows to a table.
    Insert {
        /// Target table.
        table: String,
        /// One row per value tuple, in table column order.
        rows: Vec<Vec<super::parser::Value>>,
    },
    /// Read every row of a table, filter, then project.
    ///
    /// Always a full scan in P6; there is no `IndexScan` variant yet.
    SeqScan {
        /// Table to read.
        table: String,
        /// Columns to return, in output order. Never empty.
        columns: Vec<String>,
        /// Optional filter, already resolved against the schema.
        filter: Option<Expr>,
    },
}

/// Plan a single statement against `catalog`.
///
/// `catalog` is read-only. `CREATE TABLE` plans without consulting it, but must
/// check for a duplicate name (planner or executor; pick one and document it —
/// this scaffold expects the planner to reject duplicates).
///
/// Edge cases:
/// - `SELECT * FROM t` where `t` has zero columns is impossible: `CREATE TABLE`
///   requires at least one column, so reject empty schemas at plan time.
/// - `INSERT INTO t VALUES (NULL)` into a `NOT NULL` column: [`crate::Error::Type`].
/// - `SELECT missing FROM t`: [`crate::Error::Sql`] naming the column and offset.
pub fn plan(statement: &Statement, catalog: &Catalog) -> Result<Plan> {
    todo!("P6: match on the statement, resolve against catalog, return a typed Plan")
}
