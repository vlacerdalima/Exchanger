# matching-engine

[![CI](https://github.com/vlacerdalima/Exchanger/actions/workflows/ci.yml/badge.svg)](https://github.com/vlacerdalima/Exchanger/actions/workflows/ci.yml)

A single-market limit order book in Rust: it reads commands from a file, matches orders with
price-time priority and prints one line per event.

## Quick start

```sh
cargo run -- examples/basic.txt   # run the example
cargo test                         # library, parser and CLI tests
```

Requires Rust 1.85 or later (edition 2024). The input has one command per line:

```
BID|ASK QTY @ PRICE    submit a limit order (BID = buy, ASK = sell)
CANCEL ID              cancel a resting order
VIEW ID                show an order's quantities and status
```

Keywords are uppercase. Quantities are unsigned integers; prices have at most 2 decimal places.
Blank lines are skipped. Lines that fail to parse are reported on stderr as `line N: <reason>`;
everything else goes to stdout.

## Example

`examples/basic.txt`:

```
BID 4 @ 99
ASK 5 @ 100
ASK 2 @ 100
ASK 3 @ 101
BID 9 @ 101
CANCEL 3
ASK 6 @ 98
BID 0 @ 10
CANCEL 99
CANCEL 0
VIEW 3
VIEW 5
VIEW 1
VIEW 42
```

Output of `cargo run -q -- examples/basic.txt`:

```
ACCEPTED 0
ACCEPTED 1
ACCEPTED 2
ACCEPTED 3
ACCEPTED 4
TRADE maker=1 taker=4 qt=5 price=100.00
TRADE maker=2 taker=4 qt=2 price=100.00
TRADE maker=3 taker=4 qt=2 price=101.00
CANCELLED 3 qt=1
ACCEPTED 5
TRADE maker=0 taker=5 qt=4 price=99.00
REJECTED line 8: ZeroQuantity
CANCEL_REJECTED 99: UnknownId
CANCEL_REJECTED 0: AlreadyFilled
ORDER id=3 side=ASK price=101.00 original=3 executed=2 remaining=0 status=Cancelled
ORDER id=5 side=ASK price=98.00 original=6 executed=4 remaining=2 status=Resting
ORDER id=1 side=ASK price=100.00 original=5 executed=5 remaining=0 status=Filled
VIEW_REJECTED 42: UnknownId
```

Order 4 buys up to 101 and sweeps two ask levels; each fill trades at the resting order's price.
Ids are assigned only to accepted orders, so the rejected line 8 does not consume one.

## Design decisions

### Prices and quantities are integers
`Price` and `Qt` are newtypes over `u64`, with prices stored in cents. The parser converts the
decimal text straight to cents and never goes through `f64` (`0.29 * 100.0` is `28.999…`). Integer
arithmetic is exact, and the newtypes make passing a price where a quantity is expected a compile
error. Floating point was rejected.

### One ordered map per side, FIFO queue per level
Each side is a `BTreeMap<Price, VecDeque<RestingOrder>>`: the best bid is the last key, the best ask
the first, and the deque keeps arrival order within a price. Rejected: an array indexed by price
tick, whose size follows the price range ((max − min) × 100 slots), not the number of orders; and two
binary heaps, which cannot remove an order from the middle or list the best levels.

### Queue entries hold only id and remaining quantity
Side and price are implied by which map and which level an entry is in, so they are not stored
again. Storing the full order was rejected: it duplicates data that could diverge, and an entry
is 16 bytes instead of 32.

### Trades execute at the maker's price
A resting price is a commitment; an incoming order that crosses it trades at that price, even if
its own limit is better. Executing at the taker's price was rejected: makers would post the most
aggressive price only to gain priority.

### One matching loop for both sides
`submit` has a single loop. The side-specific parts are two small functions: `best_level` (first
entry of the asks, last entry of the bids) and `crosses` (direction of the price comparison). They
take the map, not `&mut self`, so the borrow of the opposite side does not lock the order index
that the loop also updates.

### Validation before any state change
`submit` checks quantity, then price, before assigning an id. A rejected order consumes no id and
leaves the book untouched. The side cannot be invalid: it is an enum, and the parser only produces
`Buy` or `Sell`. The parser checks syntax only; validation lives in the library, so it applies to
every caller, not just the CLI.

### Per-order record with derived quantities
An index `Vec<OrderRecord>`, positioned by id, keeps side, limit price, original quantity and state
(`Resting`, `Filled`, or `Cancelled { remaining }`). Executed and remaining quantities are derived by
subtraction; the only quantity a trade changes is the queue entry's. An earlier version stored only
`Option<(Side, Price)>` per id and could not tell a filled order from a cancelled one. Cancel uses
the record to find the level: O(1) index lookup, O(log L) level lookup, O(K) scan of the level.

## Known limitations

- One market, and only limit orders.
- The order index grows without bound: one entry per order ever accepted, never reclaimed.
- Cancel costs O(log L + K), with L price levels and K orders in the level.
- No persistence: all state lives in memory for the duration of one run.
- No performance numbers have been measured yet.

## Testing

All tests are integration tests in `tests/` and use only the public API.

- **Hand-computed scenarios.** Expected trades and book states were worked out by hand for a fixed
  base book. The matching tests were written against a `todo!()` stub and failed before `submit`
  existed.
- **CLI snapshot.** `tests/cli.rs` runs the binary on `examples/basic.txt` and compares stdout with
  an [insta](https://insta.rs) snapshot. Each new snapshot was checked line by line against the
  hand-computed output before being accepted.
- **Manual mutation testing.** Bugs were introduced on purpose to confirm that some test fails: trading
  at the taker's price, and leaving an empty level in the map. Both were caught; the second by a
  single test, because a defensive branch in the loop removes the empty level on the next iteration.
- **CI** runs `cargo fmt --check`, `cargo clippy -- -D warnings` and `cargo test`. The crate sets
  `unsafe_code = "forbid"` and warns on `unwrap()`; library code uses neither.

## Project structure

```
src/lib.rs        order book, matching, cancel, view
src/parser.rs     one input line -> Command (syntax only)
src/main.rs       CLI: reads the file, calls the book, formats events
tests/            lib.rs, parser.rs, cli.rs (one per source file) and insta snapshots
examples/         input files for the CLI
```

Public API of `OrderBook`:

- `submit(side, price, qt) -> Result<(Id, Vec<Trade>), Reject>`: validates, matches, rests the remainder.
- `cancel(id) -> Result<Qt, CancelReject>`: removes a resting order and returns its remaining quantity.
- `view(id) -> Result<OrderView, ViewError>`: original, executed and remaining quantities, plus status.
- `best_bid() / best_ask() -> Option<(Price, RestingOrder)>`: best level and the oldest order in it.
- `level(side, price) -> impl Iterator<Item = &RestingOrder>`: orders at one level, oldest first.

## Roadmap

- Order types: IOC, Market, PostOnly and FOK.
- Property-based tests that check book invariants after every operation.
- An event log with deterministic replay.
- Benchmarks, published together with the measurement conditions.

## License

MIT. See [LICENSE](LICENSE).
