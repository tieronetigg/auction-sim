use crate::types::{BidderId, ItemId, Money, SimTime};

#[derive(Debug, Clone)]
pub struct Bid {
    pub bidder_id: BidderId,
    pub item_id: ItemId,
    pub amount: Money,
    pub timestamp: SimTime,
}

#[derive(Debug, Clone)]
pub struct BidRecord {
    pub bid: Bid,
    /// Whether this is the current standing high bid.
    pub standing: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BidError {
    BelowMinimum { minimum: Money },
    AuctionNotActive,
    UnknownBidder,
    /// Sealed-bid: this bidder already submitted a bid.
    AlreadyBid,
}
