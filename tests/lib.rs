use matching_engine::{Id, OrderBook, Price, Qt, Side};

// `RestingOrder` cannot be built outside the crate, so the helpers below turn it
// into plain values that `assert_eq!` can compare.

/// Ids at the given level, in queue order.
fn ids_at(book: &OrderBook, side: Side, price: Price) -> Vec<Id> {
    book.level(side, price).map(|order| order.id()).collect()
}

/// Best bid as `(price, id, qt)`.
fn best_bid(book: &OrderBook) -> Option<(Price, Id, Qt)> {
    book.best_bid()
        .map(|(price, order)| (price, order.id(), order.qt()))
}

/// Best ask as `(price, id, qt)`.
fn best_ask(book: &OrderBook) -> Option<(Price, Id, Qt)> {
    book.best_ask()
        .map(|(price, order)| (price, order.id(), order.qt()))
}

// Empty book

#[test]
fn empty_book_has_no_best_bid_or_ask() {
    let book = OrderBook::new();

    assert_eq!(best_bid(&book), None);
    assert_eq!(best_ask(&book), None);
}

#[test]
fn empty_book_has_empty_levels() {
    let book = OrderBook::new();

    assert_eq!(ids_at(&book, Side::Buy, Price(500)), vec![]);
    assert_eq!(ids_at(&book, Side::Sell, Price(500)), vec![]);
}

// insert: ids

#[test]
fn first_id_is_zero() {
    let mut book = OrderBook::new();

    assert_eq!(book.insert(Side::Buy, Price(500), Qt(10)), Id(0));
}

#[test]
fn ids_are_shared_across_sides() {
    let mut book = OrderBook::new();

    assert_eq!(book.insert(Side::Buy, Price(500), Qt(10)), Id(0));
    assert_eq!(book.insert(Side::Sell, Price(510), Qt(10)), Id(1));
    assert_eq!(book.insert(Side::Buy, Price(490), Qt(10)), Id(2));
}

// insert: side

#[test]
fn buy_rests_on_bid_side_only() {
    let mut book = OrderBook::new();
    let id = book.insert(Side::Buy, Price(500), Qt(10));

    assert_eq!(ids_at(&book, Side::Buy, Price(500)), vec![id]);
    assert_eq!(ids_at(&book, Side::Sell, Price(500)), vec![]);
    assert_eq!(best_ask(&book), None);
}

#[test]
fn sell_rests_on_ask_side_only() {
    let mut book = OrderBook::new();
    let id = book.insert(Side::Sell, Price(500), Qt(10));

    assert_eq!(ids_at(&book, Side::Sell, Price(500)), vec![id]);
    assert_eq!(ids_at(&book, Side::Buy, Price(500)), vec![]);
    assert_eq!(best_bid(&book), None);
}

#[test]
fn resting_order_keeps_id_and_qt() {
    let mut book = OrderBook::new();
    let id = book.insert(Side::Buy, Price(500), Qt(7));

    let orders: Vec<(Id, Qt)> = book
        .level(Side::Buy, Price(500))
        .map(|order| (order.id(), order.qt()))
        .collect();
    assert_eq!(orders, vec![(id, Qt(7))]);
}

// Time priority (FIFO within a level)

#[test]
fn same_level_keeps_arrival_order() {
    let mut book = OrderBook::new();
    let first = book.insert(Side::Buy, Price(500), Qt(1));
    let second = book.insert(Side::Buy, Price(500), Qt(2));
    let third = book.insert(Side::Buy, Price(500), Qt(3));

    assert_eq!(
        ids_at(&book, Side::Buy, Price(500)),
        vec![first, second, third]
    );
}

#[test]
fn best_bid_is_oldest_order_at_best_level() {
    let mut book = OrderBook::new();
    let oldest = book.insert(Side::Buy, Price(500), Qt(1));
    book.insert(Side::Buy, Price(500), Qt(2));

    assert_eq!(best_bid(&book), Some((Price(500), oldest, Qt(1))));
}

#[test]
fn best_ask_is_oldest_order_at_best_level() {
    let mut book = OrderBook::new();
    let oldest = book.insert(Side::Sell, Price(500), Qt(1));
    book.insert(Side::Sell, Price(500), Qt(2));

    assert_eq!(best_ask(&book), Some((Price(500), oldest, Qt(1))));
}

// Price priority

#[test]
fn best_bid_is_highest_price() {
    let mut book = OrderBook::new();
    book.insert(Side::Buy, Price(500), Qt(1));
    let highest = book.insert(Side::Buy, Price(510), Qt(2));
    book.insert(Side::Buy, Price(490), Qt(3));

    assert_eq!(best_bid(&book), Some((Price(510), highest, Qt(2))));
}

#[test]
fn best_ask_is_lowest_price() {
    let mut book = OrderBook::new();
    book.insert(Side::Sell, Price(500), Qt(1));
    book.insert(Side::Sell, Price(510), Qt(2));
    let lowest = book.insert(Side::Sell, Price(490), Qt(3));

    assert_eq!(best_ask(&book), Some((Price(490), lowest, Qt(3))));
}

#[test]
fn sides_do_not_mix() {
    let mut book = OrderBook::new();
    let bid = book.insert(Side::Buy, Price(400), Qt(1));
    let ask = book.insert(Side::Sell, Price(600), Qt(2));

    assert_eq!(best_bid(&book), Some((Price(400), bid, Qt(1))));
    assert_eq!(best_ask(&book), Some((Price(600), ask, Qt(2))));
}

// level

#[test]
fn level_at_missing_price_is_empty() {
    let mut book = OrderBook::new();
    book.insert(Side::Buy, Price(500), Qt(1));

    assert_eq!(ids_at(&book, Side::Buy, Price(501)), vec![]);
}

#[test]
fn level_on_other_side_is_empty() {
    let mut book = OrderBook::new();
    book.insert(Side::Buy, Price(500), Qt(1));

    assert_eq!(ids_at(&book, Side::Sell, Price(500)), vec![]);
}

#[test]
fn levels_are_independent() {
    let mut book = OrderBook::new();
    let at_500 = book.insert(Side::Buy, Price(500), Qt(1));
    let at_510 = book.insert(Side::Buy, Price(510), Qt(2));

    assert_eq!(ids_at(&book, Side::Buy, Price(500)), vec![at_500]);
    assert_eq!(ids_at(&book, Side::Buy, Price(510)), vec![at_510]);
}

// Current behavior that will change (validation and matching)

#[test]
fn zero_qt_and_price_are_accepted_without_validation() {
    let mut book = OrderBook::new();
    let id = book.insert(Side::Buy, Price(0), Qt(0));

    assert_eq!(best_bid(&book), Some((Price(0), id, Qt(0))));
}

#[test]
fn crossed_book_rests_without_matching() {
    let mut book = OrderBook::new();
    let bid = book.insert(Side::Buy, Price(510), Qt(1));
    let ask = book.insert(Side::Sell, Price(500), Qt(1));

    assert_eq!(best_bid(&book), Some((Price(510), bid, Qt(1))));
    assert_eq!(best_ask(&book), Some((Price(500), ask, Qt(1))));
}

#[test]
fn extreme_prices_are_stored() {
    let mut book = OrderBook::new();
    let low = book.insert(Side::Buy, Price(0), Qt(1));
    let high = book.insert(Side::Buy, Price(u64::MAX), Qt(1));

    assert_eq!(best_bid(&book), Some((Price(u64::MAX), high, Qt(1))));
    assert_eq!(ids_at(&book, Side::Buy, Price(0)), vec![low]);
}
