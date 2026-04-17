use crate::bid::{Bid, BidError};
use crate::event::AuctionEvent;
use crate::item::Item;
use crate::mechanism::{Auction, VisibleAuctionState};
use crate::outcome::{Allocation, AuctionOutcome, Payment, Receipt};
use crate::types::{AuctionPhase, AuctionType, BidderId, ItemId, Money, SimTime};

pub struct DoubleAuctionConfig {
    /// Seconds from start until all orders close and trades are resolved.
    pub deadline: SimTime,
}

pub struct DoubleAuction {
    pub config: DoubleAuctionConfig,
    pub item: Item,
    buyers: Vec<BidderId>,
    sellers: Vec<BidderId>,
    submitted: Vec<BidderId>,
    buy_orders: Vec<Bid>,
    sell_orders: Vec<Bid>,
    current_time: SimTime,
    phase: AuctionPhase,
    outcome: Option<AuctionOutcome>,
}

impl DoubleAuction {
    /// `buyers` submit bids; `sellers` submit asks (via `submit_bid`).
    pub fn new(
        config: DoubleAuctionConfig,
        item: Item,
        buyers: Vec<BidderId>,
        sellers: Vec<BidderId>,
    ) -> Self {
        DoubleAuction {
            config,
            item,
            buyers,
            sellers,
            submitted: Vec::new(),
            buy_orders: Vec::new(),
            sell_orders: Vec::new(),
            current_time: 0.0,
            phase: AuctionPhase::Bidding,
            outcome: None,
        }
    }

    /// k-double auction clearing (k = 0.5 uniform price).
    /// Sorts buy orders descending and sell orders ascending, finds all crossing
    /// pairs, and sets the clearing price as the midpoint of the last matching
    /// bid and ask.
    fn resolve(&mut self) -> Vec<AuctionEvent> {
        self.phase = AuctionPhase::Complete;

        let mut bids = std::mem::take(&mut self.buy_orders);
        let mut asks = std::mem::take(&mut self.sell_orders);

        bids.sort_by(|a, b| b.amount.0.partial_cmp(&a.amount.0).unwrap_or(std::cmp::Ordering::Equal));
        asks.sort_by(|a, b| a.amount.0.partial_cmp(&b.amount.0).unwrap_or(std::cmp::Ordering::Equal));

        // Find how many pairs cross.
        let k = bids
            .iter()
            .zip(asks.iter())
            .take_while(|(b, a)| b.amount >= a.amount)
            .count();

        let outcome = if k == 0 {
            AuctionOutcome {
                allocations: vec![],
                payments: vec![],
                receipts: vec![],
                revenue: Money::zero(),
                social_welfare: None,
                efficiency: None,
            }
        } else {
            // Clearing price = midpoint of last matching bid and ask (k = 0.5 rule).
            let last_bid = bids[k - 1].amount;
            let last_ask = asks[k - 1].amount;
            let clearing_price = Money((last_bid.0 + last_ask.0) / 2.0);

            let allocations: Vec<Allocation> = bids[..k]
                .iter()
                .map(|b| Allocation { bidder_id: b.bidder_id, item_id: self.item.id })
                .collect();

            let payments: Vec<Payment> = bids[..k]
                .iter()
                .map(|b| Payment { bidder_id: b.bidder_id, amount: clearing_price })
                .collect();

            let receipts: Vec<Receipt> = asks[..k]
                .iter()
                .map(|a| Receipt { bidder_id: a.bidder_id, amount: clearing_price })
                .collect();

            let revenue = clearing_price * k as f64;

            AuctionOutcome {
                allocations,
                payments,
                receipts,
                revenue,
                social_welfare: None,
                efficiency: None,
            }
        };

        self.outcome = Some(outcome.clone());
        vec![AuctionEvent::AuctionClosed, AuctionEvent::AllocationDecided(outcome)]
    }
}

impl Auction for DoubleAuction {
    fn auction_type(&self) -> AuctionType {
        AuctionType::Double
    }

    fn phase(&self) -> AuctionPhase {
        self.phase
    }

    fn item_id(&self) -> ItemId {
        self.item.id
    }

    fn item_name(&self) -> &str {
        &self.item.name
    }

    fn visible_state(&self) -> VisibleAuctionState {
        // Active = all participants (buyers and sellers) who haven't submitted yet.
        let all: Vec<BidderId> = self.buyers.iter().chain(self.sellers.iter()).copied().collect();
        let active: Vec<BidderId> = all
            .into_iter()
            .filter(|id| !self.submitted.contains(id))
            .collect();

        VisibleAuctionState {
            auction_type: AuctionType::Double,
            item_id: self.item.id,
            current_price: None,
            min_bid: Money::zero(),
            standing_bidder: None,
            bid_count: self.buy_orders.len() + self.sell_orders.len(),
            phase: self.phase,
            time_since_last_bid: 0.0,
            active_bidders: active,
            deadline_remaining: Some((self.config.deadline - self.current_time).max(0.0)),
        }
    }

    fn submit_bid(&mut self, bid: Bid) -> Result<Vec<AuctionEvent>, BidError> {
        if self.phase != AuctionPhase::Bidding {
            return Err(BidError::AuctionNotActive);
        }
        let is_buyer = self.buyers.contains(&bid.bidder_id);
        let is_seller = self.sellers.contains(&bid.bidder_id);
        if !is_buyer && !is_seller {
            return Err(BidError::UnknownBidder);
        }
        if self.submitted.contains(&bid.bidder_id) {
            return Err(BidError::AlreadyBid);
        }
        if bid.amount <= Money::zero() {
            return Err(BidError::BelowMinimum { minimum: Money(0.01) });
        }

        self.submitted.push(bid.bidder_id);

        if is_buyer {
            self.buy_orders.push(bid.clone());
            Ok(vec![AuctionEvent::BidSubmitted(bid)])
        } else {
            self.sell_orders.push(bid.clone());
            Ok(vec![AuctionEvent::AskSubmitted(bid)])
        }
    }

    fn tick(&mut self, delta: SimTime) -> Vec<AuctionEvent> {
        if self.phase != AuctionPhase::Bidding {
            return vec![];
        }
        self.current_time += delta;
        if self.current_time >= self.config.deadline {
            self.resolve()
        } else {
            vec![]
        }
    }

    fn outcome(&self) -> Option<&AuctionOutcome> {
        self.outcome.as_ref()
    }
}
