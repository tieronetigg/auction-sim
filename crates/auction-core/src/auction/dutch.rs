use crate::bid::{Bid, BidError};
use crate::event::AuctionEvent;
use crate::item::Item;
use crate::mechanism::{Auction, VisibleAuctionState};
use crate::outcome::{Allocation, AuctionOutcome, Payment};
use crate::types::{AuctionPhase, AuctionType, BidderId, ItemId, Money, SimTime};

pub struct DutchConfig {
    /// Opening price — starts high and falls.
    pub start_price: Money,
    /// Price drop in dollars per second.
    pub decrement_per_second: Money,
    /// If price reaches the floor without a caller, auction closes with no winner.
    pub floor_price: Money,
}

pub struct DutchAuction {
    pub config: DutchConfig,
    pub item: Item,
    bidders: Vec<BidderId>,
    pub current_price: Money,
    current_time: SimTime,
    /// Last price value we emitted a PriceChanged for (throttles spam events).
    last_event_price: Money,
    phase: AuctionPhase,
    outcome: Option<AuctionOutcome>,
}

impl DutchAuction {
    pub fn new(config: DutchConfig, item: Item, bidders: Vec<BidderId>) -> Self {
        let start_price = config.start_price;
        DutchAuction {
            config,
            item,
            bidders,
            current_price: start_price,
            current_time: 0.0,
            last_event_price: start_price,
            phase: AuctionPhase::Bidding,
            outcome: None,
        }
    }
}

impl Auction for DutchAuction {
    fn auction_type(&self) -> AuctionType {
        AuctionType::Dutch
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
            auction_type: AuctionType::Dutch,
            item_id: self.item.id,
            current_price: Some(self.current_price),
            min_bid: self.current_price,  // caller bids at the current clock price
            standing_bidder: None,
            bid_count: 0,
            phase: self.phase,
            time_since_last_bid: self.current_time,
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
        // Accept any bid amount — the caller accepts whatever the current price is.
        let call_price = self.current_price;
        self.phase = AuctionPhase::Complete;

        let outcome = AuctionOutcome {
            allocations: vec![Allocation { bidder_id: bid.bidder_id, item_id: self.item.id }],
            payments: vec![Payment { bidder_id: bid.bidder_id, amount: call_price }],
            receipts: vec![],
            revenue: call_price,
            social_welfare: None,
            efficiency: None,
        };
        self.outcome = Some(outcome.clone());

        Ok(vec![
            AuctionEvent::BidAccepted { bid, new_standing: call_price },
            AuctionEvent::AuctionClosed,
            AuctionEvent::AllocationDecided(outcome),
        ])
    }

    fn tick(&mut self, delta: SimTime) -> Vec<AuctionEvent> {
        if self.phase != AuctionPhase::Bidding {
            return vec![];
        }
        self.current_time += delta;

        // Decrement price.
        let new_price = self.current_price - self.config.decrement_per_second * delta;

        if new_price <= self.config.floor_price {
            let old = self.last_event_price;
            self.current_price = self.config.floor_price;
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
                AuctionEvent::PriceChanged { old, new: self.current_price },
                AuctionEvent::AuctionClosed,
                AuctionEvent::AllocationDecided(outcome),
            ];
        }

        self.current_price = new_price;

        // Emit PriceChanged only when price drops by at least $1 to avoid flooding the log.
        let mut events = vec![];
        if self.last_event_price - self.current_price >= Money(1.0) {
            events.push(AuctionEvent::PriceChanged {
                old: self.last_event_price,
                new: self.current_price,
            });
            self.last_event_price = self.current_price;
        }
        events
    }

    fn outcome(&self) -> Option<&AuctionOutcome> {
        self.outcome.as_ref()
    }
}
