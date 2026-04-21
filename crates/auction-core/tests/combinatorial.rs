use auction_core::auction::combinatorial::{
    CombinatorialAuction, CombinatorialConfig, CombinatorialError, CombinatorialPaymentRule,
};
use auction_core::package::Package;
use auction_core::types::{BidderId, ItemId, Money};

const DEADLINE: f64 = 10.0;

fn item(id: u32) -> ItemId {
    ItemId(id)
}

fn bidder(id: u32) -> BidderId {
    BidderId(id)
}

fn pkg(ids: &[u32]) -> Package {
    Package(ids.iter().copied().map(ItemId).collect())
}

fn vcg_auction(n_bidders: u32) -> CombinatorialAuction {
    let ids = (0..n_bidders).map(BidderId).collect();
    CombinatorialAuction::new(
        CombinatorialConfig { payment_rule: CombinatorialPaymentRule::Vcg, deadline: DEADLINE },
        ids,
    )
}

fn pab_auction(n_bidders: u32) -> CombinatorialAuction {
    let ids = (0..n_bidders).map(BidderId).collect();
    CombinatorialAuction::new(
        CombinatorialConfig {
            payment_rule: CombinatorialPaymentRule::PayAsBid,
            deadline: DEADLINE,
        },
        ids,
    )
}

// ── Single-item VCG == Vickrey ────────────────────────────────────────────────

/// With 1 item and 2 bidders, VCG reduces to Vickrey: winner pays second-highest bid.
/// W* = 100 (Alice). W*_{-Alice} = 60 (Bob alone). p_Alice = 60 − (100−100) = $60.
#[test]
fn single_item_vcg_equals_vickrey() {
    let mut a = vcg_auction(2);
    a.submit_package_bid(bidder(0), pkg(&[0]), Money(100.0)).unwrap(); // Alice
    a.submit_package_bid(bidder(1), pkg(&[0]), Money(60.0)).unwrap();  // Bob
    a.tick(DEADLINE + 0.1);

    let outcome = &a.outcome().unwrap().outcome;
    assert_eq!(outcome.allocations.len(), 1);
    assert_eq!(outcome.allocations[0].bidder_id, bidder(0));
    assert_eq!(outcome.payments[0].amount, Money(60.0));
}

/// Loser has no allocation and no payment entry.
#[test]
fn single_item_vcg_loser_pays_nothing() {
    let mut a = vcg_auction(2);
    a.submit_package_bid(bidder(0), pkg(&[0]), Money(100.0)).unwrap();
    a.submit_package_bid(bidder(1), pkg(&[0]), Money(60.0)).unwrap();
    a.tick(DEADLINE + 0.1);

    let outcome = &a.outcome().unwrap().outcome;
    assert_eq!(outcome.payments.len(), 1, "only winner pays");
    assert!(!outcome.payments.iter().any(|p| p.bidder_id == bidder(1)));
    assert!(!outcome.allocations.iter().any(|al| al.bidder_id == bidder(1)));
}

// ── North/South complementarity ───────────────────────────────────────────────

/// Bob and Carol's individual items beat Alice's bundle.
///   Alice: {N,S}=$100; Bob: {N}=$70; Carol: {S}=$50. W* = $120 (split).
///   W*_{-Bob}: max(Alice=$100, Carol=$50) = $100. p_Bob = 100−(120−70) = $50.
///   W*_{-Carol}: max(Alice=$100, Bob=$70) = $100. p_Carol = 100−(120−50) = $30.
///   Revenue = $80; VCG deficit = $40.
#[test]
fn north_south_split_beats_bundle() {
    let mut a = vcg_auction(3);
    a.submit_package_bid(bidder(0), pkg(&[0, 1]), Money(100.0)).unwrap(); // Alice
    a.submit_package_bid(bidder(1), pkg(&[0]), Money(70.0)).unwrap();     // Bob
    a.submit_package_bid(bidder(2), pkg(&[1]), Money(50.0)).unwrap();     // Carol
    a.tick(DEADLINE + 0.1);

    let outcome = &a.outcome().unwrap().outcome;

    assert_eq!(outcome.allocations.len(), 2);
    assert!(outcome.allocations.iter().any(|al| al.bidder_id == bidder(1)));
    assert!(outcome.allocations.iter().any(|al| al.bidder_id == bidder(2)));
    assert!(!outcome.allocations.iter().any(|al| al.bidder_id == bidder(0)));

    let bob_pay = outcome.payments.iter().find(|p| p.bidder_id == bidder(1)).unwrap();
    let carol_pay = outcome.payments.iter().find(|p| p.bidder_id == bidder(2)).unwrap();
    assert_eq!(bob_pay.amount, Money(50.0));
    assert_eq!(carol_pay.amount, Money(30.0));

    assert_eq!(outcome.revenue, Money(80.0));
}

/// Alice's bundle beats the split.
///   Alice: {N,S}=$200; Bob: {N}=$70; Carol: {S}=$50. W* = $200 (Alice).
///   W*_{-Alice} = Bob+Carol = $120. p_Alice = 120−(200−200) = $120.
#[test]
fn north_south_bundle_beats_split() {
    let mut a = vcg_auction(3);
    a.submit_package_bid(bidder(0), pkg(&[0, 1]), Money(200.0)).unwrap(); // Alice
    a.submit_package_bid(bidder(1), pkg(&[0]), Money(70.0)).unwrap();     // Bob
    a.submit_package_bid(bidder(2), pkg(&[1]), Money(50.0)).unwrap();     // Carol
    a.tick(DEADLINE + 0.1);

    let outcome = &a.outcome().unwrap().outcome;

    assert_eq!(outcome.allocations.len(), 2);
    assert!(outcome.allocations.iter().all(|al| al.bidder_id == bidder(0)));
    assert_eq!(outcome.payments.len(), 1);
    assert_eq!(outcome.payments[0].amount, Money(120.0));
    assert_eq!(outcome.revenue, Money(120.0));
}

// ── All losers pay zero ───────────────────────────────────────────────────────

#[test]
fn all_losers_pay_zero() {
    let mut a = vcg_auction(3);
    a.submit_package_bid(bidder(0), pkg(&[0]), Money(100.0)).unwrap();
    a.submit_package_bid(bidder(1), pkg(&[0]), Money(80.0)).unwrap();
    a.submit_package_bid(bidder(2), pkg(&[0]), Money(60.0)).unwrap();
    a.tick(DEADLINE + 0.1);

    let outcome = &a.outcome().unwrap().outcome;
    assert_eq!(outcome.allocations.len(), 1);
    assert_eq!(outcome.allocations[0].bidder_id, bidder(0));
    assert_eq!(outcome.payments.len(), 1, "only winner has a payment entry");
    assert!(!outcome.payments.iter().any(|p| p.bidder_id == bidder(1)));
    assert!(!outcome.payments.iter().any(|p| p.bidder_id == bidder(2)));
}

// ── Individual rationality ────────────────────────────────────────────────────

/// Every winner's VCG payment ≤ their stated value.
#[test]
fn vcg_individual_rationality() {
    let mut a = vcg_auction(3);
    a.submit_package_bid(bidder(0), pkg(&[0, 1]), Money(100.0)).unwrap(); // Alice
    a.submit_package_bid(bidder(1), pkg(&[0]), Money(70.0)).unwrap();     // Bob wins
    a.submit_package_bid(bidder(2), pkg(&[1]), Money(50.0)).unwrap();     // Carol wins
    a.tick(DEADLINE + 0.1);

    let outcome = &a.outcome().unwrap().outcome;
    let values = [(bidder(1), 70.0_f64), (bidder(2), 50.0_f64)];
    for (id, value) in values {
        let payment = outcome.payments.iter().find(|p| p.bidder_id == id).unwrap();
        assert!(
            payment.amount.0 <= value,
            "bidder {:?} pays {:.2} > value {:.2}",
            id, payment.amount.0, value
        );
    }
}

// ── Budget deficit ────────────────────────────────────────────────────────────

/// VCG revenue may be less than total welfare (deficit is expected behaviour).
#[test]
fn vcg_may_have_budget_deficit() {
    let mut a = vcg_auction(3);
    a.submit_package_bid(bidder(0), pkg(&[0, 1]), Money(100.0)).unwrap();
    a.submit_package_bid(bidder(1), pkg(&[0]), Money(70.0)).unwrap();
    a.submit_package_bid(bidder(2), pkg(&[1]), Money(50.0)).unwrap();
    a.tick(DEADLINE + 0.1);

    let outcome = &a.outcome().unwrap().outcome;
    // W* = $120, revenue = $80.
    assert!(outcome.revenue.0 < 120.0, "revenue {} should be < welfare 120", outcome.revenue.0);
}

// ── Pay-as-bid ────────────────────────────────────────────────────────────────

/// In pay-as-bid, each winner pays exactly their submitted value.
#[test]
fn pay_as_bid_winner_pays_own_value() {
    let mut a = pab_auction(2);
    a.submit_package_bid(bidder(0), pkg(&[0]), Money(100.0)).unwrap();
    a.submit_package_bid(bidder(1), pkg(&[0]), Money(60.0)).unwrap();
    a.tick(DEADLINE + 0.1);

    let outcome = &a.outcome().unwrap().outcome;
    assert_eq!(outcome.allocations[0].bidder_id, bidder(0));
    assert_eq!(outcome.payments[0].amount, Money(100.0));
}

// ── No bids ───────────────────────────────────────────────────────────────────

#[test]
fn no_bids_empty_outcome() {
    let mut a = vcg_auction(2);
    a.tick(DEADLINE + 0.1);

    let outcome = &a.outcome().unwrap().outcome;
    assert!(outcome.allocations.is_empty());
    assert!(outcome.payments.is_empty());
    assert_eq!(outcome.revenue, Money(0.0));
}

// ── Game scenario verification (TUI default values) ──────────────────────────

/// Verify the TUI game scenario: Alice{N}=$20, Bob{S}=$15, Carol{N,S}=$40.
///
/// Without human: W* = Carol{N,S}=$40 (beats Alice+Bob=$35).
/// With human bid {N,S}=$100: W* = $100; W*_{-Human} = Carol=$40.
/// VCG payment = $40. Surplus = $60.
#[test]
fn tui_game_human_wins_with_vcg_discount() {
    let mut a = vcg_auction(4); // Human(0), Alice(1), Bob(2), Carol(3)
    // AI bids (as submitted in new_combinatorial_game)
    a.submit_package_bid(bidder(1), pkg(&[0]), Money(20.0)).unwrap();     // Alice: {N}
    a.submit_package_bid(bidder(2), pkg(&[1]), Money(15.0)).unwrap();     // Bob: {S}
    a.submit_package_bid(bidder(3), pkg(&[0, 1]), Money(40.0)).unwrap(); // Carol: {N,S}
    // Human bids truthfully on bundle
    a.submit_package_bid(bidder(0), pkg(&[0, 1]), Money(100.0)).unwrap(); // Human: {N,S}
    a.tick(DEADLINE + 0.1);

    let co = a.outcome().unwrap();
    let outcome = &co.outcome;

    // Human wins both items.
    assert_eq!(outcome.allocations.len(), 2);
    assert!(outcome.allocations.iter().all(|al| al.bidder_id == bidder(0)));

    // VCG payment = W*_{-Human} = Carol{N,S}=$40.
    assert_eq!(outcome.payments[0].amount, Money(40.0), "human pays VCG externality $40");
    assert_eq!(outcome.revenue, Money(40.0));
}

/// Without human bid, Carol wins (W* = $40 > Alice+Bob = $35).
#[test]
fn tui_game_carol_wins_without_human() {
    let mut a = vcg_auction(4);
    a.submit_package_bid(bidder(1), pkg(&[0]), Money(20.0)).unwrap();
    a.submit_package_bid(bidder(2), pkg(&[1]), Money(15.0)).unwrap();
    a.submit_package_bid(bidder(3), pkg(&[0, 1]), Money(40.0)).unwrap();
    a.tick(DEADLINE + 0.1);

    let co = a.outcome().unwrap();
    let outcome = &co.outcome;
    // Carol wins the bundle.
    assert_eq!(outcome.allocations.len(), 2);
    assert!(outcome.allocations.iter().all(|al| al.bidder_id == bidder(3)));
    // VCG: W*_{-Carol} = Alice+Bob=$35. p_Carol = 35-(40-40) = $35.
    assert_eq!(outcome.payments[0].amount, Money(35.0));
}

// ── Closed auction rejects bids ───────────────────────────────────────────────

#[test]
fn closed_auction_rejects_bids() {
    let mut a = vcg_auction(2);
    a.tick(DEADLINE + 0.1);
    let err = a.submit_package_bid(bidder(0), pkg(&[0]), Money(100.0)).unwrap_err();
    assert_eq!(err, CombinatorialError::AuctionClosed);
}

// ── XOR bidding: at most one bid per bidder selected ─────────────────────────

/// Alice bids {N}=$60 AND {N,S}=$90; Bob bids {S}=$50.
///
/// Feasible allocations (XOR: at most one Alice bid, disjoint items):
///   Alice{N}+Bob{S} = $110  ← winner
///   Alice{N,S} alone = $90
///   Bob{S} alone = $50
///
/// W* = $110. W*_{-Alice}=$50 (Bob). p_Alice = 50−(110−60) = $0.
/// W*_{-Bob}=$90 (Alice{N,S}). p_Bob = 90−(110−50) = $30.
/// Revenue = $30.
#[test]
fn multiple_bids_per_bidder_xor_at_most_one_selected() {
    let mut a = vcg_auction(2);
    a.submit_package_bid(bidder(0), pkg(&[0]), Money(60.0)).unwrap();    // Alice bids N
    a.submit_package_bid(bidder(0), pkg(&[0, 1]), Money(90.0)).unwrap(); // Alice bids {N,S}
    a.submit_package_bid(bidder(1), pkg(&[1]), Money(50.0)).unwrap();    // Bob bids S
    a.tick(DEADLINE + 0.1);

    let outcome = &a.outcome().unwrap().outcome;

    // Alice wins N, Bob wins S.
    let alice_items: Vec<ItemId> = outcome
        .allocations.iter().filter(|al| al.bidder_id == bidder(0)).map(|al| al.item_id).collect();
    let bob_items: Vec<ItemId> = outcome
        .allocations.iter().filter(|al| al.bidder_id == bidder(1)).map(|al| al.item_id).collect();
    assert_eq!(alice_items, vec![item(0)]);
    assert_eq!(bob_items, vec![item(1)]);

    let alice_pay = outcome.payments.iter().find(|p| p.bidder_id == bidder(0)).unwrap();
    let bob_pay = outcome.payments.iter().find(|p| p.bidder_id == bidder(1)).unwrap();
    assert_eq!(alice_pay.amount, Money(0.0));
    assert_eq!(bob_pay.amount, Money(30.0));
    assert_eq!(outcome.revenue, Money(30.0));
}
