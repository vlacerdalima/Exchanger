use std::{env, fs};

use anyhow::{Context, Result};
use matching_engine::parser::parse_line;

fn main() -> Result<()> {
    let path = env::args()
        .nth(1)
        .context("usage: matching-engine <file>")?;
    let contents = fs::read_to_string(&path).with_context(|| format!("could not read {path}"))?;

    for (index, line) in contents.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }

        match parse_line(line) {
            Ok(parsed) => println!("{parsed:?}"),
            Err(error) => eprintln!("line {}: {error}", index + 1),
        }
    }

    Ok(())
}
