/// Phase 1 smoke-test: 5 AI bidders with private values compete in an English auction.
///
/// Run with:  cargo run -p auction-engine --example english_sim
use auction_ai::truthful::TruthfulBidder;
use auction_core::auction::english::{EnglishAuction, EnglishConfig};
use auction_core::bidder::BidderStrategy;
use auction_core::event::AuctionEvent;
use auction_core::item::Item;
use auction_core::types::{BidderId, ItemId, Money};
use auction_engine::engine::{BidderConfig, SimulationEngine};

fn main() {
    println!("╔══════════════════════════════════════════════╗");
    println!("║        English Auction Simulation            ║");
    println!("╚══════════════════════════════════════════════╝");
    println!();
    println!("MECHANISM");
    println!("  Open ascending-bid (English) auction.");
    println!("  Each new bid must beat the standing price by $10.");
    println!("  Hammer falls after 30 seconds of silence.");
    println!();

    // --- Item ---
    let item = Item {
        id: ItemId(0),
        name: "Vintage Chronograph Watch".to_string(),
        reserve_price: Some(Money(80.0)),
    };
    println!("ITEM:  {} (reserve: {})", item.name, item.reserve_price.unwrap());

    // --- Bidder private values (unknown to each other) ---
    let data: &[(&str, f64)] = &[
        ("Alice", 420.0),
        ("Bob",   380.0),
        ("Carol", 310.0),
        ("Dave",  450.0),
        ("Eve",   290.0),
    ];

    println!("\nBIDDERS  (private values revealed here for study)");
    for (name, value) in data {
        println!("  {:6}  true value: {}", name, Money(*value));
    }
    println!();

    // Build the bidder list.
    let bidders: Vec<BidderConfig> = data
        .iter()
        .enumerate()
        .map(|(i, (name, value))| BidderConfig {
            id: BidderId(i as u32),
            name: name.to_string(),
            strategy: Box::new(TruthfulBidder::new(BidderId(i as u32), *name))
                as Box<dyn BidderStrategy>,
            value: Money(*value),
        })
        .collect();

    // --- Auction config ---
    let config = EnglishConfig {
        start_price: Money(100.0),
        min_increment: Money(10.0),
        activity_timeout: 30.0,
    };
    let auction = EnglishAuction::new(config, item, bidders.iter().map(|b| b.id).collect());

    // tick_delta=1s, think_time=5s → each bidder re-evaluates every 5 seconds.
    let mut engine = SimulationEngine::new(Box::new(auction), bidders, 1.0, 5.0);

    println!("─── Auction start (opening price: $100.00) ───");
    println!();

    engine.run_to_completion();

    // --- Print the interesting events ---
    for (time, event) in engine.event_log.iter() {
        match event {
            AuctionEvent::BidAccepted { bid, new_standing } => {
                let name = engine.name_of(bid.bidder_id);
                println!("  t={:6.1}s  {:6} bids {}  (standing high bid)", time, name, new_standing);
            }
            AuctionEvent::AuctionClosed => {
                println!();
                println!("  t={:.1}s  >>> HAMMER FALLS <<<", time);
            }
            AuctionEvent::AllocationDecided(outcome) => {
                println!();
                println!("══════════════════ RESULT ══════════════════");
                match outcome.allocations.first() {
                    Some(alloc) => {
                        let name = engine.name_of(alloc.bidder_id);
                        let paid = outcome
                            .payments
                            .first()
                            .map(|p| p.amount)
                            .unwrap_or(Money::zero());
                        // Find true value for winner
                        let true_val = data
                            .iter()
                            .find(|(n, _)| *n == name)
                            .map(|(_, v)| Money(*v))
                            .unwrap_or(Money::zero());
                        let surplus = true_val - paid;

                        println!("  Winner  : {}", name);
                        println!("  Paid    : {}  (true value: {})", paid, true_val);
                        println!("  Surplus : {}", surplus);
                        println!("  Revenue : {}", outcome.revenue);
                    }
                    None => {
                        println!("  No winner — reserve price not met.");
                    }
                }
            }
            _ => {}
        }
    }

    // --- Theory debrief ---
    println!();
    println!("══════════════════ THEORY NOTE ══════════════════");
    println!();
    println!("DOMINANT STRATEGY");
    println!("  In an English auction, staying in while price < your value");
    println!("  and dropping out when price = your value weakly dominates");
    println!("  all other strategies. Bidding above value risks negative");
    println!("  surplus; dropping out early forfeits reachable gains.");
    println!();
    println!("REVENUE EQUIVALENCE");
    println!("  Under standard assumptions (symmetric, independent private");
    println!("  values), an English auction yields the same expected revenue");
    println!("  as a Vickrey (second-price sealed-bid) auction: the item");
    println!("  goes to the highest-value bidder who pays approximately the");
    println!("  second-highest value.");
    println!();
    println!("WINNER'S CURSE");
    println!("  When values have a common component (e.g., oil-field auctions),");
    println!("  winning reveals you were the most optimistic — your estimate");
    println!("  likely overstates true value. Rational bidders shade down.");
    println!("  (Not relevant in this pure private-value simulation.)");
    println!();
    println!("Try next: cargo run -p auction-engine --example english_sim");
    println!("Then Phase 2: the interactive TUI.");
}
