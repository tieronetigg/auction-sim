use std::collections::BTreeSet;

use crate::types::{BidderId, ItemId, Money, SimTime};

/// An unordered, deduplicated set of items offered as a single lot.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Package(pub BTreeSet<ItemId>);

impl Package {
    pub fn single(id: ItemId) -> Self {
        Package(std::iter::once(id).collect())
    }

    pub fn is_disjoint(&self, other: &Package) -> bool {
        self.0.is_disjoint(&other.0)
    }
}

/// A bid on a package: willing to pay `value` for exactly this set of items.
#[derive(Debug, Clone)]
pub struct PackageBid {
    pub bidder_id: BidderId,
    pub package: Package,
    pub value: Money,
    pub timestamp: SimTime,
}

/// Result of welfare maximisation: the winning allocation and total social welfare.
pub struct WelfareResult<'a> {
    pub winners: Vec<&'a PackageBid>,
    pub welfare: Money,
}

/// Brute-force XOR winner determination.
///
/// Enumerates all 2^n subsets of `bids`. A subset is *feasible* if no two bids
/// in it share an item AND at most one bid per bidder is selected. Returns the
/// feasible subset with the highest total value. When `exclude` is `Some(id)`,
/// all bids from that bidder are ignored (used to compute VCG payments).
pub fn welfare_max<'a>(bids: &'a [PackageBid], exclude: Option<BidderId>) -> WelfareResult<'a> {
    let candidates: Vec<&'a PackageBid> = bids
        .iter()
        .filter(|b| exclude != Some(b.bidder_id))
        .collect();

    let n = candidates.len();
    let mut best_welfare = Money(0.0);
    let mut best_mask: u64 = 0;

    for mask in 0u64..(1u64 << n) {
        let selected: Vec<&PackageBid> = (0..n)
            .filter(|&i| mask & (1 << i) != 0)
            .map(|i| candidates[i])
            .collect();

        if !is_feasible(&selected) {
            continue;
        }

        let welfare: Money = selected.iter().fold(Money(0.0), |acc, b| acc + b.value);
        if welfare.0 > best_welfare.0 {
            best_welfare = welfare;
            best_mask = mask;
        }
    }

    let winners = (0..n)
        .filter(|&i| best_mask & (1 << i) != 0)
        .map(|i| candidates[i])
        .collect();

    WelfareResult { winners, welfare: best_welfare }
}

fn is_feasible(bids: &[&PackageBid]) -> bool {
    // At most one bid per bidder.
    for i in 0..bids.len() {
        for j in (i + 1)..bids.len() {
            if bids[i].bidder_id == bids[j].bidder_id {
                return false;
            }
        }
    }
    // No two bids share an item.
    for i in 0..bids.len() {
        for j in (i + 1)..bids.len() {
            if !bids[i].package.is_disjoint(&bids[j].package) {
                return false;
            }
        }
    }
    true
}
