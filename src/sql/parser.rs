//! # Parser — Phase 6
//!
//! Recursive-descent parser over the token stream. Produces the AST below and
//! nothing else: no name resolution, no type checking, no storage access.
//!
//! ## Grammar (P6)
//!
//! ```text
//! statements  := statement (';' statement)* ';'?
//! statement   := create_table | insert | select
//!
//! create_table := CREATE TABLE ident '(' column_def (',' column_def)* ')'
//! column_def   := ident data_type ('NOT' NULL)?
//! data_type    := INTEGER | TEXT | BOOLEAN
//!
//! insert      := INSERT INTO ident ('(' ident (',' ident)* ')')?
//!                VALUES '(' expr (',' expr)* ')' (',' '(' expr (',' expr)* ')')*
//!
//! select      := SELECT projection FROM ident ('WHERE' expr)?
//! projection  := '*' | ident (',' ident)*
//!
//! expr        := or_expr
//! or_expr     := and_expr (OR and_expr)*
//! and_expr    := cmp_expr (AND cmp_expr)*
//! cmp_expr    := primary (('='|'!='|'<>'|'<'|'<='|'>'|'>=') primary)?
//! primary     := integer | string | TRUE | FALSE | NULL
//!              | ident | '(' expr ')'
//! ```
//!
//! ## Conventions
//!
//! - The parser does not evaluate anything. `1 + 1` is not in the grammar;
//!   `WHERE id = 1` keeps `Expr::Literal(Integer(1))` and
//!   `Expr::Column("id")` side by side.
//! - These are *syntax* errors: missing `FROM`, unbalanced parentheses, a
//!   trailing comma, `CREATE TABLE` without columns. They use
//!   [`crate::Error::Sql`] with the offset of the token that was expected or
//!   found.
//! - `SELECT *` becomes [`SelectItem::Wildcard`]; the planner expands it. The
//!   parser must not know that `users` has three columns.
//! - Empty input is a valid program with zero statements. A lone `;` is also
//!   valid (zero statements). This keeps the REPL simple.
//! - Duplicate column names in `CREATE TABLE` are a planning error, not a parse
//!   error; the parser only checks grammar. (It may reject them anyway if that
//!   is easier, but document the choice.)
//!
//! ## Edge cases to get right
//!
//! - Trailing semicolon: `SELECT * FROM t;` and `SELECT * FROM t` are the same
//!   program.
//! - Multiple statements: `CREATE ...; INSERT ...; SELECT ...;` returns three
//!   statements in source order.
//! - `INSERT INTO t () VALUES ()` is invalid (empty column/value lists).
//! - Arity mismatch (`INSERT INTO t (a, b) VALUES (1)`) is *not* a parse error
//!   if you cannot see the schema; detect it in the planner. Document where.
//! - Keywords cannot be used as unquoted identifiers. `SELECT from FROM t` is an
//!   error, not a column named `from`.
//! - Deeply nested parentheses: return an error rather than blowing the stack on
//!   adversarial input (a recursion depth limit of, say, 64 is plenty).
#![allow(dead_code, unused_variables)]

use crate::error::Result;

use super::lexer::Token;

/// A complete SQL statement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Statement {
    /// `CREATE TABLE ...`
    CreateTable(CreateTable),
    /// `INSERT INTO ...`
    Insert(Insert),
    /// `SELECT ...`
    Select(Select),
}

/// `CREATE TABLE name (columns...)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateTable {
    /// Table name, lowercased.
    pub table: String,
    /// At least one column, in declaration order.
    pub columns: Vec<ColumnDef>,
}

/// One declared column.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnDef {
    /// Column name, lowercased.
    pub name: String,
    /// Declared type.
    pub data_type: DataType,
    /// True unless the declaration says `NOT NULL`.
    pub nullable: bool,
}

/// `INSERT INTO table [(columns)] VALUES (...), (...)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Insert {
    /// Target table, lowercased.
    pub table: String,
    /// Explicit column list, if the statement had one. `None` means "all
    /// columns in declaration order", which only the planner can expand.
    pub columns: Option<Vec<String>>,
    /// One inner vec per `(...)` tuple. Expressions are folded by the planner.
    pub rows: Vec<Vec<Expr>>,
}

/// `SELECT projection FROM table [WHERE expr]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Select {
    /// What to return, in order.
    pub projection: Vec<SelectItem>,
    /// Table to read, lowercased.
    pub table: String,
    /// Optional row filter. `None` means all rows.
    pub filter: Option<Expr>,
}

/// One item in a `SELECT` projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectItem {
    /// `*` — expanded by the planner using the catalog.
    Wildcard,
    /// A named column.
    Column(String),
}

/// Scalar expression. Only literal/column comparisons and boolean combinations
/// are supported in P6.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    /// A constant, already lexed into a typed value.
    Literal(Value),
    /// A reference to a column of the current row.
    Column(String),
    /// A binary operation. `And`/`Or` combine boolean expressions;
    /// the comparison variants combine two scalars.
    Binary {
        /// Operator to apply.
        op: BinaryOp,
        /// Left operand.
        left: Box<Expr>,
        /// Right operand.
        right: Box<Expr>,
    },
}

/// Binary operators understood by the `WHERE` clause.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    /// `=`
    Eq,
    /// `!=` / `<>`
    Ne,
    /// `<`
    Lt,
    /// `<=`
    Le,
    /// `>`
    Gt,
    /// `>=`
    Ge,
    /// `AND`
    And,
    /// `OR`
    Or,
}

/// A typed SQL value.
///
/// `Null` exists because `NOT NULL` is only meaningful if the absence of a value
/// is representable. Comparisons with `NULL` are three-valued in real SQL; P6
/// may simplify to "NULL never matches" and document it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    /// SQL `NULL`.
    Null,
    /// 64-bit signed integer.
    Integer(i64),
    /// UTF-8 text.
    Text(String),
    /// `TRUE` / `FALSE`.
    Boolean(bool),
}

/// SQL column types supported in P6.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataType {
    /// `INTEGER` — 64-bit signed.
    Integer,
    /// `TEXT` — UTF-8 string.
    Text,
    /// `BOOLEAN` — `TRUE` / `FALSE`.
    Boolean,
}

/// Recursive-descent parser state.
///
/// One `Parser` per program; `parse_statements` consumes it. Tokens include a
/// final [`super::lexer::TokenKind::Eof`] sentinel, which keeps lookahead
/// bounds-check-free.
#[derive(Debug)]
pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    /// Build a parser from an already-tokenised program.
    ///
    /// The token slice must end with `Eof` (the lexer guarantees this). Do not
    /// re-lex here; tests may feed hand-built token streams.
    pub fn new(tokens: Vec<Token>) -> Self {
        todo!("P6: store the tokens at position zero and assert a trailing Eof")
    }

    /// Parse the whole program.
    ///
    /// Skips empty statements (`;;`), stops at `Eof`, and returns statements in
    /// source order. A parse error must not leave a partially consumed stream
    /// visible to the caller (the parser is consumed by value).
    pub fn parse_statements(&mut self) -> Result<Vec<Statement>> {
        todo!("P6: skip semicolons until Eof, parsing one statement per iteration")
    }
}

/// Parse a complete SQL program. Convenience wrapper used by the executor/tests.
pub fn parse(sql: &str) -> Result<Vec<Statement>> {
    todo!("P6: Parser::new(lexer::tokenize(sql)?).parse_statements()")
}
