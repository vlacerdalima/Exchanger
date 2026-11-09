//! Parser for input lines: `BID|ASK QTY @ PRICE` (submit a limit order),
//! `CANCEL ID` (cancel a resting order) and `VIEW ID` (show an order's state).
//!
//! It only checks syntax: business validation (zero quantity or price, tick
//! size, unknown ids etc.) does not happen here.

use std::fmt;

use crate::{Id, Price, Qt, Side};

/// One parsed input line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    /// `BID|ASK QTY @ PRICE`: submit a limit order.
    Submit {
        /// `BID` is buy, `ASK` is sell.
        side: Side,
        /// Limit price, in cents.
        price: Price,
        /// Quantity.
        qt: Qt,
    },
    /// `CANCEL ID`: cancel a resting order.
    Cancel(Id),
    /// `VIEW ID`: show an order's state.
    View(Id),
}

/// Why a line could not be parsed; each variant carries the offending input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// The line has the wrong number of whitespace-separated tokens.
    WrongTokenCount {
        /// Number of tokens found.
        found: usize,
    },
    /// The first token is not `BID`, `ASK`, `CANCEL` or `VIEW`.
    UnknownCommand(String),
    /// The third token of a submit is not `@`.
    MissingAt(String),
    /// The quantity is not an unsigned integer that fits in `u64`.
    InvalidQuantity(String),
    /// The price is not a non-negative decimal that fits in `u64` cents.
    InvalidPrice(String),
    /// The price has more than 2 decimal places.
    TooManyDecimals(String),
    /// The id is not an unsigned integer that fits in `u64`.
    InvalidId(String),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::WrongTokenCount { found } => {
                write!(
                    f,
                    "expected SIDE QTY @ PRICE, CANCEL ID or VIEW ID, found {found} tokens"
                )
            }
            ParseError::UnknownCommand(token) => {
                write!(
                    f,
                    "unknown command '{token}' (use BID, ASK, CANCEL or VIEW)"
                )
            }
            ParseError::MissingAt(token) => write!(f, "expected '@', found '{token}'"),
            ParseError::InvalidQuantity(token) => {
                write!(f, "invalid quantity '{token}' (unsigned integer)")
            }
            ParseError::InvalidPrice(token) => write!(f, "invalid price '{token}'"),
            ParseError::TooManyDecimals(token) => {
                write!(f, "price '{token}' has more than 2 decimal places")
            }
            ParseError::InvalidId(token) => write!(f, "invalid id '{token}' (unsigned integer)"),
        }
    }
}

impl std::error::Error for ParseError {}

/// Parses one input line into a [`Command`].
///
/// - `BID|ASK QTY @ PRICE` becomes [`Command::Submit`]. The price accepts up to 2
///   decimal places and is stored in cents: `5` becomes `Price(500)` and `1.1`
///   becomes `Price(110)`.
/// - `CANCEL ID` becomes [`Command::Cancel`].
/// - `VIEW ID` becomes [`Command::View`].
///
/// Keywords are uppercase only; the order id of a submit is assigned by the book.
pub fn parse_line(line: &str) -> Result<Command, ParseError> {
    let tokens: Vec<&str> = line.split_whitespace().collect();
    let wrong_count = || ParseError::WrongTokenCount {
        found: tokens.len(),
    };

    // The first token picks the format; the CANCEL and VIEW arms come first so a
    // 4-token CANCEL or VIEW line is a token-count error, not an unknown command.
    match tokens[..] {
        ["CANCEL", id] => parse_id(id).map(Command::Cancel),
        ["VIEW", id] => parse_id(id).map(Command::View),
        ["CANCEL" | "VIEW", ..] => Err(wrong_count()),
        [side, qt, at, price] => parse_submit(side, qt, at, price),
        [] | ["BID" | "ASK", ..] => Err(wrong_count()),
        [command, ..] => Err(ParseError::UnknownCommand(command.to_string())),
    }
}

fn parse_submit(side: &str, qt: &str, at: &str, price: &str) -> Result<Command, ParseError> {
    let side = parse_side(side)?;
    let qt = parse_qt(qt)?;
    if at != "@" {
        return Err(ParseError::MissingAt(at.to_string()));
    }
    let price = parse_price(price)?;

    Ok(Command::Submit { side, price, qt })
}

fn parse_side(token: &str) -> Result<Side, ParseError> {
    match token {
        "BID" => Ok(Side::Buy),
        "ASK" => Ok(Side::Sell),
        _ => Err(ParseError::UnknownCommand(token.to_string())),
    }
}

fn parse_qt(token: &str) -> Result<Qt, ParseError> {
    parse_unsigned(token)
        .map(Qt)
        .ok_or_else(|| ParseError::InvalidQuantity(token.to_string()))
}

fn parse_id(token: &str) -> Result<Id, ParseError> {
    parse_unsigned(token)
        .map(Id)
        .ok_or_else(|| ParseError::InvalidId(token.to_string()))
}

// `u64::from_str` accepts "+5", so the digits are checked first.
fn parse_unsigned(token: &str) -> Option<u64> {
    if !is_digits(token) {
        return None;
    }
    token.parse().ok()
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
