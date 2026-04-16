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
