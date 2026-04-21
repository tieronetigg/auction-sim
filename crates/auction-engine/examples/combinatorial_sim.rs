/// Headless combinatorial auction driver — demonstrates both PayAsBid and VCG.
///
/// Scenario: 2 items (North=0, South=1), 3 bidders.
///   Alice (0): bundle {N,S} = $200
///   Bob   (1): N alone      = $70
///   Carol (2): S alone      = $50
use auction_core::auction::combinatorial::{
    CombinatorialAuction, CombinatorialConfig, CombinatorialPaymentRule,
};
use auction_core::package::Package;
use auction_core::types::{BidderId, ItemId, Money};

fn pkg(ids: &[u32]) -> Package {
    Package(ids.iter().copied().map(ItemId).collect())
}

fn run(rule: CombinatorialPaymentRule) {
    let label = match rule {
        CombinatorialPaymentRule::PayAsBid => "Pay-as-bid",
        CombinatorialPaymentRule::Vcg => "VCG",
    };
    println!("── {label} ─────────────────────────────────────────");

    let ids = vec![BidderId(0), BidderId(1), BidderId(2)];
    let mut a = CombinatorialAuction::new(
        CombinatorialConfig { payment_rule: rule, deadline: 10.0 },
        ids,
    );

    a.submit_package_bid(BidderId(0), pkg(&[0, 1]), Money(200.0)).unwrap(); // Alice: {N,S}
    a.submit_package_bid(BidderId(1), pkg(&[0]), Money(70.0)).unwrap();     // Bob: N
    a.submit_package_bid(BidderId(2), pkg(&[1]), Money(50.0)).unwrap();     // Carol: S

    a.tick(11.0);
    let co = a.outcome().unwrap();
    let outcome = &co.outcome;

    let names = ["Alice", "Bob", "Carol"];

    println!("Allocations:");
    for al in &outcome.allocations {
        println!("  {} wins item {}", names[al.bidder_id.0 as usize], al.item_id.0);
    }

    println!("Payments:");
    for p in &outcome.payments {
        println!("  {} pays {}", names[p.bidder_id.0 as usize], p.amount);
    }

    println!("Revenue: {}", outcome.revenue);
    println!();
}

fn main() {
    run(CombinatorialPaymentRule::PayAsBid);
    run(CombinatorialPaymentRule::Vcg);
}
