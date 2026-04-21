use auction_ai::seller::TruthfulSellerBidder;
use auction_ai::shading::BidShadingBidder;
use auction_ai::truthful::TruthfulBidder;
use auction_core::auction::all_pay::{AllPayAuction, AllPayConfig};
use auction_core::auction::double::{DoubleAuction, DoubleAuctionConfig};
use auction_core::auction::dutch::{DutchAuction, DutchConfig};
use auction_core::auction::english::{EnglishAuction, EnglishConfig};
use auction_core::auction::sealed_bid::{SealedBidAuction, SealedBidConfig, SealedMechanism};
use auction_core::bidder::BidderStrategy;
use auction_core::item::Item;
use auction_core::types::{BidderId, ItemId, Money};
use auction_engine::engine::{BidderConfig, SimulationEngine};

fn item() -> Item {
    Item { id: ItemId(0), name: "Test Item".to_string(), reserve_price: None }
}

fn bc(id: u32, name: &str, value: f64, strategy: Box<dyn BidderStrategy>) -> BidderConfig {
    BidderConfig { id: BidderId(id), name: name.to_string(), strategy, value: Money(value) }
}

// ── English ───────────────────────────────────────────────────────────────────

/// 3 truthful AI bidders: B (v=$300) has highest value and wins.
/// With min_increment=$10, B must bid $210 to beat A's last bid of $200.
#[test]
fn english_full_sim() {
    let bidder_ids = vec![BidderId(0), BidderId(1), BidderId(2)];
    let auction = EnglishAuction::new(
        EnglishConfig {
            start_price: Money(100.0),
            min_increment: Money(10.0),
            activity_timeout: 10.0,
        },
        item(),
        bidder_ids,
    );
    let bidders = vec![
        bc(0, "A", 200.0, Box::new(TruthfulBidder::new(BidderId(0), "A"))),
        bc(1, "B", 300.0, Box::new(TruthfulBidder::new(BidderId(1), "B"))),
        bc(2, "C", 150.0, Box::new(TruthfulBidder::new(BidderId(2), "C"))),
    ];
    let mut engine = SimulationEngine::new(Box::new(auction), bidders, 1.0, 3.0);
    engine.stagger_starts();
    engine.run_to_completion();

    let outcome = engine.outcome().unwrap();
    assert_eq!(outcome.allocations[0].bidder_id, BidderId(1), "B (highest value) wins");
    assert!(outcome.payments[0].amount > Money(200.0), "B must outbid A's max");
    assert!(outcome.payments[0].amount <= Money(300.0), "B never bids above value");
}

// ── Dutch ─────────────────────────────────────────────────────────────────────

/// 3 truthful AI bidders: B (v=$300) calls as soon as the clock reaches $300.
/// A (v=$200) and C (v=$150) have not yet called at that price.
#[test]
fn dutch_full_sim() {
    let bidder_ids = vec![BidderId(0), BidderId(1), BidderId(2)];
    let auction = DutchAuction::new(
        DutchConfig {
            start_price: Money(500.0),
            decrement_per_second: Money(10.0),
            floor_price: Money(50.0),
        },
        item(),
        bidder_ids,
    );
    let bidders = vec![
        bc(0, "A", 200.0, Box::new(TruthfulBidder::new(BidderId(0), "A"))),
        bc(1, "B", 300.0, Box::new(TruthfulBidder::new(BidderId(1), "B"))),
        bc(2, "C", 150.0, Box::new(TruthfulBidder::new(BidderId(2), "C"))),
    ];
    // Short think_time so bidders react within one 1s tick.
    let mut engine = SimulationEngine::new(Box::new(auction), bidders, 1.0, 0.3);
    engine.stagger_starts();
    engine.run_to_completion();

    let outcome = engine.outcome().unwrap();
    assert_eq!(outcome.allocations[0].bidder_id, BidderId(1), "B (highest value) calls first");
    assert_eq!(outcome.payments[0].amount, Money(300.0), "B calls when clock = $300");
}

// ── FPSB ──────────────────────────────────────────────────────────────────────

/// 3 bid-shading AI bidders (0.8× true value): B's shaded bid ($240) is highest.
#[test]
fn fpsb_full_sim() {
    let bidder_ids = vec![BidderId(0), BidderId(1), BidderId(2)];
    let auction = SealedBidAuction::new(
        SealedBidConfig { mechanism: SealedMechanism::FirstPrice, deadline: 30.0, reserve_price: None },
        item(),
        bidder_ids,
    );
    let bidders = vec![
        bc(0, "A", 200.0, Box::new(BidShadingBidder::new(BidderId(0), "A", 0.8))),
        bc(1, "B", 300.0, Box::new(BidShadingBidder::new(BidderId(1), "B", 0.8))),
        bc(2, "C", 150.0, Box::new(BidShadingBidder::new(BidderId(2), "C", 0.8))),
    ];
    let mut engine = SimulationEngine::new(Box::new(auction), bidders, 1.0, 3.0);
    engine.stagger_starts();
    engine.run_to_completion();

    // Shaded bids: A→$160, B→$240, C→$120. B wins.
    let outcome = engine.outcome().unwrap();
    assert_eq!(outcome.allocations[0].bidder_id, BidderId(1), "B (highest shaded bid) wins");
    assert_eq!(outcome.payments[0].amount, Money(240.0), "B pays own bid in FPSB");
}

// ── Vickrey ───────────────────────────────────────────────────────────────────

/// 3 truthful AI bidders: B wins and pays the second-highest bid (A's $200).
#[test]
fn vickrey_full_sim() {
    let bidder_ids = vec![BidderId(0), BidderId(1), BidderId(2)];
    let auction = SealedBidAuction::new(
        SealedBidConfig { mechanism: SealedMechanism::SecondPrice, deadline: 30.0, reserve_price: None },
        item(),
        bidder_ids,
    );
    let bidders = vec![
        bc(0, "A", 200.0, Box::new(TruthfulBidder::new(BidderId(0), "A"))),
        bc(1, "B", 300.0, Box::new(TruthfulBidder::new(BidderId(1), "B"))),
        bc(2, "C", 150.0, Box::new(TruthfulBidder::new(BidderId(2), "C"))),
    ];
    let mut engine = SimulationEngine::new(Box::new(auction), bidders, 1.0, 3.0);
    engine.stagger_starts();
    engine.run_to_completion();

    let outcome = engine.outcome().unwrap();
    assert_eq!(outcome.allocations[0].bidder_id, BidderId(1), "B (highest value) wins");
    assert_eq!(outcome.payments[0].amount, Money(200.0), "B pays second-highest bid (A's $200)");
}

// ── All-Pay ───────────────────────────────────────────────────────────────────

/// 3 truthful AI bidders: B wins; all 3 have payment entries.
#[test]
fn allpay_full_sim() {
    let bidder_ids = vec![BidderId(0), BidderId(1), BidderId(2)];
    let auction = AllPayAuction::new(
        AllPayConfig { deadline: 30.0, reserve_price: None },
        item(),
        bidder_ids,
    );
    let bidders = vec![
        bc(0, "A", 200.0, Box::new(TruthfulBidder::new(BidderId(0), "A"))),
        bc(1, "B", 300.0, Box::new(TruthfulBidder::new(BidderId(1), "B"))),
        bc(2, "C", 150.0, Box::new(TruthfulBidder::new(BidderId(2), "C"))),
    ];
    let mut engine = SimulationEngine::new(Box::new(auction), bidders, 1.0, 3.0);
    engine.stagger_starts();
    engine.run_to_completion();

    let outcome = engine.outcome().unwrap();
    assert_eq!(outcome.allocations[0].bidder_id, BidderId(1), "B (highest bid) wins");
    assert_eq!(outcome.payments.len(), 3, "all 3 bidders pay in all-pay");
    assert_eq!(outcome.revenue, Money(650.0), "revenue = 200 + 300 + 150");
}

// ── Double auction ────────────────────────────────────────────────────────────

/// 2 truthful buyers (A=$120, B=$80) + 2 truthful sellers (S1=$50, S2=$100).
/// Only A and S1 cross; 1 trade at clearing = (120+50)/2 = $85.
#[test]
fn double_full_sim() {
    let buyer_ids = vec![BidderId(0), BidderId(1)];
    let seller_ids = vec![BidderId(2), BidderId(3)];
    let all_ids: Vec<BidderId> = buyer_ids.iter().chain(seller_ids.iter()).copied().collect();

    let auction = DoubleAuction::new(
        DoubleAuctionConfig { deadline: 20.0 },
        item(),
        buyer_ids,
        seller_ids,
    );
    let bidders = vec![
        bc(0, "A",  120.0, Box::new(TruthfulBidder::new(BidderId(0), "A"))),
        bc(1, "B",   80.0, Box::new(TruthfulBidder::new(BidderId(1), "B"))),
        bc(2, "S1",  50.0, Box::new(TruthfulSellerBidder::new(BidderId(2), "S1"))),
        bc(3, "S2", 100.0, Box::new(TruthfulSellerBidder::new(BidderId(3), "S2"))),
    ];
    let _ = all_ids; // constructed above for clarity, unused directly
    let mut engine = SimulationEngine::new(Box::new(auction), bidders, 1.0, 3.0);
    engine.stagger_starts();
    engine.run_to_completion();

    // Sorted bids desc: [120, 80]; sorted asks asc: [50, 100].
    // Pair (120, 50) crosses; (80, 100) does not. k=1.
    // Clearing = (120+50)/2 = 85.
    let outcome = engine.outcome().unwrap();
    assert_eq!(outcome.allocations.len(), 1, "1 trade");
    assert_eq!(outcome.allocations[0].bidder_id, BidderId(0), "A (buyer) wins");
    assert_eq!(outcome.payments[0].amount, Money(85.0), "A pays clearing $85");
    assert_eq!(outcome.receipts[0].amount, Money(85.0), "S1 receives clearing $85");
    assert!(!outcome.allocations.iter().any(|al| al.bidder_id == BidderId(1)), "B unmatched");
}
