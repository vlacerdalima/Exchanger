use matching_engine::parser::{Command, ParseError, parse_line};
use matching_engine::{Id, Price, Qt, Side};

fn parsed(side: Side, price: u64, qt: u64) -> Command {
    Command::Submit {
        side,
        price: Price(price),
        qt: Qt(qt),
    }
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
            Err(ParseError::UnknownCommand(side.to_string()))
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

// CANCEL

#[test]
fn cancel_command() {
    assert_eq!(parse_line("CANCEL 3"), Ok(Command::Cancel(Id(3))));
}

#[test]
fn cancel_without_id() {
    assert_eq!(
        parse_line("CANCEL"),
        Err(ParseError::WrongTokenCount { found: 1 })
    );
}

#[test]
fn cancel_with_extra_token() {
    assert_eq!(
        parse_line("CANCEL 3 4"),
        Err(ParseError::WrongTokenCount { found: 3 })
    );
}

#[test]
fn invalid_id() {
    for id in ["abc", "+3"] {
        assert_eq!(
            parse_line(&format!("CANCEL {id}")),
            Err(ParseError::InvalidId(id.to_string()))
        );
    }
}

#[test]
fn sell_is_an_unknown_command() {
    assert_eq!(
        parse_line("SELL 1 @ 10"),
        Err(ParseError::UnknownCommand("SELL".to_string()))
    );
}

// VIEW

#[test]
fn view_command() {
    assert_eq!(parse_line("VIEW 3"), Ok(Command::View(Id(3))));
}

#[test]
fn view_without_id() {
    assert_eq!(
        parse_line("VIEW"),
        Err(ParseError::WrongTokenCount { found: 1 })
    );
}

#[test]
fn view_with_invalid_id() {
    assert_eq!(
        parse_line("VIEW x"),
        Err(ParseError::InvalidId("x".to_string()))
    );
}

#[test]
fn unknown_command_with_wrong_token_count() {
    assert_eq!(
        parse_line("BUY 1 5"),
        Err(ParseError::UnknownCommand("BUY".to_string()))
    );
}
