//! Single-market matching engine: takes a sequence of commands and returns a
//! sequence of events.

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Order {
    pub side: Side,
    pub price: Price,
    pub qt: Qt,
}

impl Order {
    pub fn new(side: Side, price: Price, qt: Qt) -> Self {
        Order { side, price, qt }
    }
}
