mod common;

use auction_core::auction::sealed_bid::{SealedBidAuction, SealedBidConfig, SealedMechanism};
use auction_core::bid::BidError;
use auction_core::mechanism::Auction;
use auction_core::types::Money;

const DEADLINE: f64 = 10.0;

fn fpsb(reserve: Option<f64>) -> SealedBidAuction {
    SealedBidAuction::new(
        SealedBidConfig { mechanism: SealedMechanism::FirstPrice, deadline: DEADLINE, reserve_price: reserve.map(Money) },
        common::item(None),
        common::ids(4),
    )
}

fn vickrey(reserve: Option<f64>) -> SealedBidAuction {
    SealedBidAuction::new(
        SealedBidConfig { mechanism: SealedMechanism::SecondPrice, deadline: DEADLINE, reserve_price: reserve.map(Money) },
        common::item(None),
        common::ids(4),
    )
}

// ── FPSB ─────────────────────────────────────────────────────────────────────

/// Winner pays exactly their own bid.
#[test]
fn fpsb_winner_pays_own_bid() {
    let mut a = fpsb(None);
    a.submit_bid(common::bid(0, 200.0)).unwrap();
    a.submit_bid(common::bid(1, 300.0)).unwrap(); // winner
    a.submit_bid(common::bid(2, 150.0)).unwrap();
    a.tick(DEADLINE + 0.1);

    let outcome = a.outcome().unwrap();
    assert_eq!(outcome.allocations[0].bidder_id.0, 1);
    assert_eq!(outcome.payments[0].amount, Money(300.0));
}

/// Only the winner has a payment entry in FPSB (losers pay nothing).
#[test]
fn fpsb_only_winner_pays() {
    let mut a = fpsb(None);
    a.submit_bid(common::bid(0, 100.0)).unwrap();
    a.submit_bid(common::bid(1, 250.0)).unwrap();
    a.tick(DEADLINE + 0.1);

    let outcome = a.outcome().unwrap();
    assert_eq!(outcome.payments.len(), 1);
    assert_eq!(outcome.payments[0].bidder_id.0, 1);
}

/// Reserve not met → no allocation, no payment.
#[test]
fn fpsb_reserve_not_met() {
    let mut a = fpsb(Some(400.0));
    a.submit_bid(common::bid(0, 350.0)).unwrap();
    a.submit_bid(common::bid(1, 380.0)).unwrap();
    a.tick(DEADLINE + 0.1);

    let outcome = a.outcome().unwrap();
    assert!(outcome.allocations.is_empty());
    assert!(outcome.payments.is_empty());
}

// ── Vickrey ───────────────────────────────────────────────────────────────────

/// Winner pays the second-highest bid, not their own.
#[test]
fn vickrey_winner_pays_second_highest_bid() {
    let mut a = vickrey(None);
    a.submit_bid(common::bid(0, 100.0)).unwrap();
    a.submit_bid(common::bid(1, 300.0)).unwrap(); // winner
    a.submit_bid(common::bid(2, 200.0)).unwrap(); // second-highest
    a.tick(DEADLINE + 0.1);

    let outcome = a.outcome().unwrap();
    assert_eq!(outcome.allocations[0].bidder_id.0, 1);
    assert_eq!(outcome.payments[0].amount, Money(200.0)); // not 300
}

/// With only one bidder above the reserve, the winner pays the reserve (truth-dominance floor).
#[test]
fn vickrey_single_bidder_pays_reserve() {
    let mut a = vickrey(Some(150.0));
    a.submit_bid(common::bid(0, 300.0)).unwrap(); // only bidder
    a.tick(DEADLINE + 0.1);

    let outcome = a.outcome().unwrap();
    assert_eq!(outcome.allocations[0].bidder_id.0, 0);
    assert_eq!(outcome.payments[0].amount, Money(150.0)); // pays reserve, not own bid
}

// ── Shared sealed-bid behaviour ───────────────────────────────────────────────

/// A second submission from the same bidder returns AlreadyBid.
#[test]
fn duplicate_submission_rejected() {
    let mut a = fpsb(None);
    a.submit_bid(common::bid(0, 100.0)).unwrap();
    let err = a.submit_bid(common::bid(0, 200.0)).unwrap_err();
    assert_eq!(err, BidError::AlreadyBid);
}

/// Bids are accepted before deadline; no outcome until deadline elapses.
#[test]
fn no_outcome_before_deadline() {
    let mut a = fpsb(None);
    a.submit_bid(common::bid(0, 100.0)).unwrap();
    a.tick(DEADLINE - 1.0);
    assert!(a.outcome().is_none());
    a.tick(2.0);
    assert!(a.outcome().is_some());
}

// ── FPSB additional ───────────────────────────────────────────────────────────

/// Losers are absent from both allocations and payments in FPSB.
#[test]
fn fpsb_loser_has_no_allocation() {
    let mut a = fpsb(None);
    a.submit_bid(common::bid(0, 100.0)).unwrap(); // loser
    a.submit_bid(common::bid(1, 200.0)).unwrap(); // loser
    a.submit_bid(common::bid(2, 300.0)).unwrap(); // winner
    a.tick(DEADLINE + 0.1);

    let outcome = a.outcome().unwrap();
    assert_eq!(outcome.allocations.len(), 1);
    assert_eq!(outcome.allocations[0].bidder_id.0, 2);
    assert_eq!(outcome.payments.len(), 1);
    assert!(!outcome.payments.iter().any(|p| p.bidder_id.0 == 0));
    assert!(!outcome.payments.iter().any(|p| p.bidder_id.0 == 1));
}

/// In a FPSB tie, the first submitter wins (Vec::sort_by is stable).
#[test]
fn fpsb_tie_first_submitter_wins() {
    let mut a = fpsb(None);
    a.submit_bid(common::bid(0, 200.0)).unwrap(); // submitted first
    a.submit_bid(common::bid(1, 200.0)).unwrap(); // same amount, submitted second
    a.tick(DEADLINE + 0.1);

    let outcome = a.outcome().unwrap();
    assert_eq!(outcome.allocations[0].bidder_id.0, 0, "first submitter wins on tie");
}

/// Bid amounts vs reserve: below → no sale; at/above → sale with correct payment.
#[test]
fn fpsb_bid_variations() {
    let cases: &[(f64, f64, bool)] = &[
        (150.0, 200.0, false), // below reserve → no sale
        (200.0, 200.0, true),  // at reserve → sale, pays $200
        (350.0, 200.0, true),  // above reserve → sale, pays $350
    ];
    for &(bid_amount, reserve, expect_sale) in cases {
        let mut a = fpsb(Some(reserve));
        a.submit_bid(common::bid(0, bid_amount)).unwrap();
        a.tick(DEADLINE + 0.1);
        let outcome = a.outcome().unwrap();
        if expect_sale {
            assert_eq!(outcome.allocations.len(), 1, "bid={bid_amount}");
            assert_eq!(outcome.payments[0].amount, Money(bid_amount), "bid={bid_amount}");
        } else {
            assert!(outcome.allocations.is_empty(), "bid={bid_amount}");
            assert!(outcome.payments.is_empty(), "bid={bid_amount}");
        }
    }
}

// ── Vickrey additional ────────────────────────────────────────────────────────

/// Losers are absent from both allocations and payments in Vickrey.
#[test]
fn vickrey_loser_has_no_allocation() {
    let mut a = vickrey(None);
    a.submit_bid(common::bid(0, 100.0)).unwrap();
    a.submit_bid(common::bid(1, 200.0)).unwrap();
    a.submit_bid(common::bid(2, 300.0)).unwrap(); // winner
    a.tick(DEADLINE + 0.1);

    let outcome = a.outcome().unwrap();
    assert_eq!(outcome.allocations.len(), 1);
    assert_eq!(outcome.allocations[0].bidder_id.0, 2);
    assert_eq!(outcome.payments.len(), 1);
    assert_eq!(outcome.payments[0].bidder_id.0, 2);
    assert!(!outcome.payments.iter().any(|p| p.bidder_id.0 == 0));
    assert!(!outcome.payments.iter().any(|p| p.bidder_id.0 == 1));
}

/// Single bidder, no reserve → winner pays $0 (no second price exists).
#[test]
fn vickrey_single_bidder_no_reserve_pays_zero() {
    let mut a = vickrey(None);
    a.submit_bid(common::bid(0, 300.0)).unwrap();
    a.tick(DEADLINE + 0.1);

    let outcome = a.outcome().unwrap();
    assert_eq!(outcome.allocations[0].bidder_id.0, 0);
    assert_eq!(outcome.payments[0].amount, Money(0.0));
}

/// In a Vickrey tie, the first submitter wins and pays the tied second price.
#[test]
fn vickrey_tie_first_submitter_wins() {
    let mut a = vickrey(None);
    a.submit_bid(common::bid(0, 200.0)).unwrap(); // first — wins
    a.submit_bid(common::bid(1, 200.0)).unwrap(); // second
    a.tick(DEADLINE + 0.1);

    let outcome = a.outcome().unwrap();
    assert_eq!(outcome.allocations[0].bidder_id.0, 0, "first submitter wins");
    assert_eq!(outcome.payments[0].amount, Money(200.0), "second price = tied amount");
}
