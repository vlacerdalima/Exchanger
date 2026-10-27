use std::{env, fs};

use anyhow::{Context, Result};

fn main() -> Result<()> {
    let path = env::args()
        .nth(1)
        .context("uso: matching-engine <arquivo>")?;
    let contents =
        fs::read_to_string(&path).with_context(|| format!("não foi possível ler {path}"))?;

    for line in contents.lines().filter(|line| !line.trim().is_empty()) {
        println!("{line}");
    }

    Ok(())
}
