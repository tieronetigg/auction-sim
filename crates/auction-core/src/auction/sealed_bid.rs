use crate::bid::{Bid, BidError};
use crate::event::AuctionEvent;
use crate::item::Item;
use crate::mechanism::{Auction, VisibleAuctionState};
use crate::outcome::{Allocation, AuctionOutcome, Payment};
use crate::types::{AuctionPhase, AuctionType, BidderId, ItemId, Money, SimTime};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SealedMechanism {
    /// Winner pays their own bid.
    FirstPrice,
    /// Winner pays the second-highest bid (Vickrey).
    SecondPrice,
}

pub struct SealedBidConfig {
    pub mechanism: SealedMechanism,
    /// Seconds from auction start until bids close and winner is resolved.
    pub deadline: SimTime,
    pub reserve_price: Option<Money>,
}

pub struct SealedBidAuction {
    pub config: SealedBidConfig,
    pub item: Item,
    registered_bidders: Vec<BidderId>,
    submitted_bidders: Vec<BidderId>,
    bids: Vec<Bid>,
    current_time: SimTime,
    phase: AuctionPhase,
    outcome: Option<AuctionOutcome>,
}

impl SealedBidAuction {
    pub fn new(config: SealedBidConfig, item: Item, bidders: Vec<BidderId>) -> Self {
        SealedBidAuction {
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

        // Sort bids descending by amount.
        let mut sorted = self.bids.clone();
        sorted.sort_by(|a, b| {
            b.amount.0.partial_cmp(&a.amount.0).unwrap_or(std::cmp::Ordering::Equal)
        });

        let outcome = match sorted.first() {
            None => AuctionOutcome {
                allocations: vec![],
                payments: vec![],
                receipts: vec![],
                revenue: Money::zero(),
                social_welfare: None,
                efficiency: None,
            },
            Some(top) => {
                // Check reserve price.
                let reserve_met = self
                    .config
                    .reserve_price
                    .map(|r| top.amount >= r)
                    .unwrap_or(true);

                if !reserve_met {
                    AuctionOutcome {
                        allocations: vec![],
                        payments: vec![],
                        receipts: vec![],
                        revenue: Money::zero(),
                        social_welfare: None,
                        efficiency: None,
                    }
                } else {
                    let payment_amount = match self.config.mechanism {
                        SealedMechanism::FirstPrice => top.amount,
                        SealedMechanism::SecondPrice => sorted
                            .get(1)
                            .map(|b| b.amount)
                            .or(self.config.reserve_price)
                            .unwrap_or(Money::zero()),
                    };
                    AuctionOutcome {
                        allocations: vec![Allocation {
                            bidder_id: top.bidder_id,
                            item_id: self.item.id,
                        }],
                        payments: vec![Payment {
                            bidder_id: top.bidder_id,
                            amount: payment_amount,
                        }],
                        receipts: vec![],
                        revenue: payment_amount,
                        social_welfare: None,
                        efficiency: None,
                    }
                }
            }
        };

        self.outcome = Some(outcome.clone());
        vec![AuctionEvent::AuctionClosed, AuctionEvent::AllocationDecided(outcome)]
    }
}

impl Auction for SealedBidAuction {
    fn auction_type(&self) -> AuctionType {
        match self.config.mechanism {
            SealedMechanism::FirstPrice => AuctionType::FirstPriceSealedBid,
            SealedMechanism::SecondPrice => AuctionType::Vickrey,
        }
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
        // Active bidders = registered bidders who haven't submitted yet.
        let active: Vec<BidderId> = self
            .registered_bidders
            .iter()
            .filter(|id| !self.submitted_bidders.contains(id))
            .copied()
            .collect();

        VisibleAuctionState {
            auction_type: self.auction_type(),
            item_id: self.item.id,
            current_price: None,      // sealed — price is not public
            min_bid: Money::zero(),   // any positive amount accepted
            standing_bidder: None,    // hidden until reveal
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
