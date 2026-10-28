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

// TODO: remove once the operations exist.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct OrderBook {
    bids: BTreeMap<Price, VecDeque<Order>>,
    asks: BTreeMap<Price, VecDeque<Order>>,
    // Support index, positioned by Id.0; None = order that already left the book.
    index: Vec<Option<(Side, Price)>>,
    next_id: u64,
}

impl OrderBook {
    pub fn new() -> Self {
        OrderBook {
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            index: Vec::new(),
            next_id: 0,
        }
    }
}

impl Default for OrderBook {
    fn default() -> Self {
        Self::new()
    }
}
