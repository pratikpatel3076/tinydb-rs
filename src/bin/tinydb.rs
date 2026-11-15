//! `tinydb` — the interactive shell (stub).
//!
//! This binary is **I/O scaffolding only**: banner, prompt, line reading,
//! dot-commands, clean EOF handling. There is deliberately no engine logic here
//! yet, and no `todo!()` either — a REPL that panics on the first keystroke is
//! not useful during development.
//!
//! SQL input currently prints a "not wired up" message. Starting with P1, route
//! it through `MemTable`; from P6, use `tinydb_rs::sql::Database::execute`.
//!
//! Run with: `cargo run --bin tinydb`

use std::io::{self, BufRead, Write};

const HELP: &str = "\
Commands:
  .help          show this help
  .quit / .exit  leave the shell
Any other line is treated as SQL (wired up in P6).";

fn main() -> io::Result<()> {
    println!("tinydb {} (scaffold)", env!("CARGO_PKG_VERSION"));
    println!("No storage engine yet - see docs/ROADMAP.md.");
    println!("Type .help for commands, .quit to leave.\n");

    let stdin = io::stdin();
    let mut input = stdin.lock();
    let mut line = String::new();

    loop {
        print!("tinydb> ");
        io::stdout().flush()?;

        line.clear();
        if input.read_line(&mut line)? == 0 {
            // EOF (Ctrl-Z on Windows, Ctrl-D elsewhere).
            println!();
            return Ok(());
        }

        let command = line.trim();
        match command {
            "" => continue,
            ".quit" | ".exit" | "quit" | "exit" => return Ok(()),
            ".help" | "help" => println!("{HELP}"),
            other => {
                println!("error: SQL execution is not implemented yet (input: {other:?})");
                println!("       P1 starts in src/memtable.rs; SQL lands in P6.");
            }
        }
    }
}
