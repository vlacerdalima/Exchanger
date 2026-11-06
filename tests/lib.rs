use matching_engine::{Id, OrderBook, Price, Qt, Reject, Side, Trade};

// `RestingOrder` cannot be built outside the crate, so the helpers below turn it
// into plain values that `assert_eq!` can compare.

/// Submits an order that must not cross anything, so it rests in the book; returns its id.
fn place(book: &mut OrderBook, side: Side, price: Price, qt: Qt) -> Id {
    let (id, trades) = book.submit(side, price, qt).unwrap();
    assert_eq!(
        trades,
        vec![],
        "setup order was expected to rest, not trade"
    );
    id
}

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

// Ids

#[test]
fn first_id_is_zero() {
    let mut book = OrderBook::new();

    assert_eq!(place(&mut book, Side::Buy, Price(500), Qt(10)), Id(0));
}

#[test]
fn ids_are_shared_across_sides() {
    let mut book = OrderBook::new();

    assert_eq!(place(&mut book, Side::Buy, Price(500), Qt(10)), Id(0));
    assert_eq!(place(&mut book, Side::Sell, Price(510), Qt(10)), Id(1));
    assert_eq!(place(&mut book, Side::Buy, Price(490), Qt(10)), Id(2));
}

// Side

#[test]
fn buy_rests_on_bid_side_only() {
    let mut book = OrderBook::new();
    let id = place(&mut book, Side::Buy, Price(500), Qt(10));

    assert_eq!(ids_at(&book, Side::Buy, Price(500)), vec![id]);
    assert_eq!(ids_at(&book, Side::Sell, Price(500)), vec![]);
    assert_eq!(best_ask(&book), None);
}

#[test]
fn sell_rests_on_ask_side_only() {
    let mut book = OrderBook::new();
    let id = place(&mut book, Side::Sell, Price(500), Qt(10));

    assert_eq!(ids_at(&book, Side::Sell, Price(500)), vec![id]);
    assert_eq!(ids_at(&book, Side::Buy, Price(500)), vec![]);
    assert_eq!(best_bid(&book), None);
}

#[test]
fn resting_order_keeps_id_and_qt() {
    let mut book = OrderBook::new();
    let id = place(&mut book, Side::Buy, Price(500), Qt(7));

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
    let first = place(&mut book, Side::Buy, Price(500), Qt(1));
    let second = place(&mut book, Side::Buy, Price(500), Qt(2));
    let third = place(&mut book, Side::Buy, Price(500), Qt(3));

    assert_eq!(
        ids_at(&book, Side::Buy, Price(500)),
        vec![first, second, third]
    );
}

#[test]
fn best_bid_is_oldest_order_at_best_level() {
    let mut book = OrderBook::new();
    let oldest = place(&mut book, Side::Buy, Price(500), Qt(1));
    place(&mut book, Side::Buy, Price(500), Qt(2));

    assert_eq!(best_bid(&book), Some((Price(500), oldest, Qt(1))));
}

#[test]
fn best_ask_is_oldest_order_at_best_level() {
    let mut book = OrderBook::new();
    let oldest = place(&mut book, Side::Sell, Price(500), Qt(1));
    place(&mut book, Side::Sell, Price(500), Qt(2));

    assert_eq!(best_ask(&book), Some((Price(500), oldest, Qt(1))));
}

// Price priority

#[test]
fn best_bid_is_highest_price() {
    let mut book = OrderBook::new();
    place(&mut book, Side::Buy, Price(500), Qt(1));
    let highest = place(&mut book, Side::Buy, Price(510), Qt(2));
    place(&mut book, Side::Buy, Price(490), Qt(3));

    assert_eq!(best_bid(&book), Some((Price(510), highest, Qt(2))));
}

#[test]
fn best_ask_is_lowest_price() {
    let mut book = OrderBook::new();
    place(&mut book, Side::Sell, Price(500), Qt(1));
    place(&mut book, Side::Sell, Price(510), Qt(2));
    let lowest = place(&mut book, Side::Sell, Price(490), Qt(3));

    assert_eq!(best_ask(&book), Some((Price(490), lowest, Qt(3))));
}

#[test]
fn sides_do_not_mix() {
    let mut book = OrderBook::new();
    let bid = place(&mut book, Side::Buy, Price(400), Qt(1));
    let ask = place(&mut book, Side::Sell, Price(600), Qt(2));

    assert_eq!(best_bid(&book), Some((Price(400), bid, Qt(1))));
    assert_eq!(best_ask(&book), Some((Price(600), ask, Qt(2))));
}

// level

#[test]
fn level_at_missing_price_is_empty() {
    let mut book = OrderBook::new();
    place(&mut book, Side::Buy, Price(500), Qt(1));

    assert_eq!(ids_at(&book, Side::Buy, Price(501)), vec![]);
}

#[test]
fn level_on_other_side_is_empty() {
    let mut book = OrderBook::new();
    place(&mut book, Side::Buy, Price(500), Qt(1));

    assert_eq!(ids_at(&book, Side::Sell, Price(500)), vec![]);
}

#[test]
fn levels_are_independent() {
    let mut book = OrderBook::new();
    let at_500 = place(&mut book, Side::Buy, Price(500), Qt(1));
    let at_510 = place(&mut book, Side::Buy, Price(510), Qt(2));

    assert_eq!(ids_at(&book, Side::Buy, Price(500)), vec![at_500]);
    assert_eq!(ids_at(&book, Side::Buy, Price(510)), vec![at_510]);
}

#[test]
fn extreme_prices_are_stored() {
    let mut book = OrderBook::new();
    let low = place(&mut book, Side::Buy, Price(1), Qt(1));
    let high = place(&mut book, Side::Buy, Price(u64::MAX), Qt(1));

    assert_eq!(best_bid(&book), Some((Price(u64::MAX), high, Qt(1))));
    assert_eq!(ids_at(&book, Side::Buy, Price(1)), vec![low]);
}

// submit (matching)

/// Base book, built with `submit` (nothing crosses):
/// Buy 4 @ 99 -> Id(0), Sell 5 @ 100 -> Id(1), Sell 2 @ 100 -> Id(2), Sell 3 @ 101 -> Id(3).
fn base_book() -> OrderBook {
    let mut book = OrderBook::new();
    place(&mut book, Side::Buy, Price(99), Qt(4));
    place(&mut book, Side::Sell, Price(100), Qt(5));
    place(&mut book, Side::Sell, Price(100), Qt(2));
    place(&mut book, Side::Sell, Price(101), Qt(3));
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

    let (id, trades) = book.submit(Side::Buy, Price(101), Qt(9)).unwrap();

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

    let (id, trades) = book.submit(Side::Sell, Price(98), Qt(6)).unwrap();

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

    let (id, trades) = book.submit(Side::Buy, Price(100), Qt(5)).unwrap();

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

    let (id, trades) = book.submit(Side::Sell, Price(100), Qt(2)).unwrap();

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
    book.submit(Side::Buy, Price(101), Qt(9)).unwrap();

    let (id, trades) = book.submit(Side::Buy, Price(101), Qt(1)).unwrap();

    assert_eq!(id, Id(5));
    assert_eq!(trades, vec![trade(3, 5, 101, 1)]);
    assert_eq!(best_ask(&book), None);
}

#[test]
fn sell_sweeps_two_bid_levels() {
    let mut book = OrderBook::new();
    place(&mut book, Side::Buy, Price(99), Qt(5));
    place(&mut book, Side::Buy, Price(98), Qt(5));

    let (id, trades) = book.submit(Side::Sell, Price(98), Qt(8)).unwrap();

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

    let (id, trades) = book.submit(Side::Sell, Price(99), Qt(4)).unwrap();

    assert_eq!(id, Id(4));
    assert_eq!(trades, vec![trade(0, 4, 99, 4)]);
    assert_eq!(best_bid(&book), None);
    assert_eq!(orders_at(&book, Side::Sell, Price(99)), vec![]);
    assert_eq!(best_ask(&book), Some((Price(100), Id(1), Qt(5))));
}

#[test]
fn partially_filled_maker_stays_at_front() {
    let mut book = base_book();

    let (id, trades) = book.submit(Side::Buy, Price(100), Qt(3)).unwrap();

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

    let (id, trades) = book.submit(Side::Buy, Price(105), Qt(12)).unwrap();

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
    book.submit(Side::Sell, Price(98), Qt(6)).unwrap();

    let (id, trades) = book.submit(Side::Buy, Price(98), Qt(2)).unwrap();

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

    let (id, trades) = book.submit(Side::Buy, Price(500), Qt(10)).unwrap();

    assert_eq!(id, Id(0));
    assert_eq!(trades, vec![]);
    assert_eq!(best_bid(&book), Some((Price(500), Id(0), Qt(10))));
    assert_eq!(best_ask(&book), None);
}

#[test]
fn sell_on_empty_book_rests() {
    let mut book = OrderBook::new();

    let (id, trades) = book.submit(Side::Sell, Price(500), Qt(10)).unwrap();

    assert_eq!(id, Id(0));
    assert_eq!(trades, vec![]);
    assert_eq!(best_ask(&book), Some((Price(500), Id(0), Qt(10))));
    assert_eq!(best_bid(&book), None);
}

#[test]
fn buy_without_cross_joins_back_of_queue() {
    let mut book = base_book();

    let (id, trades) = book.submit(Side::Buy, Price(99), Qt(1)).unwrap();

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

    let (id, _) = book.submit(Side::Buy, Price(100), Qt(5)).unwrap();

    assert_eq!(id, Id(4));
    assert_eq!(orders_at(&book, Side::Buy, Price(100)), vec![]);
    assert_eq!(place(&mut book, Side::Sell, Price(200), Qt(1)), Id(5));
    assert_eq!(book.submit(Side::Buy, Price(50), Qt(1)).unwrap().0, Id(6));
}

#[test]
fn buy_exhausts_ask_side() {
    let mut book = base_book();

    let (id, trades) = book.submit(Side::Buy, Price(101), Qt(12)).unwrap();

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

// submit (validation)

#[test]
fn rejects_zero_quantity() {
    let mut book = OrderBook::new();

    assert_eq!(
        book.submit(Side::Buy, Price(100), Qt(0)),
        Err(Reject::ZeroQuantity)
    );
}

#[test]
fn rejects_zero_price() {
    let mut book = OrderBook::new();

    assert_eq!(
        book.submit(Side::Buy, Price(0), Qt(10)),
        Err(Reject::ZeroPrice)
    );
}

#[test]
fn quantity_is_checked_before_price() {
    let mut book = OrderBook::new();

    assert_eq!(
        book.submit(Side::Buy, Price(0), Qt(0)),
        Err(Reject::ZeroQuantity)
    );
}

#[test]
fn rejected_order_does_not_consume_id() {
    let mut book = OrderBook::new();

    assert!(book.submit(Side::Buy, Price(100), Qt(0)).is_err());
    assert_eq!(
        book.submit(Side::Buy, Price(100), Qt(10)),
        Ok((Id(0), vec![]))
    );
}

#[test]
fn rejected_order_does_not_change_book() {
    let mut book = base_book();

    assert!(book.submit(Side::Sell, Price(100), Qt(0)).is_err());

    assert_eq!(best_bid(&book), Some((Price(99), Id(0), Qt(4))));
    assert_eq!(best_ask(&book), Some((Price(100), Id(1), Qt(5))));
    assert_eq!(
        orders_at(&book, Side::Sell, Price(100)),
        vec![(Id(1), Qt(5)), (Id(2), Qt(2))]
    );
}
