use matching_engine::parser::{ParseError, parse_line};
use matching_engine::{Price, Qt, Side};

fn parsed(side: Side, price: u64, qt: u64) -> (Side, Price, Qt) {
    (side, Price(price), Qt(qt))
}

#[test]
fn integer_price_becomes_cents() {
    assert_eq!(parse_line("BID 10 @ 5"), Ok(parsed(Side::Buy, 500, 10)));
}

#[test]
fn ask_becomes_sell() {
    assert_eq!(parse_line("ASK 3 @ 1.1"), Ok(parsed(Side::Sell, 110, 3)));
}

#[test]
fn price_with_two_decimal_places() {
    assert_eq!(parse_line("BID 1 @ 1.10"), Ok(parsed(Side::Buy, 110, 1)));
    assert_eq!(parse_line("ASK 1 @ 0.29"), Ok(parsed(Side::Sell, 29, 1)));
    assert_eq!(parse_line("BID 1 @ 0.05"), Ok(parsed(Side::Buy, 5, 1)));
}

#[test]
fn extra_whitespace_is_tolerated() {
    assert_eq!(
        parse_line("  BID   10 \t@  5  "),
        Ok(parsed(Side::Buy, 500, 10))
    );
}

#[test]
fn zero_passes_the_parser() {
    assert_eq!(parse_line("BID 0 @ 0"), Ok(parsed(Side::Buy, 0, 0)));
}

#[test]
fn largest_representable_price() {
    assert_eq!(
        parse_line("BID 1 @ 184467440737095516.15"),
        Ok(parsed(Side::Buy, u64::MAX, 1))
    );
}

#[test]
fn wrong_token_count() {
    assert_eq!(
        parse_line(""),
        Err(ParseError::WrongTokenCount { found: 0 })
    );
    assert_eq!(
        parse_line("BID 10@5"),
        Err(ParseError::WrongTokenCount { found: 2 })
    );
    assert_eq!(
        parse_line("BID 10 @ 5 extra"),
        Err(ParseError::WrongTokenCount { found: 5 })
    );
}

#[test]
fn unknown_side() {
    for side in ["BUY", "bid", "ask", "SELL"] {
        assert_eq!(
            parse_line(&format!("{side} 10 @ 5")),
            Err(ParseError::UnknownSide(side.to_string()))
        );
    }
}

#[test]
fn missing_at() {
    assert_eq!(
        parse_line("BID 10 # 5"),
        Err(ParseError::MissingAt("#".to_string()))
    );
}

#[test]
fn invalid_quantity() {
    for qt in ["+5", "-5", "1.5", "abc", "18446744073709551616"] {
        assert_eq!(
            parse_line(&format!("BID {qt} @ 5")),
            Err(ParseError::InvalidQuantity(qt.to_string()))
        );
    }
}

#[test]
fn invalid_price() {
    for price in [
        ".5",
        "5.",
        "-1",
        "+1",
        "1e3",
        "1.2.3",
        "1,5",
        "184467440737095517",
        "184467440737095516.16",
    ] {
        assert_eq!(
            parse_line(&format!("BID 1 @ {price}")),
            Err(ParseError::InvalidPrice(price.to_string()))
        );
    }
}

#[test]
fn more_than_two_decimal_places() {
    assert_eq!(
        parse_line("BID 1 @ 1.234"),
        Err(ParseError::TooManyDecimals("1.234".to_string()))
    );
}
