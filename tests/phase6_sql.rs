//! Phase 6 acceptance tests: SQL subset (`CREATE TABLE`, `INSERT`, `SELECT`).
//!
//! Currently fails on `todo!()`. Run with: `cargo test --test phase6_sql`

use tinydb_rs::sql::executor::{Database, QueryResult};
use tinydb_rs::sql::lexer::{tokenize, TokenKind};
use tinydb_rs::sql::parser::{self, BinaryOp, DataType, Expr, SelectItem, Statement, Value};
use tinydb_rs::{Error, Result};

fn assert_sql_error(result: Result<Vec<QueryResult>>) {
    match result {
        Err(Error::Sql { .. }) | Err(Error::Type { .. }) => {}
        other => panic!("expected a Sql or Type error, got {other:?}"),
    }
}

#[test]
fn lexer_recognises_keywords_literals_and_operators() -> Result<()> {
    let tokens = tokenize("SELECT id FROM users WHERE id >= 10;")?;

    let kinds: Vec<TokenKind> = tokens.iter().map(|t| t.kind.clone()).collect();
    assert_eq!(
        kinds,
        vec![
            TokenKind::Keyword,
            TokenKind::Ident,
            TokenKind::Keyword,
            TokenKind::Ident,
            TokenKind::Keyword,
            TokenKind::Ident,
            TokenKind::Ge,
            TokenKind::Integer,
            TokenKind::Semicolon,
            TokenKind::Eof,
        ]
    );

    // Keywords are normalised to lowercase, identifiers keep their spelling but
    // are folded by the parser.
    assert_eq!(tokens[0].text, "select");
    assert_eq!(tokens[3].text, "users");
    assert_eq!(tokens[7].text, "10");
    assert_eq!(tokens.last().expect("eof").kind, TokenKind::Eof);

    Ok(())
}

#[test]
fn lexer_decodes_doubled_quotes_in_strings() -> Result<()> {
    let tokens = tokenize("SELECT 'It''s fine' -- trailing comment")?;

    assert_eq!(tokens[1].kind, TokenKind::String);
    assert_eq!(tokens[1].text, "It's fine");

    Ok(())
}

#[test]
fn lexer_rejects_an_unterminated_string() {
    assert!(tokenize("SELECT 'oops").is_err());
}

#[test]
fn parser_builds_a_create_table_statement() -> Result<()> {
    let statements = parser::parse("CREATE TABLE users (id INTEGER NOT NULL, name TEXT)")?;
    assert_eq!(statements.len(), 1);

    match &statements[0] {
        Statement::CreateTable(create) => {
            assert_eq!(create.table, "users");
            assert_eq!(create.columns.len(), 2);
            assert_eq!(create.columns[0].name, "id");
            assert_eq!(create.columns[0].data_type, DataType::Integer);
            assert!(!create.columns[0].nullable);
            assert_eq!(create.columns[1].name, "name");
            assert_eq!(create.columns[1].data_type, DataType::Text);
            assert!(create.columns[1].nullable);
        }
        other => panic!("expected CREATE TABLE, got {other:?}"),
    }

    Ok(())
}

#[test]
fn parser_builds_a_select_with_where() -> Result<()> {
    let statements = parser::parse("SELECT * FROM users WHERE id = 1")?;

    match &statements[0] {
        Statement::Select(select) => {
            assert_eq!(select.table, "users");
            assert_eq!(select.projection, vec![SelectItem::Wildcard]);
            assert_eq!(
                select.filter,
                Some(Expr::Binary {
                    op: BinaryOp::Eq,
                    left: Box::new(Expr::Column("id".to_string())),
                    right: Box::new(Expr::Literal(Value::Integer(1))),
                })
            );
        }
        other => panic!("expected SELECT, got {other:?}"),
    }

    Ok(())
}

#[test]
fn parser_handles_multiple_statements_and_empty_programs() -> Result<()> {
    let program =
        parser::parse("CREATE TABLE t (id INTEGER); INSERT INTO t VALUES (1); SELECT * FROM t;")?;
    assert_eq!(program.len(), 3);
    assert!(matches!(program[0], Statement::CreateTable(_)));
    assert!(matches!(program[1], Statement::Insert(_)));
    assert!(matches!(program[2], Statement::Select(_)));

    assert!(parser::parse("")?.is_empty());
    assert!(parser::parse(";;")?.is_empty());

    Ok(())
}

#[test]
fn create_insert_select_roundtrip() -> Result<()> {
    let mut db = Database::open_in_memory()?;

    assert_eq!(
        db.execute("CREATE TABLE users (id INTEGER NOT NULL, name TEXT)")?,
        vec![QueryResult::CreatedTable {
            name: "users".to_string()
        }]
    );

    assert_eq!(
        db.execute("INSERT INTO users VALUES (1, 'ada'), (2, 'grace')")?,
        vec![QueryResult::Inserted { count: 2 }]
    );

    assert_eq!(
        db.execute("SELECT id, name FROM users WHERE id = 2")?,
        vec![QueryResult::Rows {
            columns: vec!["id".to_string(), "name".to_string()],
            rows: vec![vec![Value::Integer(2), Value::Text("grace".to_string())]],
        }]
    );

    Ok(())
}

#[test]
fn select_star_returns_all_rows_in_insertion_order() -> Result<()> {
    let mut db = Database::open_in_memory()?;
    db.execute("CREATE TABLE t (id INTEGER, label TEXT)")?;
    db.execute("INSERT INTO t VALUES (2, 'two'), (1, 'one')")?;

    let results = db.execute("SELECT * FROM t")?;
    assert_eq!(
        results,
        vec![QueryResult::Rows {
            columns: vec!["id".to_string(), "label".to_string()],
            rows: vec![
                vec![Value::Integer(2), Value::Text("two".to_string())],
                vec![Value::Integer(1), Value::Text("one".to_string())],
            ],
        }]
    );

    Ok(())
}

#[test]
fn empty_table_selects_zero_rows_with_column_names() -> Result<()> {
    let mut db = Database::open_in_memory()?;
    db.execute("CREATE TABLE t (id INTEGER)")?;

    assert_eq!(
        db.execute("SELECT id FROM t WHERE id = 1")?,
        vec![QueryResult::Rows {
            columns: vec!["id".to_string()],
            rows: vec![],
        }]
    );

    Ok(())
}

#[test]
fn select_of_unknown_column_is_an_error() -> Result<()> {
    let mut db = Database::open_in_memory()?;
    db.execute("CREATE TABLE t (id INTEGER)")?;

    assert_sql_error(db.execute("SELECT nope FROM t"));
    assert_sql_error(db.execute("SELECT id FROM missing_table"));

    Ok(())
}

#[test]
fn inserting_the_wrong_type_is_an_error() -> Result<()> {
    let mut db = Database::open_in_memory()?;
    db.execute("CREATE TABLE t (id INTEGER NOT NULL)")?;

    assert_sql_error(db.execute("INSERT INTO t VALUES ('not a number')"));
    assert_sql_error(db.execute("INSERT INTO t VALUES (NULL)"));

    Ok(())
}

#[test]
fn creating_a_duplicate_table_is_an_error() -> Result<()> {
    let mut db = Database::open_in_memory()?;
    db.execute("CREATE TABLE t (id INTEGER)")?;

    assert_sql_error(db.execute("CREATE TABLE t (id INTEGER)"));

    Ok(())
}

#[test]
fn schema_and_table_names_reflect_the_catalog() -> Result<()> {
    let mut db = Database::open_in_memory()?;
    assert!(db.table_names()?.is_empty());
    assert!(db.schema("nope")?.is_none());

    db.execute("CREATE TABLE b (id INTEGER)")?;
    db.execute("CREATE TABLE a (name TEXT)")?;

    assert_eq!(db.table_names()?, vec!["a".to_string(), "b".to_string()]);
    let schema = db.schema("a")?.expect("table a exists");
    assert_eq!(schema.name, "a");
    assert_eq!(schema.columns.len(), 1);

    Ok(())
}
