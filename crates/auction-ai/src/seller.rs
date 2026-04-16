use rand::RngCore;

use auction_core::bid::Bid;
use auction_core::bidder::BidderStrategy;
use auction_core::mechanism::VisibleAuctionState;
use auction_core::types::{AuctionPhase, AuctionType, BidderId, Money};

/// Truthful seller for double auctions: submits cost as ask price.
/// Submitting at true cost is a dominant strategy in many double-auction
/// formats and serves as the educational benchmark.
pub struct TruthfulSellerBidder {
    bidder_id: BidderId,
    name: String,
}

impl TruthfulSellerBidder {
    pub fn new(bidder_id: BidderId, name: impl Into<String>) -> Self {
        TruthfulSellerBidder { bidder_id, name: name.into() }
    }
}

impl BidderStrategy for TruthfulSellerBidder {
    fn bidder_id(&self) -> BidderId {
        self.bidder_id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn decide(
        &mut self,
        state: &VisibleAuctionState,
        my_value: Money, // interpreted as cost for sellers
        _rng: &mut dyn RngCore,
    ) -> Option<Bid> {
        if state.auction_type != AuctionType::Double {
            return None;
        }
        if state.phase != AuctionPhase::Bidding {
            return None;
        }
        // Only submit once (active_bidders contains self while pending).
        if !state.active_bidders.contains(&self.bidder_id) {
            return None;
        }

        // Submit ask = cost (truthful seller strategy).
        Some(Bid {
            bidder_id: self.bidder_id,
            item_id: state.item_id,
            amount: my_value,
            timestamp: 0.0,
        })
    }
}
