# Auction Simulator

An interactive auction theory simulator written in Rust. Compete against AI bidders across eight classic mechanisms, read the theory before you bid, and get a full debrief afterward that explains what happened and why.

Available as both a terminal app (TUI) and a browser app (Yew/WASM, deployed to GitHub Pages).

**[Play in the browser →](https://tieronetigg.github.io/auction-sim/)**

---

## Auction types

| # | Format | Key concept |
|---|--------|-------------|
| EN | English | Open ascending price — stay in below your value, drop out above it |
| DU | Dutch | Descending clock — first caller wins; strategically identical to FPSB |
| FP | First-Price Sealed-Bid | Pay your own bid; optimal strategy is to shade downward |
| VK | Vickrey | Pay the second-highest bid; truthful bidding is dominant |
| AP | All-Pay | Everyone pays their bid; models lobbying, litigation, R&D races |
| DA | Double Auction | Two-sided market; buyers and sellers clear at a uniform price |
| CB | Combinatorial | Bundle bids on multiple items; winner-determination by welfare max |
| VG | VCG Mechanism | Pay your externality; strategy-proof and efficient |

Each type has:
- A theory intro screen with the key result and simulation parameters
- A live round against AI bidders with realistic strategies
- A debrief screen that reveals all values, explains the outcome, and highlights the relevant economic insight

---

## Running locally

### Terminal app

```bash
cargo run -p auction-tui
```

Requires a terminal with 256-colour support. Navigate with arrow keys; bid by typing an amount and pressing Enter. Dutch auction: press Enter or Space to call at the current clock price.

### Web app (Yew/WASM)

```bash
cargo install trunk
trunk serve crates/auction-web/index.html
```

Then open `http://localhost:8080`.

### Headless smoke-test

```bash
cargo run -p auction-engine --example english_sim
```

---

## Workspace layout

```
crates/
├── auction-core/       Domain logic — mechanisms, bids, outcomes, events
├── auction-ai/         AI bidder strategies (truthful, shading, BNE all-pay, seller)
├── auction-engine/     Simulation driver — tick loop, AI scheduling, event log
├── auction-education/  Debrief analysis and theory hints
├── auction-tui/        Terminal UI (ratatui + crossterm)
└── auction-web/        Browser UI (Yew + Trunk, deployed to GitHub Pages)
```

`auction-core` has zero I/O and no workspace dependencies — everything else builds on it.

---

## AI strategies

| Mechanism | AI strategy |
|-----------|-------------|
| English | Truthful — stay in below value |
| Dutch | Truthful — call at own value |
| FPSB | Shading — bid 0.80 × value (≈ BNE for n = 6) |
| Vickrey | Truthful |
| All-Pay | BNE formula — b(v) = (n−1)/n · v · (v/H)^(n−1) |
| Double (buyers) | Truthful |
| Double (sellers) | Truthful ask at own value |
| Combinatorial / VCG | Fixed sealed bids submitted at game start |

---

## Build status

| Phase | Status | Description |
|-------|--------|-------------|
| 1 | ✅ | English auction, truthful AI, headless sim |
| 2 | ✅ | TUI shell — menu, event loop |
| 3 | ✅ | First playable auction (English) |
| 4 | ✅ | Dutch, FPSB, Vickrey |
| 5 | ✅ | Education layer — hints, debrief analysis, sparkline |
| 6 | ✅ | All-Pay + Double auction |
| 7 | ✅ | Combinatorial + VCG |
| 8 | — | Persistence + session history + replay |
| 9 | — | SMRA (stretch) |

---

## Tech

- Rust 2021 edition, MSRV 1.76
- TUI: [ratatui](https://github.com/ratatui-org/ratatui) + [crossterm](https://github.com/crossterm-rs/crossterm)
- Web: [Yew](https://yew.rs) + [Trunk](https://trunkrs.dev), compiled to WASM
- Deployed via GitHub Actions to GitHub Pages on every push to `main`
