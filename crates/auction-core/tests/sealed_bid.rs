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
