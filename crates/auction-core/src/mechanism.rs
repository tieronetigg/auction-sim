use crate::bid::{Bid, BidError};
use crate::event::AuctionEvent;
use crate::outcome::AuctionOutcome;
use crate::types::{AuctionPhase, AuctionType, BidderId, ItemId, Money, SimTime};

/// The subset of auction state a bidder can legitimately observe.
/// Open formats (English, Dutch) expose current_price; sealed formats do not.
#[derive(Debug, Clone)]
pub struct VisibleAuctionState {
    pub auction_type: AuctionType,
    pub item_id: ItemId,
    /// Standing price (public in open auctions, None in sealed-bid phases).
    pub current_price: Option<Money>,
    /// Minimum amount for a valid next bid.
    pub min_bid: Money,
    /// Who holds the standing high bid, if anyone (None in sealed formats).
    pub standing_bidder: Option<BidderId>,
    /// Total bids accepted/submitted so far.
    pub bid_count: usize,
    pub phase: AuctionPhase,
    /// Seconds since the last accepted bid (meaningful for English; 0 for sealed).
    pub time_since_last_bid: SimTime,
    /// Bidders still eligible to bid (for sealed: those who haven't submitted yet).
    pub active_bidders: Vec<BidderId>,
    /// Seconds remaining until the submission deadline (sealed-bid only; None otherwise).
    pub deadline_remaining: Option<SimTime>,
}

/// Core interface every auction mechanism implements.
pub trait Auction {
    fn auction_type(&self) -> AuctionType;
    fn phase(&self) -> AuctionPhase;

    fn item_id(&self) -> ItemId;
    fn item_name(&self) -> &str;

    /// Returns the publicly observable state.
    fn visible_state(&self) -> VisibleAuctionState;

    /// Submit a bid. Returns resulting events on success, or an error.
    fn submit_bid(&mut self, bid: Bid) -> Result<Vec<AuctionEvent>, BidError>;

    /// Advance the auction clock by `delta` seconds.
    /// Returns any triggered events (timeout close, Dutch price drop, etc.).
    fn tick(&mut self, delta: SimTime) -> Vec<AuctionEvent>;

    /// Final outcome. Returns None until phase == Complete.
    fn outcome(&self) -> Option<&AuctionOutcome>;
}
