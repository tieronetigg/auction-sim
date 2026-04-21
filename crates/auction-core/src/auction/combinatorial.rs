use crate::event::AuctionEvent;
use crate::outcome::{Allocation, AuctionOutcome, Payment};
use crate::package::{welfare_max, Package, PackageBid};
use crate::types::{BidderId, Money, SimTime};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombinatorialPaymentRule {
    /// Each winner pays their own bid (pay-as-bid).
    PayAsBid,
    /// VCG: each winner pays the externality they impose on others.
    Vcg,
}

#[derive(Debug, Clone)]
pub struct CombinatorialConfig {
    pub payment_rule: CombinatorialPaymentRule,
    pub deadline: SimTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CombinatorialError {
    AuctionClosed,
    UnknownBidder,
}

/// Standalone combinatorial auction (XOR bidding, uniform deadline).
///
/// Does not implement the `Auction` trait — it has multi-item, multi-bid
/// semantics that don't fit the single-item `Auction` interface. Used
/// directly by tests and the headless example.
pub struct CombinatorialAuction {
    pub config: CombinatorialConfig,
    bids: Vec<PackageBid>,
    elapsed: SimTime,
    outcome: Option<CombinatorialOutcome>,
    bidder_ids: Vec<BidderId>,
}

#[derive(Debug, Clone)]
pub struct CombinatorialOutcome {
    /// AuctionOutcome for compatibility with display/test helpers.
    pub outcome: AuctionOutcome,
    /// VCG or pay-as-bid payments, indexed by winner.
    pub vcg_payments: Vec<VcgPayment>,
}

#[derive(Debug, Clone)]
pub struct VcgPayment {
    pub bidder_id: BidderId,
    pub amount: Money,
}

impl CombinatorialAuction {
    pub fn new(config: CombinatorialConfig, bidder_ids: Vec<BidderId>) -> Self {
        CombinatorialAuction {
            config,
            bids: Vec::new(),
            elapsed: 0.0,
            outcome: None,
            bidder_ids,
        }
    }

    pub fn is_open(&self) -> bool {
        self.outcome.is_none() && self.elapsed < self.config.deadline
    }

    pub fn submit_package_bid(
        &mut self,
        bidder_id: BidderId,
        package: Package,
        value: Money,
    ) -> Result<AuctionEvent, CombinatorialError> {
        if !self.is_open() {
            return Err(CombinatorialError::AuctionClosed);
        }
        if !self.bidder_ids.contains(&bidder_id) {
            return Err(CombinatorialError::UnknownBidder);
        }
        let bid = PackageBid { bidder_id, package, value, timestamp: self.elapsed };
        let event = AuctionEvent::PackageBidSubmitted(bid.clone());
        self.bids.push(bid);
        Ok(event)
    }

    pub fn tick(&mut self, delta: SimTime) {
        if self.outcome.is_some() {
            return;
        }
        self.elapsed += delta;
        if self.elapsed >= self.config.deadline {
            self.resolve();
        }
    }

    pub fn outcome(&self) -> Option<&CombinatorialOutcome> {
        self.outcome.as_ref()
    }

    fn resolve(&mut self) {
        let result = welfare_max(&self.bids, None);

        let mut allocations = Vec::new();
        let mut payments = Vec::new();
        let mut vcg_payments = Vec::new();
        let mut revenue = Money(0.0);

        for winning_bid in &result.winners {
            let payment_amount = match self.config.payment_rule {
                CombinatorialPaymentRule::PayAsBid => winning_bid.value,
                CombinatorialPaymentRule::Vcg => {
                    // p_i = W*_{-i} − (W* − v_i)
                    let w_star = result.welfare;
                    let w_without = welfare_max(&self.bids, Some(winning_bid.bidder_id)).welfare;
                    let externality = w_star - winning_bid.value;
                    let p = w_without - externality;
                    Money(p.0.max(0.0))
                }
            };

            for &item_id in &winning_bid.package.0 {
                allocations.push(Allocation { bidder_id: winning_bid.bidder_id, item_id });
            }
            payments.push(Payment { bidder_id: winning_bid.bidder_id, amount: payment_amount });
            vcg_payments.push(VcgPayment { bidder_id: winning_bid.bidder_id, amount: payment_amount });
            revenue = revenue + payment_amount;
        }

        self.outcome = Some(CombinatorialOutcome {
            outcome: AuctionOutcome {
                allocations,
                payments,
                receipts: vec![],
                revenue,
                social_welfare: None,
                efficiency: None,
            },
            vcg_payments,
        });
    }
}
