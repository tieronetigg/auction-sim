use rand::RngCore;

use auction_core::bid::Bid;
use auction_core::bidder::BidderStrategy;
use auction_core::mechanism::VisibleAuctionState;
use auction_core::types::{AuctionPhase, AuctionType, BidderId, Money};

/// Bids a shaded fraction of true value — the equilibrium strategy in FPSB.
///
/// In an n-bidder symmetric FPSB auction with uniform values, the Bayes-Nash
/// equilibrium bid is `value × (n-1)/n`. This bidder uses a configurable
/// `shade_factor` to model that shading.
pub struct BidShadingBidder {
    bidder_id: BidderId,
    name: String,
    /// Fraction of true value to bid (e.g. 0.75 bids 75 % of value).
    shade_factor: f64,
}

impl BidShadingBidder {
    pub fn new(bidder_id: BidderId, name: impl Into<String>, shade_factor: f64) -> Self {
        BidShadingBidder { bidder_id, name: name.into(), shade_factor }
    }
}

impl BidderStrategy for BidShadingBidder {
    fn bidder_id(&self) -> BidderId {
        self.bidder_id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn decide(
        &mut self,
        state: &VisibleAuctionState,
        my_value: Money,
        _rng: &mut dyn RngCore,
    ) -> Option<Bid> {
        if state.phase != AuctionPhase::Bidding {
            return None;
        }
        match state.auction_type {
            AuctionType::FirstPriceSealedBid => {
                // Shade the bid; duplicate submissions are rejected by the auction.
                let shaded = Money(my_value.0 * self.shade_factor);
                Some(Bid {
                    bidder_id: self.bidder_id,
                    item_id: state.item_id,
                    amount: shaded,
                    timestamp: 0.0,
                })
            }
            _ => None,
        }
    }
}
