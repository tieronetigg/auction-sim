mod common;

use auction_core::auction::english::{EnglishAuction, EnglishConfig};
use auction_core::mechanism::Auction;
use auction_core::types::{AuctionPhase, Money};

const TIMEOUT: f64 = 5.0;

fn auction(reserve: Option<f64>) -> EnglishAuction {
    EnglishAuction::new(
        EnglishConfig {
            start_price: Money(100.0),
            min_increment: Money(10.0),
            activity_timeout: TIMEOUT,
        },
        common::item(reserve),
        common::ids(3), // bidders 0, 1, 2
    )
}

/// After the silence timeout the auction closes and the standing bidder wins,
/// paying their bid — not the start price.
#[test]
fn winner_pays_standing_bid_not_start_price() {
    let mut a = auction(None);
    a.submit_bid(common::bid(0, 200.0)).unwrap();
    a.tick(TIMEOUT + 0.1);

    let outcome = a.outcome().unwrap();
    assert_eq!(outcome.allocations.len(), 1);
    assert_eq!(outcome.allocations[0].bidder_id.0, 0);
    assert_eq!(outcome.payments[0].amount, Money(200.0));
}

/// With no bids the auction closes with no allocation.
#[test]
fn no_bids_no_sale() {
    let mut a = auction(None);
    a.tick(TIMEOUT + 0.1);

    let outcome = a.outcome().unwrap();
    assert!(outcome.allocations.is_empty());
    assert!(outcome.payments.is_empty());
    assert_eq!(a.phase(), AuctionPhase::Complete);
}

/// The highest bidder wins, not the first bidder.
#[test]
fn highest_bidder_wins() {
    let mut a = auction(None);
    a.submit_bid(common::bid(0, 110.0)).unwrap(); // bidder 0 leads
    a.submit_bid(common::bid(1, 150.0)).unwrap(); // bidder 1 outbids
    a.tick(TIMEOUT + 0.1);

    let outcome = a.outcome().unwrap();
    assert_eq!(outcome.allocations[0].bidder_id.0, 1);
    assert_eq!(outcome.payments[0].amount, Money(150.0));
}

/// A bid exactly at the reserve clears; auction closes with that winner.
#[test]
fn reserve_met_sale_proceeds() {
    let mut a = auction(Some(100.0));
    a.submit_bid(common::bid(0, 100.0)).unwrap();
    a.tick(TIMEOUT + 0.1);

    let outcome = a.outcome().unwrap();
    assert_eq!(outcome.allocations.len(), 1);
}

/// Standing price below the reserve at timeout → no sale.
#[test]
fn reserve_not_met_no_allocation() {
    let mut a = auction(Some(300.0));
    a.submit_bid(common::bid(0, 150.0)).unwrap();
    a.tick(TIMEOUT + 0.1);

    let outcome = a.outcome().unwrap();
    assert!(outcome.allocations.is_empty());
    assert!(outcome.payments.is_empty());
    assert_eq!(outcome.revenue, Money::zero());
}

/// A second bid must exceed the standing bid by at least min_increment.
#[test]
fn bid_below_increment_rejected() {
    let mut a = auction(None);
    a.submit_bid(common::bid(0, 100.0)).unwrap();
    // next min is 110; bidding 105 should fail
    let err = a.submit_bid(common::bid(1, 105.0)).unwrap_err();
    assert_eq!(err, auction_core::bid::BidError::BelowMinimum { minimum: Money(110.0) });
}

/// Auction is still active mid-timeout; phase changes only after timeout elapses.
#[test]
fn phase_bidding_mid_silence() {
    let mut a = auction(None);
    a.submit_bid(common::bid(0, 100.0)).unwrap();
    a.tick(TIMEOUT - 1.0); // not yet elapsed
    assert_eq!(a.phase(), AuctionPhase::Bidding);
    a.tick(2.0); // now elapsed
    assert_eq!(a.phase(), AuctionPhase::Complete);
}

/// The losing bidder appears in neither allocations nor payments.
#[test]
fn loser_has_no_allocation_or_payment() {
    let mut a = auction(None);
    a.submit_bid(common::bid(0, 110.0)).unwrap(); // loser
    a.submit_bid(common::bid(1, 200.0)).unwrap(); // winner
    a.tick(TIMEOUT + 0.1);

    let outcome = a.outcome().unwrap();
    assert_eq!(outcome.allocations.len(), 1);
    assert_eq!(outcome.allocations[0].bidder_id.0, 1);
    assert_eq!(outcome.payments.len(), 1);
    assert_eq!(outcome.payments[0].bidder_id.0, 1);
    assert!(!outcome.payments.iter().any(|p| p.bidder_id.0 == 0));
}

/// Bid amounts relative to reserve: below → no sale; at and above → sale.
#[test]
fn bid_variations_with_reserve() {
    // (bid_amount, expect_sale)
    let cases: &[(f64, bool)] = &[(150.0, false), (200.0, true), (350.0, true)];
    for &(amount, expect_sale) in cases {
        let mut a = auction(Some(200.0));
        a.submit_bid(common::bid(0, amount)).unwrap();
        a.tick(TIMEOUT + 0.1);
        let outcome = a.outcome().unwrap();
        if expect_sale {
            assert!(!outcome.allocations.is_empty(), "bid={amount}: expected sale");
            assert_eq!(outcome.payments[0].amount, Money(amount), "bid={amount}: wrong payment");
        } else {
            assert!(outcome.allocations.is_empty(), "bid={amount}: expected no sale");
        }
    }
}

/// A bid far below the current standing price is rejected with the correct minimum.
#[test]
fn bid_far_below_standing_price_rejected() {
    let mut a = auction(None);
    a.submit_bid(common::bid(0, 200.0)).unwrap(); // standing at $200
    // minimum is now 210; bidding $100 must fail
    let err = a.submit_bid(common::bid(1, 100.0)).unwrap_err();
    assert_eq!(err, auction_core::bid::BidError::BelowMinimum { minimum: Money(210.0) });
}
