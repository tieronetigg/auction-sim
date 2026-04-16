use crate::bid::Bid;
use crate::outcome::AuctionOutcome;
use crate::types::{BidderId, Money};

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum AuctionEvent {
    BidAccepted { bid: Bid, new_standing: Money },
    BidRejected { bid: Bid, reason: BidRejectionReason },
    PriceChanged { old: Money, new: Money },
    BidderDropped { bidder_id: BidderId },
    AuctionClosed,
    AllocationDecided(AuctionOutcome),
    /// Sealed-bid acknowledgment: bid received (amount stays hidden from other bidders).
    BidSubmitted(Bid),
    /// Double-auction acknowledgment: sell order received (amount stays hidden until reveal).
    AskSubmitted(Bid),
}

#[derive(Debug, Clone, PartialEq)]
pub enum BidRejectionReason {
    BelowMinimum,
    AuctionNotActive,
    UnknownBidder,
}
