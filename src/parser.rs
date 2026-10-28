//! Parser for input lines in the `SIDE QTY @ PRICE` format.
//!
//! It only checks syntax: business validation (zero quantity or price, tick
//! size etc.) does not happen here.

use std::fmt;

use crate::{Price, Qt, Side};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    WrongTokenCount { found: usize },
    UnknownSide(String),
    MissingAt(String),
    InvalidQuantity(String),
    InvalidPrice(String),
    TooManyDecimals(String),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::WrongTokenCount { found } => {
                write!(f, "expected 4 tokens (SIDE QTY @ PRICE), found {found}")
            }
            ParseError::UnknownSide(token) => {
                write!(f, "unknown side '{token}' (use BID or ASK)")
            }
            ParseError::MissingAt(token) => write!(f, "expected '@', found '{token}'"),
            ParseError::InvalidQuantity(token) => {
                write!(f, "invalid quantity '{token}' (unsigned integer)")
            }
            ParseError::InvalidPrice(token) => write!(f, "invalid price '{token}'"),
            ParseError::TooManyDecimals(token) => {
                write!(f, "price '{token}' has more than 2 decimal places")
            }
        }
    }
}

impl std::error::Error for ParseError {}

/// Parses a `BID|ASK QTY @ PRICE` line into its side, price and quantity.
///
/// Building the [`Order`](crate::Order), including its id, is up to the caller.
///
/// The price accepts up to 2 decimal places and is stored in cents:
/// `5` becomes `Price(500)` and `1.1` becomes `Price(110)`.
pub fn parse_line(line: &str) -> Result<(Side, Price, Qt), ParseError> {
    let tokens: Vec<&str> = line.split_whitespace().collect();
    let [side, qt, at, price] = tokens[..] else {
        return Err(ParseError::WrongTokenCount {
            found: tokens.len(),
        });
    };

    let side = parse_side(side)?;
    let qt = parse_qt(qt)?;
    if at != "@" {
        return Err(ParseError::MissingAt(at.to_string()));
    }
    let price = parse_price(price)?;

    Ok((side, price, qt))
}

fn parse_side(token: &str) -> Result<Side, ParseError> {
    match token {
        "BID" => Ok(Side::Buy),
        "ASK" => Ok(Side::Sell),
        _ => Err(ParseError::UnknownSide(token.to_string())),
    }
}

fn parse_qt(token: &str) -> Result<Qt, ParseError> {
    let invalid = || ParseError::InvalidQuantity(token.to_string());

    // `u64::from_str` accepts "+5", so the digits are checked first.
    if !is_digits(token) {
        return Err(invalid());
    }
    token.parse().map(Qt).map_err(|_| invalid())
}

// Read as text, never through f64: 0.29 * 100.0 gives 28.999...
fn parse_price(token: &str) -> Result<Price, ParseError> {
    let invalid = || ParseError::InvalidPrice(token.to_string());

    let (int_part, frac_part) = match token.split_once('.') {
        Some((int_part, frac_part)) => (int_part, Some(frac_part)),
        None => (token, None),
    };

    if !is_digits(int_part) {
        return Err(invalid());
    }
    let units: u64 = int_part.parse().map_err(|_| invalid())?;

    let frac_cents = match frac_part {
        None => 0,
        Some(frac) if !is_digits(frac) => return Err(invalid()),
        Some(frac) if frac.len() > 2 => {
            return Err(ParseError::TooManyDecimals(token.to_string()));
        }
        Some(frac) => {
            let value: u64 = frac.parse().map_err(|_| invalid())?;
            if frac.len() == 1 { value * 10 } else { value }
        }
    };

    units
        .checked_mul(100)
        .and_then(|cents| cents.checked_add(frac_cents))
        .map(Price)
        .ok_or_else(invalid)
}

fn is_digits(token: &str) -> bool {
    !token.is_empty() && token.bytes().all(|byte| byte.is_ascii_digit())
}
