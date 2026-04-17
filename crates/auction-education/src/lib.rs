use auction_core::event::AuctionEvent;
use auction_core::mechanism::VisibleAuctionState;
use auction_core::outcome::AuctionOutcome;
use auction_core::types::{AuctionPhase, AuctionType, BidderId, Money, SimTime};

// ─────────────────────────────────────────────
// Hint generation
// ─────────────────────────────────────────────

/// Severity level for a live hint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HintLevel {
    /// Informational — strategy tips and status.
    Info,
    /// Caution — worth paying attention to.
    Caution,
    /// Urgent — action likely required now.
    Urgent,
}

/// Returns a context-sensitive hint for display during a live auction.
/// Returns `None` when the auction is complete or no relevant hint applies.
pub fn live_hint(
    auction_type: AuctionType,
    state: &VisibleAuctionState,
    human_id: BidderId,
    human_value: Money,
    bid_submitted: bool,
) -> Option<(HintLevel, String)> {
    if state.phase == AuctionPhase::Complete {
        return None;
    }

    match auction_type {
        AuctionType::English => hint_english(state, human_id, human_value),
        AuctionType::Dutch => hint_dutch(state, human_value),
        AuctionType::FirstPriceSealedBid => hint_fpsb(human_value, bid_submitted),
        AuctionType::Vickrey => hint_vickrey(human_value, bid_submitted),
        AuctionType::AllPay => hint_allpay(human_value, bid_submitted),
        AuctionType::Double => hint_double(human_value, bid_submitted),
        _ => None,
    }
}

fn hint_english(
    state: &VisibleAuctionState,
    human_id: BidderId,
    human_value: Money,
) -> Option<(HintLevel, String)> {
    let min_bid = state.min_bid;
    let current_price = state.current_price.unwrap_or(min_bid);

    // Highest-priority: price has passed value
    if min_bid > human_value {
        return Some((
            HintLevel::Urgent,
            format!("Min bid {} exceeds your value — drop out now", min_bid),
        ));
    }

    // Human currently leads
    if state.standing_bidder == Some(human_id) {
        if state.time_since_last_bid > 10.0 {
            return Some((
                HintLevel::Caution,
                format!(
                    "You lead at {} — {:.0}s silence, going once...",
                    current_price, state.time_since_last_bid
                ),
            ));
        }
        return Some((
            HintLevel::Info,
            format!("You hold the standing bid at {}", current_price),
        ));
    }

    // Silence warning when someone else leads
    if state.time_since_last_bid > 10.0 {
        return Some((
            HintLevel::Caution,
            format!(
                "Silence {:.0}s / 15s — auction may close soon",
                state.time_since_last_bid
            ),
        ));
    }

    Some((
        HintLevel::Info,
        format!("Min bid {} is below your value — bid to stay in", min_bid),
    ))
}

fn hint_dutch(state: &VisibleAuctionState, human_value: Money) -> Option<(HintLevel, String)> {
    let price = state.current_price?;

    if price <= human_value {
        return Some((
            HintLevel::Urgent,
            format!("Clock at {} — at or below your value, call NOW!", price),
        ));
    }

    let gap = price - human_value;
    if gap <= Money(20.0) {
        return Some((
            HintLevel::Caution,
            format!(
                "Only {} above your value — get ready to call",
                gap
            ),
        ));
    }

    Some((
        HintLevel::Info,
        format!(
            "Clock at {} — {} above your value, wait",
            price, gap
        ),
    ))
}

fn hint_fpsb(human_value: Money, bid_submitted: bool) -> Option<(HintLevel, String)> {
    if bid_submitted {
        return Some((
            HintLevel::Info,
            "Bid sealed — waiting for the deadline".to_string(),
        ));
    }
    // Bayes-Nash equilibrium with 6 bidders: (n-1)/n = 5/6 ~= 83%
    let eq_bid = Money(human_value.0 * 5.0 / 6.0);
    Some((
        HintLevel::Info,
        format!(
            "Equilibrium bid (6 bidders): ~83% of your value ~ {} — shade below true value",
            eq_bid
        ),
    ))
}

fn hint_vickrey(human_value: Money, bid_submitted: bool) -> Option<(HintLevel, String)> {
    if bid_submitted {
        return Some((
            HintLevel::Info,
            "Bid sealed — you pay second-highest price if you win".to_string(),
        ));
    }
    Some((
        HintLevel::Info,
        format!(
            "Dominant strategy: bid your true value {} — no reason to shade",
            human_value
        ),
    ))
}

fn hint_allpay(human_value: Money, bid_submitted: bool) -> Option<(HintLevel, String)> {
    if bid_submitted {
        return Some((
            HintLevel::Info,
            "Bid locked — you pay it win or lose; highest bidder wins the item".to_string(),
        ));
    }
    // BNE with n=6, H=500: b(v) = (5/6) * v * (v/500)^5
    let n = 6.0_f64;
    let h = 500.0_f64;
    let v = human_value.0;
    let eq_bid = Money(((n - 1.0) / n) * v * (v / h).powf(n - 1.0));
    Some((
        HintLevel::Info,
        format!(
            "BNE bid (6 bidders, H=$500): {} — far below value; everyone pays their bid",
            eq_bid
        ),
    ))
}

fn hint_double(human_value: Money, bid_submitted: bool) -> Option<(HintLevel, String)> {
    if bid_submitted {
        return Some((
            HintLevel::Info,
            "Order locked — clearing price set at deadline; you trade only if bid ≥ ask".to_string(),
        ));
    }
    Some((
        HintLevel::Info,
        format!(
            "Bid near your true value {} — your bid can influence the clearing price",
            human_value
        ),
    ))
}

// ─────────────────────────────────────────────
// Price series (sparkline data)
// ─────────────────────────────────────────────

/// Extract a price series from the event log for sparkline rendering.
/// Values are returned as integer cents (value × 100) for ratatui's Sparkline u64 data.
///
/// - English: standing price at each `BidAccepted` event (ascending series)
/// - Dutch: clock price at each `PriceChanged` event (descending series)
/// - Sealed: empty — no real-time price signal
pub fn price_series(
    event_log: &[(SimTime, AuctionEvent)],
    auction_type: AuctionType,
) -> Vec<u64> {
    match auction_type {
        AuctionType::English => event_log
            .iter()
            .filter_map(|(_, e)| match e {
                AuctionEvent::BidAccepted { new_standing, .. } => {
                    Some((new_standing.0 * 100.0) as u64)
                }
                _ => None,
            })
            .collect(),

        AuctionType::Dutch => event_log
            .iter()
            .filter_map(|(_, e)| match e {
                AuctionEvent::PriceChanged { new, .. } => Some((new.0 * 100.0) as u64),
                _ => None,
            })
            .collect(),

        _ => vec![],
    }
}

// ─────────────────────────────────────────────
// Debrief analysis
// ─────────────────────────────────────────────

/// Returns analysis lines for the debrief: efficiency verdict and human surplus.
///
/// `ai_info` — `(id, display_name, true_value)` for each AI bidder.
pub fn debrief_insights(
    auction_type: AuctionType,
    outcome: &AuctionOutcome,
    human_id: BidderId,
    human_value: Money,
    ai_info: &[(BidderId, &str, Money)],
) -> Vec<String> {
    let mut lines = Vec::new();

    // ── Double auction: multi-trade framing ───────────────────────────────
    if auction_type == AuctionType::Double {
        let n_trades = outcome.allocations.len();
        let clearing = outcome.payments.first().map(|p| p.amount).unwrap_or(Money::zero());
        let human_traded = outcome.allocations.iter().any(|a| a.bidder_id == human_id);

        if n_trades > 0 {
            lines.push(format!("{} trade(s) cleared at uniform price {}.", n_trades, clearing));
        } else {
            lines.push("No trades cleared — no bid met any ask.".to_string());
        }

        // Derive competitive equilibrium range from true values.
        // Buyers: human + ai_info where id.0 < 4; sellers: ai_info where id.0 >= 4.
        let mut buyer_vals: Vec<f64> = vec![human_value.0];
        let mut seller_vals: Vec<f64> = Vec::new();
        for &(id, _, value) in ai_info {
            if id.0 >= 4 {
                seller_vals.push(value.0);
            } else {
                buyer_vals.push(value.0);
            }
        }
        buyer_vals.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal)); // desc
        seller_vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)); // asc
        let n_eq = buyer_vals
            .iter()
            .zip(seller_vals.iter())
            .take_while(|(b, s)| b >= s)
            .count();
        if n_eq > 0 {
            let ce_lo = Money(seller_vals[n_eq - 1]);
            let ce_hi = Money(buyer_vals[n_eq - 1]);
            let ce_mid = Money((ce_lo.0 + ce_hi.0) / 2.0);
            lines.push(format!(
                "Competitive equilibrium: {} trade(s), price range {}–{} \
                 (marginal seller ask / marginal buyer value).",
                n_eq, ce_lo, ce_hi
            ));
            lines.push(format!(
                "k=0.5 sets clearing at midpoint of the marginal pair — theory predicts ~{}.",
                ce_mid
            ));
            if n_trades > 0 && clearing.0 >= ce_lo.0 && clearing.0 <= ce_hi.0 {
                lines.push("Actual clearing is within the CE range — efficient outcome.".to_string());
            }
        }

        if human_traded {
            let surplus = human_value - clearing;
            lines.push(format!(
                "Your surplus: {} (value {} minus clearing price {}).",
                surplus, human_value, clearing
            ));
        } else {
            lines.push(format!(
                "You did not trade — your bid was at or below the clearing price ({}).",
                clearing
            ));
        }

        return lines;
    }

    // ── Find highest-value bidder ──────────────────────────────────────────
    let mut highest_id = human_id;
    let mut highest_value = human_value;
    for &(id, _, value) in ai_info {
        if value > highest_value {
            highest_id = id;
            highest_value = value;
        }
    }

    let highest_name: String = if highest_id == human_id {
        "You".to_string()
    } else {
        ai_info
            .iter()
            .find(|(id, _, _)| *id == highest_id)
            .map(|(_, name, _)| name.to_string())
            .unwrap_or_else(|| "Unknown".to_string())
    };

    // ── Efficiency verdict ─────────────────────────────────────────────────
    let winner = outcome.allocations.first().map(|a| a.bidder_id);
    match winner {
        Some(w) if w == highest_id => {
            lines.push(format!(
                "Efficient: the highest-value bidder ({}, {}) won.",
                highest_name, highest_value
            ));
        }
        Some(w) => {
            let winner_name: String = if w == human_id {
                "You".to_string()
            } else {
                ai_info
                    .iter()
                    .find(|(id, _, _)| *id == w)
                    .map(|(_, name, _)| name.to_string())
                    .unwrap_or_else(|| "Unknown".to_string())
            };
            lines.push(format!(
                "Inefficient: {} (value {}) won, but {} had the highest value ({}).",
                winner_name, value_of(w, human_id, human_value, ai_info),
                highest_name, highest_value
            ));
        }
        None => {
            lines.push("No winner — reserve price was not met.".to_string());
        }
    }

    // ── Human surplus ──────────────────────────────────────────────────────
    let human_won = winner == Some(human_id);
    let human_payment = outcome
        .payments
        .iter()
        .find(|p| p.bidder_id == human_id)
        .map(|p| p.amount)
        .unwrap_or(Money::zero());

    if human_won {
        let surplus = human_value - human_payment;
        lines.push(format!(
            "Your surplus: {} (value {} minus payment {}).",
            surplus, human_value, human_payment
        ));
    } else if auction_type == AuctionType::AllPay && human_payment > Money::zero() {
        // All-pay: losers forfeit their bid with nothing to show.
        lines.push(format!(
            "You did not win — you paid {} and received nothing (net: -{}).",
            human_payment, human_payment
        ));
    } else {
        lines.push("You did not win — your surplus is $0.00.".to_string());
    }

    lines
}

fn value_of(
    id: BidderId,
    human_id: BidderId,
    human_value: Money,
    ai_info: &[(BidderId, &str, Money)],
) -> Money {
    if id == human_id {
        return human_value;
    }
    ai_info
        .iter()
        .find(|(aid, _, _)| *aid == id)
        .map(|(_, _, v)| *v)
        .unwrap_or(Money::zero())
}

// ─────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use auction_core::outcome::{Allocation, Payment, Receipt};
    use auction_core::types::ItemId;

    fn outcome(allocations: Vec<(u32, f64)>, payments: Vec<(u32, f64)>) -> AuctionOutcome {
        AuctionOutcome {
            allocations: allocations
                .into_iter()
                .map(|(id, _)| Allocation { bidder_id: BidderId(id), item_id: ItemId(0) })
                .collect(),
            payments: payments
                .into_iter()
                .map(|(id, amt)| Payment { bidder_id: BidderId(id), amount: Money(amt) })
                .collect(),
            receipts: vec![],
            revenue: Money::zero(),
            social_welfare: None,
            efficiency: None,
        }
    }

    // ── All-pay loser ──────────────────────────────────────────────────────────

    /// An all-pay loser should get a line mentioning what they paid, not "$0.00 surplus".
    #[test]
    fn allpay_loser_shows_payment_not_zero_surplus() {
        let o = outcome(vec![(1, 100.0)], vec![(0, 50.0), (1, 100.0)]);
        let ai = vec![(BidderId(1), "Alice", Money(150.0))];
        let lines = debrief_insights(AuctionType::AllPay, &o, BidderId(0), Money(80.0), &ai);

        let combined = lines.join(" ");
        assert!(combined.contains("$50.00"), "expected paid amount in output: {combined}");
        assert!(!combined.contains("$0.00"), "must not show zero surplus for all-pay loser: {combined}");
    }

    /// The all-pay winner gets a surplus line, not a "paid and received nothing" line.
    #[test]
    fn allpay_winner_shows_surplus() {
        let o = outcome(vec![(0, 100.0)], vec![(0, 100.0), (1, 60.0)]);
        let ai = vec![(BidderId(1), "Alice", Money(80.0))];
        let lines = debrief_insights(AuctionType::AllPay, &o, BidderId(0), Money(150.0), &ai);

        let combined = lines.join(" ");
        assert!(combined.contains("surplus"), "winner should have surplus line: {combined}");
    }

    // ── Double auction CE range ────────────────────────────────────────────────

    /// The competitive equilibrium count and price range are derived correctly
    /// from true bidder values, independent of the actual submitted bids.
    ///
    /// Setup mirrors the live game:
    ///   Buyers: human $100, Alice (1) $120, Bob (2) $110, Carol (3) $90
    ///   Sellers: Dave (4) $35, Eve (5) $60, Fiona (6) $80, Grant (7) $105
    ///   Sorted buyers desc: $120, $110, $100, $90
    ///   Sorted sellers asc: $35,  $60,  $80,  $105
    ///   Crossing pairs:     (120,35) (110,60) (100,80) — 3 cross; (90,105) doesn't
    ///   CE range: [$80, $100], theory midpoint $90
    #[test]
    fn double_ce_range_from_live_game_values() {
        // Clearing at $90 is within the CE range; outcome is 3 trades.
        let o = AuctionOutcome {
            allocations: vec![
                Allocation { bidder_id: BidderId(0), item_id: ItemId(0) },
                Allocation { bidder_id: BidderId(1), item_id: ItemId(0) },
                Allocation { bidder_id: BidderId(2), item_id: ItemId(0) },
            ],
            payments: vec![
                Payment { bidder_id: BidderId(0), amount: Money(90.0) },
                Payment { bidder_id: BidderId(1), amount: Money(90.0) },
                Payment { bidder_id: BidderId(2), amount: Money(90.0) },
            ],
            receipts: vec![
                Receipt { bidder_id: BidderId(4), amount: Money(90.0) },
                Receipt { bidder_id: BidderId(5), amount: Money(90.0) },
                Receipt { bidder_id: BidderId(6), amount: Money(90.0) },
            ],
            revenue: Money(270.0),
            social_welfare: None,
            efficiency: None,
        };

        let ai = vec![
            (BidderId(1), "Alice",  Money(120.0)),
            (BidderId(2), "Bob",    Money(110.0)),
            (BidderId(3), "Carol",  Money(90.0)),
            (BidderId(4), "Dave",   Money(35.0)),
            (BidderId(5), "Eve",    Money(60.0)),
            (BidderId(6), "Fiona",  Money(80.0)),
            (BidderId(7), "Grant",  Money(105.0)),
        ];

        let lines = debrief_insights(AuctionType::Double, &o, BidderId(0), Money(100.0), &ai);
        let combined = lines.join(" ");

        assert!(combined.contains("3"), "should mention 3 trades: {combined}");
        assert!(combined.contains("$80.00"), "CE lower bound should be $80: {combined}");
        assert!(combined.contains("$100.00"), "CE upper bound should be $100: {combined}");
        assert!(combined.contains("$90.00"), "theory midpoint should be $90: {combined}");
        assert!(combined.contains("efficient"), "outcome within CE range should be flagged efficient: {combined}");
    }

    /// When no pairs cross, the CE range is empty and no trades occur.
    #[test]
    fn double_no_crossing_no_ce() {
        let o = AuctionOutcome {
            allocations: vec![],
            payments: vec![],
            receipts: vec![],
            revenue: Money::zero(),
            social_welfare: None,
            efficiency: None,
        };
        let ai = vec![
            (BidderId(1), "Alice", Money(50.0)),   // buyer; value < seller ask
            (BidderId(2), "Dave",  Money(200.0)),  // seller; ask > buyer value
        ];

        let lines = debrief_insights(AuctionType::Double, &o, BidderId(0), Money(40.0), &ai);
        let combined = lines.join(" ");
        assert!(combined.contains("No trades"), "should report no trades: {combined}");
    }
}
