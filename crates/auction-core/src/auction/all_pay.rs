use crate::bid::{Bid, BidError};
use crate::event::AuctionEvent;
use crate::item::Item;
use crate::mechanism::{Auction, VisibleAuctionState};
use crate::outcome::{Allocation, AuctionOutcome, Payment};
use crate::types::{AuctionPhase, AuctionType, BidderId, ItemId, Money, SimTime};

pub struct AllPayConfig {
    /// Seconds from start until bids close and winner is resolved.
    pub deadline: SimTime,
    pub reserve_price: Option<Money>,
}

pub struct AllPayAuction {
    pub config: AllPayConfig,
    pub item: Item,
    registered_bidders: Vec<BidderId>,
    submitted_bidders: Vec<BidderId>,
    bids: Vec<Bid>,
    current_time: SimTime,
    phase: AuctionPhase,
    outcome: Option<AuctionOutcome>,
}

impl AllPayAuction {
    pub fn new(config: AllPayConfig, item: Item, bidders: Vec<BidderId>) -> Self {
        AllPayAuction {
            config,
            item,
            registered_bidders: bidders,
            submitted_bidders: Vec::new(),
            bids: Vec::new(),
            current_time: 0.0,
            phase: AuctionPhase::Bidding,
            outcome: None,
        }
    }

    fn resolve(&mut self) -> Vec<AuctionEvent> {
        self.phase = AuctionPhase::Complete;

        // Sort descending to find the winner.
        let mut sorted = self.bids.clone();
        sorted.sort_by(|a, b| {
            b.amount.0.partial_cmp(&a.amount.0).unwrap_or(std::cmp::Ordering::Equal)
        });

        let outcome = if let Some(top) = sorted.first() {
            let reserve_met = self
                .config
                .reserve_price
                .map(|r| top.amount >= r)
                .unwrap_or(true);

            if reserve_met {
                // All-pay: winner gets the item, EVERYONE pays their own bid.
                let allocations = vec![Allocation { bidder_id: top.bidder_id, item_id: self.item.id }];
                let payments: Vec<Payment> = self
                    .bids
                    .iter()
                    .map(|b| Payment { bidder_id: b.bidder_id, amount: b.amount })
                    .collect();
                let revenue = payments.iter().fold(Money::zero(), |acc, p| acc + p.amount);
                AuctionOutcome {
                    allocations,
                    payments,
                    receipts: vec![],
                    revenue,
                    social_welfare: None,
                    efficiency: None,
                }
            } else {
                // Reserve not met — no allocation, but all bids are still lost.
                let payments: Vec<Payment> = self
                    .bids
                    .iter()
                    .map(|b| Payment { bidder_id: b.bidder_id, amount: b.amount })
                    .collect();
                let revenue = payments.iter().fold(Money::zero(), |acc, p| acc + p.amount);
                AuctionOutcome {
                    allocations: vec![],
                    payments,
                    receipts: vec![],
                    revenue,
                    social_welfare: None,
                    efficiency: None,
                }
            }
        } else {
            // No bids.
            AuctionOutcome {
                allocations: vec![],
                payments: vec![],
                receipts: vec![],
                revenue: Money::zero(),
                social_welfare: None,
                efficiency: None,
            }
        };

        self.outcome = Some(outcome.clone());
        vec![AuctionEvent::AuctionClosed, AuctionEvent::AllocationDecided(outcome)]
    }
}

impl Auction for AllPayAuction {
    fn auction_type(&self) -> AuctionType {
        AuctionType::AllPay
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
        let active: Vec<BidderId> = self
            .registered_bidders
            .iter()
            .filter(|id| !self.submitted_bidders.contains(id))
            .copied()
            .collect();

        VisibleAuctionState {
            auction_type: AuctionType::AllPay,
            item_id: self.item.id,
            current_price: None,
            min_bid: Money::zero(),
            standing_bidder: None,
            bid_count: self.bids.len(),
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
        if !self.registered_bidders.contains(&bid.bidder_id) {
            return Err(BidError::UnknownBidder);
        }
        if self.submitted_bidders.contains(&bid.bidder_id) {
            return Err(BidError::AlreadyBid);
        }
        if bid.amount <= Money::zero() {
            return Err(BidError::BelowMinimum { minimum: Money(0.01) });
        }

        self.submitted_bidders.push(bid.bidder_id);
        self.bids.push(bid.clone());
        Ok(vec![AuctionEvent::BidSubmitted(bid)])
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
