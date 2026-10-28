//! Single-market matching engine: takes a sequence of commands and returns a
//! sequence of events.

use std::collections::{BTreeMap, VecDeque};

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
struct RestingOrder {
    id: Id,
    qt: Qt,
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

    /// Puts the order in the book without trying to match it and without validating it.
    ///
    /// Validation and the `Result` return type come later; this becomes private once
    /// matching exists.
    pub fn insert(&mut self, side: Side, price: Price, qt: Qt) -> Id {
        let id = Id(self.index.len() as u64);
        let levels = match side {
            Side::Buy => &mut self.bids,
            Side::Sell => &mut self.asks,
        };
        levels
            .entry(price)
            .or_default()
            .push_back(RestingOrder { id, qt });
        self.index.push(Some((side, price)));
        id
    }
}

impl Default for OrderBook {
    fn default() -> Self {
        Self::new()
    }
}
