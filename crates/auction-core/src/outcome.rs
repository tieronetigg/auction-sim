use crate::types::{BidderId, ItemId, Money};

#[derive(Debug, Clone)]
pub struct Allocation {
    pub bidder_id: BidderId,
    pub item_id: ItemId,
}

#[derive(Debug, Clone)]
pub struct Payment {
    pub bidder_id: BidderId,
    pub amount: Money,
}

/// Amount received by a seller (used in double auctions).
#[derive(Debug, Clone)]
pub struct Receipt {
    pub bidder_id: BidderId,
    pub amount: Money,
}

#[derive(Debug, Clone)]
pub struct AuctionOutcome {
    pub allocations: Vec<Allocation>,
    pub payments: Vec<Payment>,
    /// Amounts received by sellers. Empty for single-sided auctions.
    pub receipts: Vec<Receipt>,
    pub revenue: Money,
    /// Sum of winners' true values. Set after value reveal in debrief.
    pub social_welfare: Option<f64>,
    /// social_welfare / optimal_welfare. Set after value reveal.
    pub efficiency: Option<f64>,
}
