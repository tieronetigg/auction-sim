mod common;

use auction_core::auction::dutch::{DutchAuction, DutchConfig};
use auction_core::mechanism::Auction;
use auction_core::types::{AuctionPhase, Money};

fn auction() -> DutchAuction {
    DutchAuction::new(
        DutchConfig {
            start_price: Money(500.0),
            decrement_per_second: Money(10.0),
            floor_price: Money(50.0),
        },
        common::item(None),
        common::ids(3),
    )
}

/// The caller pays the clock price at the moment they call — not the start price.
#[test]
fn caller_pays_current_clock_price() {
    let mut a = auction();
    a.tick(20.0); // price drops 20×$10 = $200 → now $300
    a.submit_bid(common::bid(0, 1.0)).unwrap(); // amount ignored; pays current price

    let outcome = a.outcome().unwrap();
    assert_eq!(outcome.allocations.len(), 1);
    assert_eq!(outcome.allocations[0].bidder_id.0, 0);
    assert_eq!(outcome.payments[0].amount, Money(300.0));
    assert_eq!(outcome.revenue, Money(300.0));
}

/// Clock reaching the floor with no caller → auction closes with no winner.
#[test]
fn floor_reached_no_sale() {
    let mut a = auction();
    // Need (500 - 50) / 10 = 45 seconds to reach floor.
    a.tick(50.0);
    assert_eq!(a.phase(), AuctionPhase::Complete);

    let outcome = a.outcome().unwrap();
    assert!(outcome.allocations.is_empty());
    assert!(outcome.payments.is_empty());
}

/// Calling at exactly the start price pays the start price.
#[test]
fn call_at_start_price() {
    let mut a = auction();
    a.submit_bid(common::bid(1, 1.0)).unwrap();

    let outcome = a.outcome().unwrap();
    assert_eq!(outcome.payments[0].amount, Money(500.0));
}

/// After a call the auction is complete; further bids are rejected.
#[test]
fn no_bids_after_close() {
    let mut a = auction();
    a.submit_bid(common::bid(0, 1.0)).unwrap();
    let err = a.submit_bid(common::bid(1, 1.0)).unwrap_err();
    assert_eq!(err, auction_core::bid::BidError::AuctionNotActive);
}

/// Non-calling bidders appear in neither allocations nor payments.
#[test]
fn non_caller_has_no_allocation_or_payment() {
    let mut a = auction();
    a.tick(20.0); // price drops to $300
    a.submit_bid(common::bid(0, 1.0)).unwrap(); // only bidder 0 calls

    let outcome = a.outcome().unwrap();
    assert_eq!(outcome.allocations.len(), 1);
    assert_eq!(outcome.allocations[0].bidder_id.0, 0);
    assert_eq!(outcome.payments.len(), 1);
    assert_eq!(outcome.payments[0].bidder_id.0, 0);
    assert!(!outcome.payments.iter().any(|p| p.bidder_id.0 == 1));
    assert!(!outcome.payments.iter().any(|p| p.bidder_id.0 == 2));
}

/// Payment always equals the clock price at the moment of the call, not the bid amount.
#[test]
fn bid_variations_payment_matches_clock() {
    // (seconds ticked before call, expected clock price)
    let cases: &[(f64, f64)] = &[(0.0, 500.0), (10.0, 400.0), (30.0, 200.0)];
    for &(ticks, expected_payment) in cases {
        let mut a = auction();
        if ticks > 0.0 {
            a.tick(ticks);
        }
        a.submit_bid(common::bid(0, 1.0)).unwrap();
        let outcome = a.outcome().unwrap();
        assert_eq!(
            outcome.payments[0].amount,
            Money(expected_payment),
            "after {ticks}s tick expected ${expected_payment}"
        );
    }
}
