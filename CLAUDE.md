# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

Interactive auction theory simulator written in Rust. Users compete against AI bidders in
simulations that illustrate mechanisms from simple to advanced, with live AI bidders,
per-type theory introductions, and post-round debriefs that reveal values and explain
key results (revenue equivalence, bid shading, truth dominance, etc.).

## Commands

```bash
# Build all crates
cargo build

# Run the headless English auction smoke-test
cargo run -p auction-engine --example english_sim

# Run the interactive TUI
cargo run -p auction-tui

# Check / test / lint / format
cargo check
cargo test
cargo clippy
cargo fmt
```

## Workspace Structure

```
crates/
├── auction-core/         Pure domain logic — no I/O, no workspace dependencies
│   ├── src/types.rs      Money, BidderId, ItemId, SimTime, AuctionType, AuctionPhase
│   ├── src/mechanism.rs  Auction trait + VisibleAuctionState
│   ├── src/bidder.rs     BidderStrategy trait
│   ├── src/item.rs       Item (name, reserve_price)
│   ├── src/bid.rs        Bid, BidRecord, BidError (incl. AlreadyBid)
│   ├── src/outcome.rs    AuctionOutcome, Allocation, Payment, Receipt
│   ├── src/event.rs      AuctionEvent (#[non_exhaustive]) incl. AskSubmitted
│   └── src/auction/
│       ├── english.rs    EnglishAuction — open ascending, activity_timeout
│       ├── dutch.rs      DutchAuction — descending clock, first caller wins
│       ├── sealed_bid.rs SealedBidAuction — deadline-based, FirstPrice|SecondPrice
│       ├── all_pay.rs    AllPayAuction — everyone pays; highest bid wins
│       └── double.rs     DoubleAuction — k-DA two-sided market, uniform clearing price
│
├── auction-ai/           AI bidder strategies
│   ├── src/truthful.rs   TruthfulBidder — correct for English, Dutch, Vickrey, Double buyers
│   ├── src/shading.rs    BidShadingBidder — bids value × shade_factor (FPSB)
│   ├── src/all_pay.rs    AllPayBidder — BNE equilibrium formula b(v)=(n-1)/n·v·(v/H)^(n-1)
│   └── src/seller.rs     TruthfulSellerBidder — submits true value as ask (Double only)
│
├── auction-engine/       Simulation driver
│   ├── src/engine.rs     SimulationEngine (Box<dyn Auction>, event log, AI loop)
│   └── examples/
│       └── english_sim.rs  Headless 5-bidder English auction smoke-test
│
├── auction-education/    Concept pages, hints, debrief analysis (Phase 5)
│
└── auction-tui/          Binary: auction-sim
    ├── src/terminal.rs   Raw-mode setup/teardown + panic hook
    ├── src/app.rs        App, Screen enum, tick loop, KeyEffect transition pattern
    └── src/screens/
        ├── mod.rs        Render dispatch + Placeholder screen
        ├── menu.rs       Main menu — EN/DU/FP/VK/AP/DA available; CB/VG coming later
        ├── intro.rs      Scrollable theory intro; AuctionType-aware (6 bodies + colours)
        ├── auction.rs    Live auction — per-type render, 6 game constructors, human input
        └── debrief.rs    Result + per-type theory note; reveals sealed bids + asks
```

## Architecture

### Domain layer (auction-core)

**auction-core** has zero I/O and depends on no other workspace crate. Everything else
depends on it.

The `Auction` trait is the central interface:

```rust
pub trait Auction {
    fn auction_type(&self) -> AuctionType;
    fn phase(&self) -> AuctionPhase;
    fn item_id(&self) -> ItemId;
    fn item_name(&self) -> &str;
    fn visible_state(&self) -> VisibleAuctionState;
    fn submit_bid(&mut self, bid: Bid) -> Result<Vec<AuctionEvent>, BidError>;
    fn tick(&mut self, delta: SimTime) -> Vec<AuctionEvent>;
    fn outcome(&self) -> Option<&AuctionOutcome>;
}
```

**VisibleAuctionState** encodes what each bidder can legitimately observe. Open formats
(English, Dutch) expose `current_price`; sealed formats set it to `None`. The
`deadline_remaining` field is `Some(secs)` only for sealed-bid auctions.

**AuctionEvent** is `#[non_exhaustive]` — always include a `_ => {}` catch-all when
matching. Key variants:
- `BidAccepted { bid, new_standing }` — English/Dutch public bids
- `BidSubmitted(Bid)` — sealed-bid acknowledgment (amount private until reveal)
- `AskSubmitted(Bid)` — Double auction seller order acknowledgment
- `PriceChanged { old, new }` — Dutch clock ticks / English price jumps
- `AuctionClosed` + `AllocationDecided(AuctionOutcome)` — emitted together at close

**Receipt** in `AuctionOutcome.receipts` — seller-side payments in Double auction;
empty `vec![]` for all single-sided auctions.

**BidError::AlreadyBid** is returned by `SealedBidAuction` when a bidder submits twice.
The engine ignores this error silently.

### Simulation engine (auction-engine)

`SimulationEngine` holds a `Box<dyn Auction>`, a vector of `BidderConfig` (each with a
`Box<dyn BidderStrategy>`), and the full event log. Key methods:

- `tick(delta)` — advances auction clock, gives each AI bidder a chance to act (gated by
  `think_time`), logs all events.
- `submit_bid_for(id, amount)` — human bid path; stamps timestamp automatically.
- `stagger_starts()` — spreads AI first-action times evenly across one `think_time` window
  so bidders don't all fire at `t=0`.
- `run_to_completion()` — headless loop for the example/test harness.

Access the item via `engine.auction.item_id()` and `engine.auction.item_name()` — not
direct field access (the field is behind the trait object).

### TUI (auction-tui)

`App` owns a `Screen` enum and drives a 60 fps loop (`main.rs`). Screen variants:
`MainMenu → AuctionIntro → LiveAuction → Debrief → Placeholder`.

Key patterns:
- **KeyEffect** — `key_transition()` returns `KeyEffect::GoTo(screen)` or `Quit` while
  still borrowing `self.screen`; the caller applies it after the borrow ends.
- **LiveAuction → Debrief transition** — `std::mem::replace` takes ownership of the live
  state so `build_debrief` can consume it without a double-borrow.
- **Per-mechanism input** — `app.rs` dispatches to `handle_english_key`,
  `handle_dutch_key`, or `handle_sealed_key` based on `state.auction_type`.
- **Dutch input** — Enter / Space calls at the current clock price (no typing needed).
- **Sealed bid input** — Human types an amount, presses Enter once; `bid_submitted` flag
  prevents re-entry. Auction rejects duplicates with `AlreadyBid` anyway.

`IntroState::new(AuctionType)` selects the body text and border colour for the theory
intro screen. `DebriefState` carries `accepted_bids` (from `BidAccepted` events,
English/Dutch), `sealed_bids` (from `BidSubmitted`, FPSB/Vickrey/AllPay), and
`sealed_asks` (from `AskSubmitted`, Double auction sellers).

**Double auction TUI conventions:**
- `is_double_seller(id: BidderId) -> bool { id.0 >= 4 }` — IDs 0-3 are buyers, 4-7 sellers.
- The human is always buyer `BidderId(0)`.
- `render_double()` splits the screen 50/50 horizontally: buyers left, sellers right.

## Build Phases

| Phase | Status | Description |
|-------|--------|-------------|
| 1 | ✅ Done | Workspace scaffold, EnglishAuction, TruthfulBidder, english_sim example |
| 2 | ✅ Done | TUI shell — ratatui + crossterm, main menu, event loop |
| 3 | ✅ Done | First playable auction (English, live human bid input) |
| 4 | ✅ Done | Dutch, FPSB, Vickrey — all playable with per-type intro/debrief |
| 5 | ✅ Done | Education layer: inline hints, debrief analysis, sparkline chart |
| 6 | ✅ Done | All-pay + Double auction (k-DA two-sided market) |
| 7 | Next   | Combinatorial + VCG |
| 8 | —      | Persistence + session history + replay |
| 9 | —      | SMRA (stretch) |

## Conventions

### Types and values
- All monetary values use the `Money(f64)` newtype. Never use raw `f64` for prices.
- `BidderId(0)` is always the human player; AI bidders start at `BidderId(1)`.
- Bid timestamps are set by `SimulationEngine` just before `submit_bid` — leave them
  as `0.0` inside `BidderStrategy::decide`.

### Adding a new auction type
1. Create `auction-core/src/auction/<type>.rs` implementing the full `Auction` trait
   (including `item_id` and `item_name`).
2. Add the module to `auction-core/src/auction/mod.rs`.
3. Add an AI strategy in `auction-ai/src/<strategy>.rs` if needed.
4. Add a `new_<type>_game()` constructor in `auction-tui/src/screens/auction.rs`.
5. Add intro text + `border_color` case in `intro.rs`.
6. Add a theory note section in `debrief.rs`.
7. Wire the menu entry (`auction_type: Some(AuctionType::...)`) and mark `available: true`.
8. Add a key handler case in `app.rs` if input differs from English.

### Compatibility
- Rust edition: 2021. MSRV: 1.76. Use `rand = "0.8"` (not 0.9/0.10).
- `unicode-segmentation` is pinned to `=1.12.0` in `Cargo.lock` (1.13+ requires Rust 1.85).
  If `cargo update` bumps it, re-pin: `cargo update unicode-segmentation --precise 1.12.0`

### Simulation parameters (current games)
| Game | Item | Human value | AI think_time | Notes |
|------|------|-------------|---------------|-------|
| English | Watch | $350 | 3.0s | 25s silence timeout, $10 increment |
| Dutch | Watch | $350 | 0.3s | $550 start, $8/s drop, $50 floor |
| FPSB | Watch | $350 | 3.0s | 30s deadline; Alice/Carol/Dave shade 0.80× = (n-1)/n BNE |
| Vickrey | Watch | $350 | 3.0s | 30s deadline; all AI bid truthfully |
| AllPay | Watch | $350 | 3.0s | 30s deadline; AI use BNE formula n=6, H=$500 |
| Double | Research Report | $100 | 3.0s | 45s deadline; 3 buyers + 4 sellers, k=0.5 |

AI values (English/Dutch/FPSB/Vickrey/AllPay): Alice $420, Bob $380, Carol $310, Dave $450, Eve $290.

Double auction participants (IDs 0-3 buyers, 4-7 sellers):
- Buyers: Human $100, Alice $120, Bob $110, Carol $90
- Sellers: Dave (ask $35), Eve (ask $60), Fiona (ask $80), Grant (ask $105)
- Expected outcome: 3 trades at ~$90 clearing price (Dave/Eve/Fiona matched with Human/Alice/Bob)
