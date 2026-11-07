use std::{env, fs};

use anyhow::{Context, Result};
use matching_engine::parser::{Command, parse_line};
use matching_engine::{OrderBook, Price};

fn main() -> Result<()> {
    let path = env::args()
        .nth(1)
        .context("usage: matching-engine <file>")?;
    let contents = fs::read_to_string(&path).with_context(|| format!("could not read {path}"))?;

    let mut book = OrderBook::new();

    for (index, line) in contents.lines().enumerate() {
        let n = index + 1;
        if line.trim().is_empty() {
            continue;
        }

        let command = match parse_line(line) {
            Ok(command) => command,
            Err(error) => {
                eprintln!("line {n}: {error}");
                continue;
            }
        };

        match command {
            Command::Submit { side, price, qt } => match book.submit(side, price, qt) {
                Ok((id, trades)) => {
                    println!("ACCEPTED {}", id.0);
                    for trade in trades {
                        println!(
                            "TRADE maker={} taker={} qt={} price={}",
                            trade.maker.0,
                            trade.taker.0,
                            trade.qt.0,
                            format_price(trade.price)
                        );
                    }
                }
                Err(reject) => println!("REJECTED line {n}: {reject:?}"),
            },
            Command::Cancel(id) => match book.cancel(id) {
                Ok(qt) => println!("CANCELLED {} qt={}", id.0, qt.0),
                Err(reject) => println!("CANCEL_REJECTED {}: {reject:?}", id.0),
            },
        }
    }

    Ok(())
}

/// Formats a price in cents with 2 decimal places: `Price(9950)` becomes `"99.50"`.
fn format_price(price: Price) -> String {
    format!("{}.{:02}", price.0 / 100, price.0 % 100)
}
