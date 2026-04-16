use crate::bid::{Bid, BidError, BidRecord};
use crate::event::AuctionEvent;
use crate::item::Item;
use crate::mechanism::{Auction, VisibleAuctionState};
use crate::outcome::{Allocation, AuctionOutcome, Payment};
use crate::types::{AuctionPhase, AuctionType, BidderId, ItemId, Money, SimTime};

pub struct EnglishConfig {
    /// Opening price; the first bid must meet this threshold.
    pub start_price: Money,
    /// Each new bid must exceed the standing high bid by at least this amount.
    pub min_increment: Money,
    /// Seconds of silence after the last bid before the hammer falls.
    pub activity_timeout: SimTime,
}

pub struct EnglishAuction {
    pub config: EnglishConfig,
    pub item: Item,
    bidders: Vec<BidderId>,
    current_price: Money,
    standing_bidder: Option<BidderId>,
    last_bid_time: SimTime,
    current_time: SimTime,
    bid_history: Vec<BidRecord>,
    phase: AuctionPhase,
    outcome: Option<AuctionOutcome>,
}

impl EnglishAuction {
    pub fn new(config: EnglishConfig, item: Item, bidders: Vec<BidderId>) -> Self {
        let start_price = config.start_price;
        EnglishAuction {
            config,
            item,
            bidders,
            current_price: start_price,
            standing_bidder: None,
            last_bid_time: 0.0,
            current_time: 0.0,
            bid_history: Vec::new(),
            phase: AuctionPhase::Bidding,
            outcome: None,
        }
    }

    fn min_bid(&self) -> Money {
        if self.standing_bidder.is_some() {
            self.current_price + self.config.min_increment
        } else {
            self.config.start_price
        }
    }

    fn close(&mut self) -> Vec<AuctionEvent> {
        self.phase = AuctionPhase::Complete;

        let (allocations, payments, revenue) = match self.standing_bidder {
            Some(winner) => {
                let alloc = vec![Allocation { bidder_id: winner, item_id: self.item.id }];
                let pay = vec![Payment { bidder_id: winner, amount: self.current_price }];
                (alloc, pay, self.current_price)
            }
            None => (vec![], vec![], Money::zero()),
        };

        let outcome = AuctionOutcome {
            allocations,
            payments,
            receipts: vec![],
            revenue,
            social_welfare: None,
            efficiency: None,
        };
        self.outcome = Some(outcome.clone());

        vec![AuctionEvent::AuctionClosed, AuctionEvent::AllocationDecided(outcome)]
    }
}

impl Auction for EnglishAuction {
    fn auction_type(&self) -> AuctionType {
        AuctionType::English
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
        VisibleAuctionState {
            auction_type: AuctionType::English,
            item_id: self.item.id,
            current_price: Some(self.current_price),
            min_bid: self.min_bid(),
            standing_bidder: self.standing_bidder,
            bid_count: self.bid_history.len(),
            phase: self.phase,
            time_since_last_bid: self.current_time - self.last_bid_time,
            active_bidders: self.bidders.clone(),
            deadline_remaining: None,
        }
    }

    fn submit_bid(&mut self, bid: Bid) -> Result<Vec<AuctionEvent>, BidError> {
        if self.phase != AuctionPhase::Bidding {
            return Err(BidError::AuctionNotActive);
        }
        if !self.bidders.contains(&bid.bidder_id) {
            return Err(BidError::UnknownBidder);
        }

        let min = self.min_bid();
        if bid.amount < min {
            return Err(BidError::BelowMinimum { minimum: min });
        }

        let old_price = self.current_price;
        self.current_price = bid.amount;
        self.standing_bidder = Some(bid.bidder_id);
        self.last_bid_time = bid.timestamp;

        for record in &mut self.bid_history {
            record.standing = false;
        }
        self.bid_history.push(BidRecord { bid: bid.clone(), standing: true });

        let mut events = vec![AuctionEvent::BidAccepted {
            bid,
            new_standing: self.current_price,
        }];
        if old_price != self.current_price {
            events.push(AuctionEvent::PriceChanged { old: old_price, new: self.current_price });
        }
        Ok(events)
    }

    fn tick(&mut self, delta: SimTime) -> Vec<AuctionEvent> {
        if self.phase != AuctionPhase::Bidding {
            return vec![];
        }
        self.current_time += delta;

        let silence = self.current_time - self.last_bid_time;
        if silence < self.config.activity_timeout {
            return vec![];
        }

        // Reserve price check: if no one bid or price is below reserve, no sale.
        if let Some(reserve) = self.item.reserve_price {
            if self.standing_bidder.is_none() || self.current_price < reserve {
                self.phase = AuctionPhase::Complete;
                let outcome = AuctionOutcome {
                    allocations: vec![],
                    payments: vec![],
                    receipts: vec![],
                    revenue: Money::zero(),
                    social_welfare: None,
                    efficiency: None,
                };
                self.outcome = Some(outcome.clone());
                return vec![
                    AuctionEvent::AuctionClosed,
                    AuctionEvent::AllocationDecided(outcome),
                ];
            }
        }

        // No bids at all and no reserve — also no sale.
        if self.standing_bidder.is_none() {
            self.phase = AuctionPhase::Complete;
            let outcome = AuctionOutcome {
                allocations: vec![],
                payments: vec![],
                receipts: vec![],
                revenue: Money::zero(),
                social_welfare: None,
                efficiency: None,
            };
            self.outcome = Some(outcome.clone());
            return vec![AuctionEvent::AuctionClosed, AuctionEvent::AllocationDecided(outcome)];
        }

        self.close()
    }

    fn outcome(&self) -> Option<&AuctionOutcome> {
        self.outcome.as_ref()
    }
}
