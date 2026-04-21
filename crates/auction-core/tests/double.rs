mod common;

use auction_core::auction::double::{DoubleAuction, DoubleAuctionConfig};
use auction_core::mechanism::Auction;
use auction_core::types::{BidderId, Money};

const DEADLINE: f64 = 10.0;

fn auction(buyers: u32, sellers: u32) -> DoubleAuction {
    let buyer_ids: Vec<BidderId> = (0..buyers).map(BidderId).collect();
    let seller_ids: Vec<BidderId> = (buyers..buyers + sellers).map(BidderId).collect();
    DoubleAuction::new(
        DoubleAuctionConfig { deadline: DEADLINE },
        common::item(None),
        buyer_ids,
        seller_ids,
    )
}

fn buyer_bid(id: u32, amount: f64) -> auction_core::bid::Bid {
    common::bid(id, amount)
}

fn seller_ask(buyers: u32, seller_offset: u32, amount: f64) -> auction_core::bid::Bid {
    common::bid(buyers + seller_offset, amount)
}

/// k=0.5 clearing price = (last_matching_bid + last_matching_ask) / 2.
#[test]
fn clearing_price_is_midpoint_of_marginal_pair() {
    // 1 buyer bids $100, 1 seller asks $80 → clearing = $90.
    let mut a = auction(1, 1);
    a.submit_bid(buyer_bid(0, 100.0)).unwrap();
    a.submit_bid(seller_ask(1, 0, 80.0)).unwrap();
    a.tick(DEADLINE + 0.1);

    let outcome = a.outcome().unwrap();
    assert_eq!(outcome.allocations.len(), 1);
    assert_eq!(outcome.payments[0].amount, Money(90.0));
    assert_eq!(outcome.receipts[0].amount, Money(90.0));
}

/// Buyer pays the clearing price — not their submitted bid.
#[test]
fn buyer_pays_clearing_not_own_bid() {
    let mut a = auction(1, 1);
    a.submit_bid(buyer_bid(0, 120.0)).unwrap(); // bid well above ask
    a.submit_bid(seller_ask(1, 0, 80.0)).unwrap();
    a.tick(DEADLINE + 0.1);

    let outcome = a.outcome().unwrap();
    let clearing = Money((120.0 + 80.0) / 2.0); // $100
    assert_eq!(outcome.payments[0].amount, clearing);
    assert_ne!(outcome.payments[0].amount, Money(120.0)); // must not pay own bid
}

/// A bid below the ask produces no trades.
#[test]
fn no_crossing_means_no_trades() {
    let mut a = auction(1, 1);
    a.submit_bid(buyer_bid(0, 60.0)).unwrap();  // bid < ask
    a.submit_bid(seller_ask(1, 0, 80.0)).unwrap();
    a.tick(DEADLINE + 0.1);

    let outcome = a.outcome().unwrap();
    assert!(outcome.allocations.is_empty());
    assert!(outcome.payments.is_empty());
    assert_eq!(outcome.revenue, Money::zero());
}

/// Only the crossing pairs trade; non-crossing pairs are unmatched.
#[test]
fn partial_crossing_trades_correct_count() {
    // 3 buyers, 3 sellers; only 2 pairs cross.
    // Buyers: $120, $100, $60  (desc)
    // Sellers: $50, $80, $110  (asc)
    // Pair 1: $120 vs $50 → cross
    // Pair 2: $100 vs $80 → cross
    // Pair 3: $60  vs $110 → no cross
    let mut a = auction(3, 3);
    a.submit_bid(buyer_bid(0, 120.0)).unwrap();
    a.submit_bid(buyer_bid(1, 100.0)).unwrap();
    a.submit_bid(buyer_bid(2, 60.0)).unwrap();
    a.submit_bid(seller_ask(3, 0, 50.0)).unwrap();
    a.submit_bid(seller_ask(3, 1, 80.0)).unwrap();
    a.submit_bid(seller_ask(3, 2, 110.0)).unwrap();
    a.tick(DEADLINE + 0.1);

    let outcome = a.outcome().unwrap();
    assert_eq!(outcome.allocations.len(), 2);
    assert_eq!(outcome.payments.len(), 2);
    assert_eq!(outcome.receipts.len(), 2);
}

/// All matched buyers pay the same uniform clearing price.
#[test]
fn all_buyers_pay_uniform_clearing_price() {
    // 3 buyers ($120, $110, $100), 3 sellers ($35, $60, $80) → 3 trades.
    // Marginal pair: buyer $100, seller $80 → clearing = $90.
    let mut a = auction(3, 3);
    a.submit_bid(buyer_bid(0, 120.0)).unwrap();
    a.submit_bid(buyer_bid(1, 110.0)).unwrap();
    a.submit_bid(buyer_bid(2, 100.0)).unwrap();
    a.submit_bid(seller_ask(3, 0, 35.0)).unwrap();
    a.submit_bid(seller_ask(3, 1, 60.0)).unwrap();
    a.submit_bid(seller_ask(3, 2, 80.0)).unwrap();
    a.tick(DEADLINE + 0.1);

    let outcome = a.outcome().unwrap();
    assert_eq!(outcome.allocations.len(), 3);

    let clearing = Money(90.0);
    for payment in &outcome.payments {
        assert_eq!(payment.amount, clearing, "bidder {} paid wrong amount", payment.bidder_id);
    }
    for receipt in &outcome.receipts {
        assert_eq!(receipt.amount, clearing, "seller {} received wrong amount", receipt.bidder_id);
    }
}

/// Revenue = clearing_price × number_of_trades.
#[test]
fn revenue_equals_clearing_times_trades() {
    let mut a = auction(2, 2);
    // Buyers: $100, $90; Sellers: $50, $70 → both cross.
    // Marginal: buyer $90, seller $70 → clearing = $80.
    a.submit_bid(buyer_bid(0, 100.0)).unwrap();
    a.submit_bid(buyer_bid(1, 90.0)).unwrap();
    a.submit_bid(seller_ask(2, 0, 50.0)).unwrap();
    a.submit_bid(seller_ask(2, 1, 70.0)).unwrap();
    a.tick(DEADLINE + 0.1);

    let outcome = a.outcome().unwrap();
    assert_eq!(outcome.allocations.len(), 2);
    assert_eq!(outcome.revenue, Money(160.0)); // 2 × $80
}

/// Unmatched buyer (bid below marginal ask) has no allocation or payment entry.
#[test]
fn unmatched_buyer_has_no_allocation() {
    // 3 buyers ($120, $100, $60), 3 sellers ($50, $80, $110) → 2 trades.
    // Buyer 2 ($60) is unmatched; sellers sorted: $50, $80, $110.
    // Marginal pair: $100 bid vs $80 ask. Buyer $60 < $110 ask → unmatched.
    let mut a = auction(3, 3);
    a.submit_bid(buyer_bid(0, 120.0)).unwrap();
    a.submit_bid(buyer_bid(1, 100.0)).unwrap();
    a.submit_bid(buyer_bid(2, 60.0)).unwrap();  // unmatched
    a.submit_bid(seller_ask(3, 0, 50.0)).unwrap();
    a.submit_bid(seller_ask(3, 1, 80.0)).unwrap();
    a.submit_bid(seller_ask(3, 2, 110.0)).unwrap();
    a.tick(DEADLINE + 0.1);

    let outcome = a.outcome().unwrap();
    assert_eq!(outcome.allocations.len(), 2);
    assert!(!outcome.allocations.iter().any(|al| al.bidder_id.0 == 2));
    assert!(!outcome.payments.iter().any(|p| p.bidder_id.0 == 2));
}

/// Unmatched seller (ask above marginal bid) has no receipt entry.
#[test]
fn unmatched_seller_has_no_receipt() {
    // Same crossing setup; seller with ask=$110 (BidderId(5)) is unmatched.
    let mut a = auction(3, 3);
    a.submit_bid(buyer_bid(0, 120.0)).unwrap();
    a.submit_bid(buyer_bid(1, 100.0)).unwrap();
    a.submit_bid(buyer_bid(2, 60.0)).unwrap();
    a.submit_bid(seller_ask(3, 0, 50.0)).unwrap();
    a.submit_bid(seller_ask(3, 1, 80.0)).unwrap();
    a.submit_bid(seller_ask(3, 2, 110.0)).unwrap(); // unmatched — BidderId(5)
    a.tick(DEADLINE + 0.1);

    let outcome = a.outcome().unwrap();
    assert_eq!(outcome.receipts.len(), 2);
    assert!(!outcome.receipts.iter().any(|r| r.bidder_id.0 == 5));
}

/// Buyer below all asks → no trade; buyer above marginal ask → one trade.
#[test]
fn double_bid_variations() {
    // No trade: buyer $50 < seller $80
    {
        let mut a = auction(1, 1);
        a.submit_bid(buyer_bid(0, 50.0)).unwrap();
        a.submit_bid(seller_ask(1, 0, 80.0)).unwrap();
        a.tick(DEADLINE + 0.1);
        let o = a.outcome().unwrap();
        assert!(o.allocations.is_empty(), "50<80: no trade expected");
    }
    // One trade: buyer $120 > seller $80, clearing = (120+80)/2 = $100
    {
        let mut a = auction(1, 1);
        a.submit_bid(buyer_bid(0, 120.0)).unwrap();
        a.submit_bid(seller_ask(1, 0, 80.0)).unwrap();
        a.tick(DEADLINE + 0.1);
        let o = a.outcome().unwrap();
        assert_eq!(o.allocations.len(), 1, "120>80: 1 trade expected");
        assert_eq!(o.payments[0].amount, Money(100.0), "clearing=(120+80)/2");
    }
}
