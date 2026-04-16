use rand::rngs::StdRng;
use rand::SeedableRng;

use auction_core::bid::{Bid, BidError};
use auction_core::bidder::BidderStrategy;
use auction_core::event::AuctionEvent;
use auction_core::mechanism::Auction;
use auction_core::outcome::AuctionOutcome;
use auction_core::types::{AuctionPhase, BidderId, Money, SimTime};

/// Bundles everything the engine needs to know about a single AI participant.
pub struct BidderConfig {
    pub id: BidderId,
    pub name: String,
    pub strategy: Box<dyn BidderStrategy>,
    pub value: Money,
}

/// Drives the simulation: advances the auction clock, invites AI bidders to act,
/// and logs every event.
pub struct SimulationEngine {
    pub auction: Box<dyn Auction>,
    pub bidder_ids: Vec<BidderId>,
    pub bidder_names: Vec<String>,
    /// Parallel to bidder_ids/bidder_names.
    strategies: Vec<Box<dyn BidderStrategy>>,
    values: Vec<Money>,
    pub current_time: SimTime,
    pub event_log: Vec<(SimTime, AuctionEvent)>,
    /// Default delta used by run_to_completion.
    pub tick_delta: SimTime,
    /// Minimum seconds between consecutive actions by the same AI bidder.
    pub think_time: SimTime,
    /// Tracks when each AI bidder last acted (index-parallel to strategies).
    pub last_action: Vec<SimTime>,
    rng: StdRng,
}

impl SimulationEngine {
    pub fn new(
        auction: Box<dyn Auction>,
        bidders: Vec<BidderConfig>,
        tick_delta: SimTime,
        think_time: SimTime,
    ) -> Self {
        let n = bidders.len();
        let bidder_ids = bidders.iter().map(|b| b.id).collect();
        let bidder_names = bidders.iter().map(|b| b.name.clone()).collect();
        let values = bidders.iter().map(|b| b.value).collect();
        let strategies = bidders.into_iter().map(|b| b.strategy).collect();
        SimulationEngine {
            auction,
            bidder_ids,
            bidder_names,
            strategies,
            values,
            current_time: 0.0,
            event_log: Vec::new(),
            tick_delta,
            think_time,
            last_action: vec![f64::NEG_INFINITY; n],
            rng: StdRng::seed_from_u64(42),
        }
    }

    /// Spread AI bidders' initial action times evenly across one think_time window
    /// so they don't all fire on the very first tick.
    pub fn stagger_starts(&mut self) {
        let n = self.strategies.len();
        if n == 0 {
            return;
        }
        for i in 0..n {
            // Bidder 0 acts first (after think_time seconds), bidder n-1 acts last.
            let frac = (n - i) as SimTime / n as SimTime;
            self.last_action[i] = -(self.think_time * frac);
        }
    }

    /// Advance the simulation by `delta` seconds.
    /// Returns all events generated during this step.
    pub fn tick(&mut self, delta: SimTime) -> Vec<AuctionEvent> {
        let mut events = Vec::new();

        // 1. Advance auction clock.
        let auction_events = self.auction.tick(delta);
        for e in &auction_events {
            self.event_log.push((self.current_time, e.clone()));
        }
        events.extend(auction_events);

        if self.auction.phase() == AuctionPhase::Complete {
            self.current_time += delta;
            return events;
        }

        // 2. Give each AI bidder a chance to act (state is refreshed per bidder so
        //    they see the effects of earlier bids within the same tick).
        for i in 0..self.strategies.len() {
            if self.current_time - self.last_action[i] < self.think_time {
                continue;
            }

            let state = self.auction.visible_state();
            let value = self.values[i];

            // self.strategies[i] and self.rng are disjoint fields — both borrows
            // are allowed simultaneously by Rust's field-level borrow checker.
            let bid_opt = self.strategies[i].decide(&state, value, &mut self.rng);

            if let Some(mut bid) = bid_opt {
                bid.timestamp = self.current_time;
                if let Ok(bid_events) = self.auction.submit_bid(bid) {
                    for e in &bid_events {
                        self.event_log.push((self.current_time, e.clone()));
                    }
                    events.extend(bid_events);
                } // rejected bids are silently dropped
            }

            // Update think timer whether or not a bid was placed.
            self.last_action[i] = self.current_time;
        }

        self.current_time += delta;
        events
    }

    /// Submit a bid on behalf of any registered bidder (e.g., the human player).
    /// The timestamp is set to the current simulation time.
    pub fn submit_bid_for(
        &mut self,
        bidder_id: BidderId,
        amount: Money,
    ) -> Result<Vec<AuctionEvent>, BidError> {
        let item_id = self.auction.item_id();
        let bid = Bid {
            bidder_id,
            item_id,
            amount,
            timestamp: self.current_time,
        };
        let events = self.auction.submit_bid(bid)?;
        for e in &events {
            self.event_log.push((self.current_time, e.clone()));
        }
        Ok(events)
    }

    /// Run the auction to completion (headless, no real-time delays).
    pub fn run_to_completion(&mut self) -> &[(SimTime, AuctionEvent)] {
        let delta = self.tick_delta;
        while self.auction.phase() != AuctionPhase::Complete {
            self.tick(delta);
            if self.current_time > 100_000.0 {
                break;
            }
        }
        &self.event_log
    }

    pub fn outcome(&self) -> Option<&AuctionOutcome> {
        self.auction.outcome()
    }

    /// Display name for a bidder, or "Unknown".
    pub fn name_of(&self, id: BidderId) -> &str {
        self.bidder_ids
            .iter()
            .position(|&b| b == id)
            .map(|i| self.bidder_names[i].as_str())
            .unwrap_or("Unknown")
    }
}
