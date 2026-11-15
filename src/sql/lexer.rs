//! # Lexer — Phase 6
//!
//! Turns SQL text into a flat token stream. No grammar, no symbol tables, no
//! allocation except for token text.
//!
//! ## Token rules
//!
//! - Whitespace separates tokens and is otherwise ignored.
//! - `--` starts a line comment that runs to the end of the line (or EOF).
//! - Identifiers: ASCII letters, digits, `_`, not starting with a digit.
//!   Folded to lowercase (so `Users` and `users` are the same table).
//! - Keywords are identifiers that match the reserved word list; they are
//!   emitted as [`TokenKind::Keyword`] and also lowercased. Keep the list in
//!   [`is_keyword`] so the parser and lexer cannot drift apart.
//! - Integers: optional `-`, then one or more digits. No floats, no exponent.
//!   Must fit in `i64`; overflow is a [`crate::Error::Sql`] at the token's
//!   offset, not a panic.
//! - Strings: single-quoted, with `''` as the escape for a single quote
//!   (`'it''s'` → `it's`). Backslashes are literal. Unterminated string is an
//!   error at the opening quote.
//! - Punctuation: `, ( ) * ;` and the comparison operators `= != <> < <= > >=`.
//!   Emit the longest match (`<=` is one token, not `<` then `=`).
//! - Every token records the byte offset of its first character. The EOF token
//!   records the input length. Offsets are for humans and error messages, not
//!   for slicing.
//!
//! ## Edge cases to get right
//!
//! - Empty input produces exactly one token: [`TokenKind::Eof`].
//! - `SELECT*FROM t` (no spaces) still produces five tokens.
//! - `1abc` is an error, not `Integer(1)` followed by `Ident("abc")`.
//! - A lone `!` or `<` at EOF is an error.
//! - Unicode identifiers are out of scope; non-ASCII outside a string literal
//!   is an error. Inside a string it is fine (and must be preserved as UTF-8).
//! - `Token::text` for a string literal should hold the *decoded* text, not the
//!   raw quoted form. Document that choice in the parser if you disagree.

use crate::error::Result;

/// What a token is, independent of its spelling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    /// A reserved word: `SELECT`, `FROM`, `WHERE`, `CREATE`, `TABLE`, ...
    Keyword,
    /// A table or column name.
    Ident,
    /// A decimal integer literal. The text is the digits (with optional `-`).
    Integer,
    /// A single-quoted string literal. The text is the decoded contents.
    String,
    /// `,`
    Comma,
    /// `(`
    LParen,
    /// `)`
    RParen,
    /// `*` (only valid in `SELECT *` in P6)
    Star,
    /// `;`
    Semicolon,
    /// `=`
    Eq,
    /// `!=` or `<>`
    Ne,
    /// `<`
    Lt,
    /// `<=`
    Le,
    /// `>`
    Gt,
    /// `>=`
    Ge,
    /// End of input. Always the last token.
    Eof,
}

/// One lexical token plus its position in the original input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    /// Token category.
    pub kind: TokenKind,
    /// Normalised text (see the module docs for folding rules).
    pub text: String,
    /// Byte offset of the token's first character.
    pub pos: usize,
}

/// Streaming lexer over a borrowed SQL string.
///
/// You may implement this as a pull-based iterator or as a single
/// [`Lexer::tokenize`] pass; the public echo of the struct is here so that
/// error offsets can be computed while scanning.
#[derive(Debug)]
pub struct Lexer<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> Lexer<'a> {
    /// Create a lexer at the start of `input`.
    ///
    /// Does not validate anything yet; errors surface from `tokenize`.
    pub fn new(input: &'a str) -> Self {
        todo!("P6: store the input and reset the cursor to zero")
    }

    /// Consume the input and return every token, ending with [`TokenKind::Eof`].
    ///
    /// Errors must carry the byte offset of the offending character.
    pub fn tokenize(self) -> Result<Vec<Token>> {
        todo!("P6: scan until EOF, pushing one token per lexeme")
    }
}

/// Convenience wrapper around [`Lexer::new`] + [`Lexer::tokenize`].
///
/// This is what the parser and the tests call.
pub fn tokenize(input: &str) -> Result<Vec<Token>> {
    todo!("P6: Lexer::new(input).tokenize()")
}

/// The reserved-word list. Keep this the single source of truth.
///
/// Return `true` for exact matches only; `input` is expected to be already
/// lowercased. Words: `and`, `boolean`, `create`, `false`, `from`, `insert`,
/// `integer`, `into`, `not`, `null`, `or`, `select`, `table`, `text`, `true`,
/// `values`, `where`.
pub fn is_keyword(input: &str) -> bool {
    todo!("P6: matches!(input, \"and\" | \"boolean\" | ...)")
}
