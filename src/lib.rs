//! Single-market matching engine: takes a sequence of commands and returns a
//! sequence of events.

use std::collections::{BTreeMap, VecDeque, btree_map::OccupiedEntry};

pub mod parser;

/// Side of an order: buy (bid) or sell (ask).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// Buy order, rests on the bid side.
    Buy,
    /// Sell order, rests on the ask side.
    Sell,
}

/// Price in cents (2 decimal places as an integer): `Price(9950)` is 99.50.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Price(pub u64);

/// Quantity, in whole units.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Qt(pub u64);

/// Order id, assigned sequentially by the book to each accepted order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Id(pub u64);

/// An order with all of its fields: side, price, quantity and id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Order {
    /// Side of the order.
    pub side: Side,
    /// Limit price.
    pub price: Price,
    /// Quantity.
    pub qt: Qt,
    /// Id of the order.
    pub id: Id,
}

impl Order {
    /// Builds an order from its fields.
    pub fn new(side: Side, price: Price, qt: Qt, id: Id) -> Self {
        Order {
            side,
            price,
            qt,
            id,
        }
    }
}

/// Queue entry of a price level; side and price come from the structure, so they are not stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RestingOrder {
    id: Id,
    qt: Qt,
}

impl RestingOrder {
    /// Id of the order.
    pub fn id(&self) -> Id {
        self.id
    }

    /// Remaining quantity of the order.
    pub fn qt(&self) -> Qt {
        self.qt
    }
}

/// One trade per maker/taker pair; `price` is always the maker's price.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Trade {
    /// Order that was resting in the book.
    pub maker: Id,
    /// Incoming order that crossed the book.
    pub taker: Id,
    /// Execution price (the maker's price).
    pub price: Price,
    /// Quantity traded.
    pub qt: Qt,
}

/// Why an order was rejected before entering the engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reject {
    /// Quantity is zero.
    ZeroQuantity,
    /// Price is zero.
    ZeroPrice,
}

/// Why a cancel was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancelReject {
    /// The id was never issued.
    UnknownId,
    /// The order already left the book by trading completely.
    AlreadyFilled,
    /// The order was already cancelled.
    AlreadyCancelled,
}

/// Lifecycle status of an order, as shown by [`OrderBook::view`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// Still in the book, possibly partially filled.
    Resting,
    /// Traded completely.
    Filled,
    /// Cancelled, possibly after a partial fill.
    Cancelled,
}

/// Snapshot of one order, returned by [`OrderBook::view`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrderView {
    /// Id assigned when the order was accepted.
    pub id: Id,
    /// Side the order was submitted on.
    pub side: Side,
    /// Limit price of the order (not the price of its trades).
    pub price: Price,
    /// Quantity the order was submitted with.
    pub original: Qt,
    /// Quantity already traded: `original - remaining` while resting, everything that
    /// traded before the cancel once cancelled.
    pub executed: Qt,
    /// Quantity still resting in the book; 0 once filled or cancelled.
    pub remaining: Qt,
    /// Where the order is in its lifecycle.
    pub status: Status,
}

/// Why a view was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewError {
    /// The id was never issued.
    UnknownId,
}

// Fixed order data always stays in the record; executed and remaining are derived by
// subtraction. The only quantity that changes on each trade is the queue entry's qt.
#[derive(Debug, Clone, Copy)]
struct OrderRecord {
    side: Side,
    price: Price,
    original: Qt,
    state: OrderState,
}

#[derive(Debug, Clone, Copy)]
enum OrderState {
    Resting,
    Filled,
    Cancelled { remaining: Qt },
}

/// Limit order book for a single market, with price-time priority matching.
#[derive(Debug, Clone)]
pub struct OrderBook {
    bids: BTreeMap<Price, VecDeque<RestingOrder>>,
    asks: BTreeMap<Price, VecDeque<RestingOrder>>,
    // One record per accepted order, positioned by Id.0.
    index: Vec<OrderRecord>,
}

impl OrderBook {
    /// Creates an empty book.
    pub fn new() -> Self {
        OrderBook {
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            index: Vec::new(),
        }
    }

    // Appends to the back of the level's queue (time priority); does not touch the index.
    fn rest(&mut self, id: Id, side: Side, price: Price, qt: Qt) {
        let levels = match side {
            Side::Buy => &mut self.bids,
            Side::Sell => &mut self.asks,
        };
        levels
            .entry(price)
            .or_default()
            .push_back(RestingOrder { id, qt });
    }

    /// Best buy level (highest price) and the first order in its queue.
    pub fn best_bid(&self) -> Option<(Price, RestingOrder)> {
        self.bids
            .last_key_value()
            .and_then(|(price, queue)| queue.front().map(|order| (*price, *order)))
    }

    /// Best sell level (lowest price) and the first order in its queue.
    pub fn best_ask(&self) -> Option<(Price, RestingOrder)> {
        self.asks
            .first_key_value()
            .and_then(|(price, queue)| queue.front().map(|order| (*price, *order)))
    }

    /// Orders at the given side and price, oldest first; empty if the level does not exist.
    pub fn level(&self, side: Side, price: Price) -> impl Iterator<Item = &RestingOrder> {
        let levels = match side {
            Side::Buy => &self.bids,
            Side::Sell => &self.asks,
        };
        levels.get(&price).into_iter().flatten()
    }

    /// Takes a limit order, matches it against the opposite side and rests whatever is
    /// left in the book; returns the order's id and the trades in the order they happened.
    ///
    /// An invalid order is rejected before anything else: it gets no id and leaves the
    /// book untouched.
    pub fn submit(&mut self, side: Side, price: Price, qt: Qt) -> Result<(Id, Vec<Trade>), Reject> {
        // Validate first, so a rejected order consumes no id and changes no state.
        // Quantity is checked before price.
        if qt.0 == 0 {
            return Err(Reject::ZeroQuantity);
        }
        if price.0 == 0 {
            return Err(Reject::ZeroPrice);
        }

        // The record is pushed after matching, so this is the next free slot.
        let id = Id(self.index.len() as u64);
        let mut remaining = qt;
        let mut trades = Vec::new();

        // Match against the other side: a field borrow, so self.index stays usable.
        let opposite = match side {
            Side::Buy => &mut self.asks,
            Side::Sell => &mut self.bids,
        };

        // Best price first; an empty side ends matching.
        while let Some(mut level) = best_level(opposite, side) {
            let level_price = *level.key();
            // Price priority: stop at the first level that does not cross.
            if !crosses(side, level_price, price) {
                break;
            }
            let queue = level.get_mut();
            // Defensive: an empty level should not exist; drop it instead of panicking.
            let Some(maker) = queue.front_mut() else {
                level.remove();
                continue;
            };
            // Time priority: the oldest maker at this level fills first.
            let fill = remaining.0.min(maker.qt.0);
            // The maker's price is the trade price.
            trades.push(Trade {
                maker: maker.id,
                taker: id,
                price: level_price,
                qt: Qt(fill),
            });
            remaining.0 -= fill;
            maker.qt.0 -= fill;
            if maker.qt.0 == 0 {
                // A filled maker leaves the book; its record keeps the history.
                let maker_id = maker.id;
                queue.pop_front();
                if let Some(record) = self.index.get_mut(maker_id.0 as usize) {
                    record.state = OrderState::Filled;
                }
            }
            // No empty levels, so best_bid/best_ask stay correct.
            if queue.is_empty() {
                level.remove();
            }
            if remaining.0 == 0 {
                break;
            }
        }

        // Whatever is left rests at the order's own limit price.
        let state = if remaining.0 > 0 {
            self.rest(id, side, price, remaining);
            OrderState::Resting
        } else {
            OrderState::Filled
        };
        // Every accepted order gets a record, already with its final state.
        self.index.push(OrderRecord {
            side,
            price,
            original: qt,
            state,
        });
        Ok((id, trades))
    }

    /// Removes a resting order from the book and returns its remaining quantity.
    ///
    /// Errors: [`CancelReject::UnknownId`] if the id was never issued,
    /// [`CancelReject::AlreadyFilled`] if the order traded completely and
    /// [`CancelReject::AlreadyCancelled`] if it was cancelled before. Cancelling issues no
    /// id, produces no trades and leaves every other order untouched.
    pub fn cancel(&mut self, id: Id) -> Result<Qt, CancelReject> {
        let Some(&OrderRecord {
            side, price, state, ..
        }) = self.index.get(id.0 as usize)
        else {
            return Err(CancelReject::UnknownId);
        };
        match state {
            OrderState::Resting => {}
            OrderState::Filled => return Err(CancelReject::AlreadyFilled),
            OrderState::Cancelled { .. } => return Err(CancelReject::AlreadyCancelled),
        }

        // Linear search within the level; the record only knows side and price.
        let pos = self.position_in_level(side, price, id);
        let levels = match side {
            Side::Buy => &mut self.bids,
            Side::Sell => &mut self.asks,
        };
        let removed = pos.and_then(|pos| levels.get_mut(&price)?.remove(pos));
        let Some(order) = removed else {
            // The record says the order is resting here, so this is a broken invariant.
            debug_assert!(
                false,
                "record says order {} is resting at {side:?} {}, but it is not in that level",
                id.0, price.0
            );
            return Err(CancelReject::AlreadyFilled);
        };

        // No empty levels, so best_bid/best_ask stay correct.
        if levels.get(&price).is_some_and(|queue| queue.is_empty()) {
            levels.remove(&price);
        }
        // Keep what was left, so view can derive the executed quantity.
        if let Some(record) = self.index.get_mut(id.0 as usize) {
            record.state = OrderState::Cancelled {
                remaining: order.qt,
            };
        }
        Ok(order.qt)
    }

    /// Current state of an order: its fixed data plus executed and remaining quantities.
    ///
    /// Error: [`ViewError::UnknownId`] if the id was never issued.
    pub fn view(&self, id: Id) -> Result<OrderView, ViewError> {
        let &OrderRecord {
            side,
            price,
            original,
            state,
        } = self.index.get(id.0 as usize).ok_or(ViewError::UnknownId)?;

        // `left` is what did not trade: in the book while resting, cancelled otherwise.
        let (status, left, remaining) = match state {
            OrderState::Resting => {
                let in_queue = self
                    .position_in_level(side, price, id)
                    .and_then(|pos| self.level(side, price).nth(pos))
                    .map(|order| order.qt);
                let Some(qt) = in_queue else {
                    // The record says the order is resting here, so this is a broken invariant.
                    debug_assert!(
                        false,
                        "record says order {} is resting at {side:?} {}, but it is not in that level",
                        id.0, price.0
                    );
                    return Ok(OrderView {
                        id,
                        side,
                        price,
                        original,
                        executed: original,
                        remaining: Qt(0),
                        status: Status::Resting,
                    });
                };
                (Status::Resting, qt, qt)
            }
            OrderState::Filled => (Status::Filled, Qt(0), Qt(0)),
            OrderState::Cancelled { remaining } => (Status::Cancelled, remaining, Qt(0)),
        };

        Ok(OrderView {
            id,
            side,
            price,
            original,
            // Saturates instead of wrapping in release if the invariant ever breaks.
            executed: Qt(original.0.saturating_sub(left.0)),
            remaining,
            status,
        })
    }

    // Position of `id` in the queue of level (side, price); None if it is not there.
    fn position_in_level(&self, side: Side, price: Price, id: Id) -> Option<usize> {
        self.level(side, price).position(|order| order.id == id)
    }
}

impl Default for OrderBook {
    fn default() -> Self {
        Self::new()
    }
}

/// Best level of the side a `taker` matches against: lowest ask for a buy, highest bid for a sell.
fn best_level(
    levels: &mut BTreeMap<Price, VecDeque<RestingOrder>>,
    taker: Side,
) -> Option<OccupiedEntry<'_, Price, VecDeque<RestingOrder>>> {
    match taker {
        Side::Buy => levels.first_entry(),
        Side::Sell => levels.last_entry(),
    }
}

/// Whether a `taker` with limit `limit` trades at `level_price`.
fn crosses(taker: Side, level_price: Price, limit: Price) -> bool {
    match taker {
        Side::Buy => level_price <= limit,
        Side::Sell => level_price >= limit,
    }
}
