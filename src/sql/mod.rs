//! # `sql` — Phase 6
//!
//! A deliberately small SQL front end: `CREATE TABLE`, `INSERT`, and
//! `SELECT ... WHERE`. No joins, no aggregates, no subqueries, no `UPDATE`,
//! no `DELETE`. The point is to build the pipeline, not to reimplement SQLite.
//!
//! ```text
//!   text ──► lexer ──► parser ──► planner ──► executor ──► rows
//!            tokens     AST        Plan        effects
//! ```
//!
//! Each stage has its own file and its own failure mode:
//!
//! | stage | input | output | fails on |
//! |-------|-------|--------|----------|
//! | [`lexer`] | `&str` | `Vec<Token>` | illegal character, unterminated string |
//! | [`parser`] | tokens | [`parser::Statement`] | grammar violations |
//! | [`planner`] | statement + catalog | [`planner::Plan`] | unknown table/column, type mismatch |
//! | [`executor`] | plan | [`executor::QueryResult`] | storage errors, constraints |
//!
//! Keeping the planner separate from the parser is the main lesson: parsing is
//! syntax, planning is semantics. Do not let the parser resolve table names.
//!
//! ## Conventions
//!
//! - Keywords and identifiers are case-insensitive; the lexer lowercases both.
//!   String literals preserve their case.
//! - Statements are separated by `;`; a trailing `;` is optional.
//! - `WHERE` supports `=`, `!=`/`<>`, `<`, `<=`, `>`, `>=`, `AND`, `OR`, and
//!   parentheses around boolean expressions. No `NOT` in P6 unless you want it.
//! - Errors carry a byte offset into the original SQL text.
#![allow(dead_code, unused_variables)]

pub mod executor;
pub mod lexer;
pub mod parser;
pub mod planner;

pub use executor::{Database, QueryResult};
pub use parser::{DataType, Expr, SelectItem, Statement, Value};
pub use planner::{plan, Plan, TableSchema};
