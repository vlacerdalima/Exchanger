//! Single-market matching engine: takes a sequence of commands and returns a
//! sequence of events.

use std::collections::{BTreeMap, VecDeque, btree_map::OccupiedEntry};

pub mod parser;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Price(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Qt(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Id(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Order {
    pub side: Side,
    pub price: Price,
    pub qt: Qt,
    pub id: Id,
}

impl Order {
    pub fn new(side: Side, price: Price, qt: Qt, id: Id) -> Self {
        Order {
            side,
            price,
            qt,
            id,
        }
    }
}

// Queue entry of a price level; side and price come from the structure, so they are not stored.
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

// One trade per maker/taker pair; `price` is always the maker's price.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Trade {
    pub maker: Id,
    pub taker: Id,
    pub price: Price,
    pub qt: Qt,
}

// Why an order was rejected before entering the engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reject {
    ZeroQuantity,
    ZeroPrice,
}

#[derive(Debug, Clone)]
pub struct OrderBook {
    bids: BTreeMap<Price, VecDeque<RestingOrder>>,
    asks: BTreeMap<Price, VecDeque<RestingOrder>>,
    // Support index, positioned by Id.0; None = order that already left the book.
    index: Vec<Option<(Side, Price)>>,
}

impl OrderBook {
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

        let id = Id(self.index.len() as u64);
        // Every order gets an index slot, even if it fills completely.
        self.index.push(None);
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
                // A filled maker leaves the book and the index.
                let maker_id = maker.id;
                queue.pop_front();
                if let Some(slot) = self.index.get_mut(maker_id.0 as usize) {
                    *slot = None;
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
        if remaining.0 > 0 {
            self.rest(id, side, price, remaining);
            if let Some(slot) = self.index.get_mut(id.0 as usize) {
                *slot = Some((side, price));
            }
        }
        Ok((id, trades))
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
