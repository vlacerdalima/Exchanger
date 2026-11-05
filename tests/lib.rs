use matching_engine::{Id, OrderBook, Price, Qt, Side, Trade};

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

// submit (matching)

/// Base book, built with `insert`:
/// Buy 4 @ 99 -> Id(0), Sell 5 @ 100 -> Id(1), Sell 2 @ 100 -> Id(2), Sell 3 @ 101 -> Id(3).
fn base_book() -> OrderBook {
    let mut book = OrderBook::new();
    book.insert(Side::Buy, Price(99), Qt(4));
    book.insert(Side::Sell, Price(100), Qt(5));
    book.insert(Side::Sell, Price(100), Qt(2));
    book.insert(Side::Sell, Price(101), Qt(3));
    book
}

/// Orders at the level as `(id, qt)`, in queue order.
fn orders_at(book: &OrderBook, side: Side, price: Price) -> Vec<(Id, Qt)> {
    book.level(side, price)
        .map(|order| (order.id(), order.qt()))
        .collect()
}

fn trade(maker: u64, taker: u64, price: u64, qt: u64) -> Trade {
    Trade {
        maker: Id(maker),
        taker: Id(taker),
        price: Price(price),
        qt: Qt(qt),
    }
}

#[test]
fn buy_sweeps_two_levels() {
    let mut book = base_book();

    let (id, trades) = book.submit(Side::Buy, Price(101), Qt(9));

    assert_eq!(id, Id(4));
    assert_eq!(
        trades,
        vec![
            trade(1, 4, 100, 5),
            trade(2, 4, 100, 2),
            trade(3, 4, 101, 2),
        ]
    );
    assert_eq!(orders_at(&book, Side::Sell, Price(100)), vec![]);
    assert_eq!(
        orders_at(&book, Side::Sell, Price(101)),
        vec![(Id(3), Qt(1))]
    );
    assert_eq!(best_ask(&book), Some((Price(101), Id(3), Qt(1))));
    assert_eq!(best_bid(&book), Some((Price(99), Id(0), Qt(4))));
    assert_eq!(orders_at(&book, Side::Buy, Price(101)), vec![]);
}

#[test]
fn sell_exhausts_bid_side() {
    let mut book = base_book();

    let (id, trades) = book.submit(Side::Sell, Price(98), Qt(6));

    assert_eq!(id, Id(4));
    assert_eq!(trades, vec![trade(0, 4, 99, 4)]);
    assert_eq!(best_bid(&book), None);
    assert_eq!(orders_at(&book, Side::Buy, Price(99)), vec![]);
    assert_eq!(best_ask(&book), Some((Price(98), Id(4), Qt(2))));
    assert_eq!(
        orders_at(&book, Side::Sell, Price(98)),
        vec![(Id(4), Qt(2))]
    );
    assert_eq!(
        orders_at(&book, Side::Sell, Price(100)),
        vec![(Id(1), Qt(5)), (Id(2), Qt(2))]
    );
    assert_eq!(
        orders_at(&book, Side::Sell, Price(101)),
        vec![(Id(3), Qt(3))]
    );
}

#[test]
fn exact_cross() {
    let mut book = base_book();

    let (id, trades) = book.submit(Side::Buy, Price(100), Qt(5));

    assert_eq!(id, Id(4));
    assert_eq!(trades, vec![trade(1, 4, 100, 5)]);
    assert_eq!(
        orders_at(&book, Side::Sell, Price(100)),
        vec![(Id(2), Qt(2))]
    );
    assert_eq!(orders_at(&book, Side::Buy, Price(100)), vec![]);
}

#[test]
fn price_out_of_reach_joins_back_of_queue() {
    let mut book = base_book();

    let (id, trades) = book.submit(Side::Sell, Price(100), Qt(2));

    assert_eq!(id, Id(4));
    assert_eq!(trades, vec![]);
    assert_eq!(
        orders_at(&book, Side::Sell, Price(100)),
        vec![(Id(1), Qt(5)), (Id(2), Qt(2)), (Id(4), Qt(2))]
    );
    assert_eq!(best_bid(&book), Some((Price(99), Id(0), Qt(4))));
}

#[test]
fn partial_fill_keeps_priority() {
    let mut book = base_book();
    book.submit(Side::Buy, Price(101), Qt(9));

    let (id, trades) = book.submit(Side::Buy, Price(101), Qt(1));

    assert_eq!(id, Id(5));
    assert_eq!(trades, vec![trade(3, 5, 101, 1)]);
    assert_eq!(best_ask(&book), None);
}

#[test]
fn sell_sweeps_two_bid_levels() {
    let mut book = OrderBook::new();
    book.insert(Side::Buy, Price(99), Qt(5));
    book.insert(Side::Buy, Price(98), Qt(5));

    let (id, trades) = book.submit(Side::Sell, Price(98), Qt(8));

    // Bids match from the highest price down.
    assert_eq!(id, Id(2));
    assert_eq!(trades, vec![trade(0, 2, 99, 5), trade(1, 2, 98, 3)]);
    assert_eq!(orders_at(&book, Side::Buy, Price(99)), vec![]);
    assert_eq!(orders_at(&book, Side::Buy, Price(98)), vec![(Id(1), Qt(2))]);
    assert_eq!(best_bid(&book), Some((Price(98), Id(1), Qt(2))));
    assert_eq!(best_ask(&book), None);
}

#[test]
fn sell_exact_cross() {
    let mut book = base_book();

    let (id, trades) = book.submit(Side::Sell, Price(99), Qt(4));

    assert_eq!(id, Id(4));
    assert_eq!(trades, vec![trade(0, 4, 99, 4)]);
    assert_eq!(best_bid(&book), None);
    assert_eq!(orders_at(&book, Side::Sell, Price(99)), vec![]);
    assert_eq!(best_ask(&book), Some((Price(100), Id(1), Qt(5))));
}

#[test]
fn partially_filled_maker_stays_at_front() {
    let mut book = base_book();

    let (id, trades) = book.submit(Side::Buy, Price(100), Qt(3));

    assert_eq!(id, Id(4));
    assert_eq!(trades, vec![trade(1, 4, 100, 3)]);
    assert_eq!(
        orders_at(&book, Side::Sell, Price(100)),
        vec![(Id(1), Qt(2)), (Id(2), Qt(2))]
    );
    assert_eq!(orders_at(&book, Side::Buy, Price(100)), vec![]);
    assert_eq!(best_ask(&book), Some((Price(100), Id(1), Qt(2))));
}

#[test]
fn better_limit_trades_at_maker_price_and_rests_at_own_limit() {
    let mut book = base_book();

    let (id, trades) = book.submit(Side::Buy, Price(105), Qt(12));

    assert_eq!(id, Id(4));
    assert_eq!(
        trades,
        vec![
            trade(1, 4, 100, 5),
            trade(2, 4, 100, 2),
            trade(3, 4, 101, 3),
        ]
    );
    assert_eq!(best_ask(&book), None);
    assert_eq!(
        orders_at(&book, Side::Buy, Price(105)),
        vec![(Id(4), Qt(2))]
    );
    assert_eq!(orders_at(&book, Side::Buy, Price(101)), vec![]);
    assert_eq!(best_bid(&book), Some((Price(105), Id(4), Qt(2))));
}

#[test]
fn rested_remainder_becomes_maker() {
    let mut book = base_book();
    book.submit(Side::Sell, Price(98), Qt(6));

    let (id, trades) = book.submit(Side::Buy, Price(98), Qt(2));

    assert_eq!(id, Id(5));
    assert_eq!(trades, vec![trade(4, 5, 98, 2)]);
    assert_eq!(orders_at(&book, Side::Sell, Price(98)), vec![]);
    assert_eq!(orders_at(&book, Side::Buy, Price(98)), vec![]);
    assert_eq!(best_bid(&book), None);
    assert_eq!(best_ask(&book), Some((Price(100), Id(1), Qt(5))));
}

#[test]
fn buy_on_empty_book_rests() {
    let mut book = OrderBook::new();

    let (id, trades) = book.submit(Side::Buy, Price(500), Qt(10));

    assert_eq!(id, Id(0));
    assert_eq!(trades, vec![]);
    assert_eq!(best_bid(&book), Some((Price(500), Id(0), Qt(10))));
    assert_eq!(best_ask(&book), None);
}

#[test]
fn sell_on_empty_book_rests() {
    let mut book = OrderBook::new();

    let (id, trades) = book.submit(Side::Sell, Price(500), Qt(10));

    assert_eq!(id, Id(0));
    assert_eq!(trades, vec![]);
    assert_eq!(best_ask(&book), Some((Price(500), Id(0), Qt(10))));
    assert_eq!(best_bid(&book), None);
}

#[test]
fn buy_without_cross_joins_back_of_queue() {
    let mut book = base_book();

    let (id, trades) = book.submit(Side::Buy, Price(99), Qt(1));

    assert_eq!(id, Id(4));
    assert_eq!(trades, vec![]);
    assert_eq!(
        orders_at(&book, Side::Buy, Price(99)),
        vec![(Id(0), Qt(4)), (Id(4), Qt(1))]
    );
    assert_eq!(best_ask(&book), Some((Price(100), Id(1), Qt(5))));
}

#[test]
fn fully_filled_order_still_consumes_id() {
    let mut book = base_book();

    let (id, _) = book.submit(Side::Buy, Price(100), Qt(5));

    assert_eq!(id, Id(4));
    assert_eq!(orders_at(&book, Side::Buy, Price(100)), vec![]);
    assert_eq!(book.insert(Side::Sell, Price(200), Qt(1)), Id(5));
    assert_eq!(book.submit(Side::Buy, Price(50), Qt(1)).0, Id(6));
}

#[test]
fn buy_exhausts_ask_side() {
    let mut book = base_book();

    let (id, trades) = book.submit(Side::Buy, Price(101), Qt(12));

    assert_eq!(id, Id(4));
    assert_eq!(
        trades,
        vec![
            trade(1, 4, 100, 5),
            trade(2, 4, 100, 2),
            trade(3, 4, 101, 3),
        ]
    );
    assert_eq!(best_ask(&book), None);
    assert_eq!(orders_at(&book, Side::Sell, Price(101)), vec![]);
    assert_eq!(
        orders_at(&book, Side::Buy, Price(101)),
        vec![(Id(4), Qt(2))]
    );
    assert_eq!(best_bid(&book), Some((Price(101), Id(4), Qt(2))));
}
