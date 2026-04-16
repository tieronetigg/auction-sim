use rand::RngCore;

use crate::bid::Bid;
use crate::mechanism::VisibleAuctionState;
use crate::types::{BidderId, Money};

/// Decision-making interface for an AI-controlled participant.
/// Called each simulation tick; returns a bid or None to pass.
pub trait BidderStrategy: Send {
    fn bidder_id(&self) -> BidderId;
    fn name(&self) -> &str;

    /// Decide whether to bid given the current visible state and private value.
    fn decide(
        &mut self,
        state: &VisibleAuctionState,
        my_value: Money,
        rng: &mut dyn RngCore,
    ) -> Option<Bid>;
}
