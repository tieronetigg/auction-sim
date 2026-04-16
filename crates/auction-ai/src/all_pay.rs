use rand::RngCore;

use auction_core::bid::Bid;
use auction_core::bidder::BidderStrategy;
use auction_core::mechanism::VisibleAuctionState;
use auction_core::types::{AuctionPhase, AuctionType, BidderId, Money};

/// Bids according to the Bayes-Nash equilibrium for an all-pay auction
/// with symmetric bidders drawing values uniformly from [0, value_upper].
///
/// Equilibrium bid: b(v) = (n-1)/n · v · (v / H)^(n-1)
///
/// Bids once during the collection period (duplicate submissions are rejected
/// by the auction with `AlreadyBid`).
pub struct AllPayBidder {
    bidder_id: BidderId,
    name: String,
    /// Number of bidders in the auction (including self), used for formula.
    n_bidders: usize,
    /// Upper bound of the value distribution H.
    value_upper: Money,
}

impl AllPayBidder {
    pub fn new(
        bidder_id: BidderId,
        name: impl Into<String>,
        n_bidders: usize,
        value_upper: Money,
    ) -> Self {
        AllPayBidder {
            bidder_id,
            name: name.into(),
            n_bidders,
            value_upper,
        }
    }

    /// Computes the BNE bid for this bidder's private value.
    fn equilibrium_bid(&self, value: Money) -> Money {
        let n = self.n_bidders as f64;
        let h = self.value_upper.0;
        if h <= 0.0 || n <= 1.0 {
            return value;
        }
        let v = value.0;
        // b(v) = (n-1)/n * v * (v/H)^(n-1)
        let bid = ((n - 1.0) / n) * v * (v / h).powf(n - 1.0);
        Money(bid.max(0.01))
    }
}

impl BidderStrategy for AllPayBidder {
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
        if state.auction_type != AuctionType::AllPay {
            return None;
        }
        if state.phase != AuctionPhase::Bidding {
            return None;
        }
        // Only bid if not yet submitted (active_bidders contains self).
        if !state.active_bidders.contains(&self.bidder_id) {
            return None;
        }

        let amount = self.equilibrium_bid(my_value);
        Some(Bid {
            bidder_id: self.bidder_id,
            item_id: state.item_id,
            amount,
            timestamp: 0.0, // overwritten by engine
        })
    }
}
