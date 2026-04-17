mod common;

use auction_core::auction::all_pay::{AllPayAuction, AllPayConfig};
use auction_core::mechanism::Auction;
use auction_core::types::{BidderId, Money};

const DEADLINE: f64 = 10.0;

fn auction() -> AllPayAuction {
    AllPayAuction::new(
        AllPayConfig { deadline: DEADLINE, reserve_price: None },
        common::item(None),
        common::ids(4),
    )
}

/// Every bidder who submitted a bid appears in `outcome.payments`,
/// regardless of whether they won. This is the invariant that `.first()` breaks.
#[test]
fn all_bidders_have_payment_entries() {
    let mut a = auction();
    a.submit_bid(common::bid(0, 50.0)).unwrap();
    a.submit_bid(common::bid(1, 100.0)).unwrap();
    a.submit_bid(common::bid(2, 75.0)).unwrap();
    a.tick(DEADLINE + 0.1);

    let outcome = a.outcome().unwrap();
    assert_eq!(outcome.payments.len(), 3, "all 3 bidders must have payments");

    let payer_ids: Vec<u32> = outcome.payments.iter().map(|p| p.bidder_id.0).collect();
    assert!(payer_ids.contains(&0));
    assert!(payer_ids.contains(&1));
    assert!(payer_ids.contains(&2));
}

/// Only the highest bidder receives the allocation; losers pay but get nothing.
#[test]
fn only_winner_has_allocation() {
    let mut a = auction();
    a.submit_bid(common::bid(0, 50.0)).unwrap();
    a.submit_bid(common::bid(1, 100.0)).unwrap(); // winner
    a.submit_bid(common::bid(2, 75.0)).unwrap();
    a.tick(DEADLINE + 0.1);

    let outcome = a.outcome().unwrap();
    assert_eq!(outcome.allocations.len(), 1);
    assert_eq!(outcome.allocations[0].bidder_id, BidderId(1));
}

/// Each bidder's payment equals their submitted bid — not the winner's bid.
#[test]
fn each_payment_matches_submitted_bid() {
    let mut a = auction();
    a.submit_bid(common::bid(0, 50.0)).unwrap();
    a.submit_bid(common::bid(1, 100.0)).unwrap();
    a.tick(DEADLINE + 0.1);

    let outcome = a.outcome().unwrap();

    let p0 = outcome.payments.iter().find(|p| p.bidder_id.0 == 0).unwrap();
    let p1 = outcome.payments.iter().find(|p| p.bidder_id.0 == 1).unwrap();
    assert_eq!(p0.amount, Money(50.0));
    assert_eq!(p1.amount, Money(100.0));
}

/// Revenue equals the sum of all submitted bids.
#[test]
fn revenue_is_sum_of_all_bids() {
    let mut a = auction();
    a.submit_bid(common::bid(0, 40.0)).unwrap();
    a.submit_bid(common::bid(1, 60.0)).unwrap();
    a.submit_bid(common::bid(2, 80.0)).unwrap();
    a.tick(DEADLINE + 0.1);

    let outcome = a.outcome().unwrap();
    assert_eq!(outcome.revenue, Money(180.0));
}

/// With no bids, payments and allocations are both empty.
#[test]
fn no_bids_empty_outcome() {
    let mut a = auction();
    a.tick(DEADLINE + 0.1);

    let outcome = a.outcome().unwrap();
    assert!(outcome.allocations.is_empty());
    assert!(outcome.payments.is_empty());
    assert_eq!(outcome.revenue, Money::zero());
}
