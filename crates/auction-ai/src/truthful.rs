use rand::RngCore;

use auction_core::bid::Bid;
use auction_core::bidder::BidderStrategy;
use auction_core::mechanism::VisibleAuctionState;
use auction_core::types::{AuctionPhase, AuctionType, BidderId, Money};

/// Bids truthfully: raises only while standing price is below private value.
///
/// Dominant strategy in Vickrey auctions; also correct for English auctions.
/// Suboptimal for first-price sealed-bid (should shade).
pub struct TruthfulBidder {
    bidder_id: BidderId,
    name: String,
}

impl TruthfulBidder {
    pub fn new(bidder_id: BidderId, name: impl Into<String>) -> Self {
        TruthfulBidder { bidder_id, name: name.into() }
    }
}

impl BidderStrategy for TruthfulBidder {
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
            AuctionType::English => {
                // Already the high bidder — no need to outbid ourselves.
                if state.standing_bidder == Some(self.bidder_id) {
                    return None;
                }
                // Bid the minimum required, but only up to true value.
                if state.min_bid <= my_value {
                    Some(Bid {
                        bidder_id: self.bidder_id,
                        item_id: state.item_id,
                        amount: state.min_bid,
                        timestamp: 0.0, // overwritten by the engine before submission
                    })
                } else {
                    None // price has exceeded value — drop out
                }
            }
            AuctionType::Dutch => {
                // Call (accept) the moment the descending clock price reaches our value.
                let price = state.current_price?;
                if price <= my_value {
                    Some(Bid {
                        bidder_id: self.bidder_id,
                        item_id: state.item_id,
                        amount: price,
                        timestamp: 0.0,
                    })
                } else {
                    None
                }
            }
            AuctionType::Vickrey | AuctionType::FirstPriceSealedBid | AuctionType::AllPay => {
                // Sealed formats: bid exactly true value.
                // Duplicate bids are rejected by the auction (AlreadyBid error).
                Some(Bid {
                    bidder_id: self.bidder_id,
                    item_id: state.item_id,
                    amount: my_value,
                    timestamp: 0.0,
                })
            }
            AuctionType::Double => {
                // Buyer in a double auction: bid true value once.
                if state.active_bidders.contains(&self.bidder_id) {
                    Some(Bid {
                        bidder_id: self.bidder_id,
                        item_id: state.item_id,
                        amount: my_value,
                        timestamp: 0.0,
                    })
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}
