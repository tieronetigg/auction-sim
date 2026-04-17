use auction_core::bid::Bid;
use auction_core::item::Item;
use auction_core::types::{BidderId, ItemId, Money};

/// Construct a test item with an optional reserve price.
pub fn item(reserve: Option<f64>) -> Item {
    Item {
        id: ItemId(0),
        name: "Test Item".to_string(),
        reserve_price: reserve.map(Money),
    }
}

/// Construct a bid with a zero timestamp (engine sets real timestamps; tests don't need them).
pub fn bid(bidder: u32, amount: f64) -> Bid {
    Bid {
        bidder_id: BidderId(bidder),
        item_id: ItemId(0),
        amount: Money(amount),
        timestamp: 0.0,
    }
}

/// Produce N sequential BidderIds starting at 0.
#[allow(dead_code)]
pub fn ids(n: u32) -> Vec<BidderId> {
    (0..n).map(BidderId).collect()
}
