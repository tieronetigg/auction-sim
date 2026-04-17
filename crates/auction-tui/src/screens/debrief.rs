use auction_core::event::AuctionEvent;
use auction_core::outcome::AuctionOutcome;
use auction_core::types::{AuctionType, BidderId, Money, SimTime};
use auction_education;
use std::collections::HashSet;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::screens::auction::AiInfo;

pub struct DebriefState {
    pub outcome: AuctionOutcome,
    pub auction_type: AuctionType,
    pub human_id: BidderId,
    pub human_value: Money,
    pub ai_info: Vec<AiInfo>,
    /// Reserve price enforced by this auction, if any (English only in current set).
    pub reserve_price: Option<Money>,
    /// BidAccepted events (English, Dutch winner) in chronological order.
    pub accepted_bids: Vec<(SimTime, BidderId, Money)>,
    /// BidSubmitted events (FPSB, Vickrey, AllPay) — revealed at debrief.
    pub sealed_bids: Vec<(BidderId, Money)>,
    /// AskSubmitted events (Double auction sellers) — revealed at debrief.
    pub sealed_asks: Vec<(BidderId, Money)>,
    pub scroll: u16,
}

impl DebriefState {
    pub fn scroll_down(&mut self) {
        self.scroll = self.scroll.saturating_add(3);
    }

    pub fn scroll_up(&mut self) {
        self.scroll = self.scroll.saturating_sub(3);
    }
}

impl DebriefState {
    pub fn build(
        outcome: AuctionOutcome,
        auction_type: AuctionType,
        human_id: BidderId,
        human_value: Money,
        ai_info: Vec<AiInfo>,
        reserve_price: Option<Money>,
        event_log: &[(SimTime, AuctionEvent)],
    ) -> Self {
        let accepted_bids = event_log
            .iter()
            .filter_map(|(t, e)| match e {
                AuctionEvent::BidAccepted { bid, new_standing } => {
                    Some((*t, bid.bidder_id, *new_standing))
                }
                _ => None,
            })
            .collect();

        let sealed_bids = event_log
            .iter()
            .filter_map(|(_, e)| match e {
                AuctionEvent::BidSubmitted(bid) => Some((bid.bidder_id, bid.amount)),
                _ => None,
            })
            .collect();

        let sealed_asks = event_log
            .iter()
            .filter_map(|(_, e)| match e {
                AuctionEvent::AskSubmitted(bid) => Some((bid.bidder_id, bid.amount)),
                _ => None,
            })
            .collect();

        DebriefState {
            outcome,
            auction_type,
            human_id,
            human_value,
            ai_info,
            reserve_price,
            accepted_bids,
            sealed_bids,
            sealed_asks,
            scroll: 0,
        }
    }
}

pub fn render(frame: &mut Frame, state: &DebriefState) {
    let area = frame.size();

    let outer = Block::default()
        .title(" Auction Result & Theory Debrief ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(""));

    // ── Result ──────────────────────────────────────────────────────────────
    lines.push(Line::from(vec![Span::styled(
        "  RESULT",
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
    )]));
    lines.push(Line::from(""));

    if state.auction_type == AuctionType::Double {
        // Double auction: multiple trades at a uniform clearing price.
        let n_trades = state.outcome.allocations.len();
        let clearing = state.outcome.payments.first().map(|p| p.amount).unwrap_or(Money::zero());
        let human_traded = state.outcome.allocations.iter().any(|a| a.bidder_id == state.human_id);

        lines.push(Line::from(vec![
            Span::styled("  Trades   : ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{} matched", n_trades), Style::default().fg(Color::White)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  Clearing : ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{}", clearing), Style::default().fg(Color::White)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  Revenue  : ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{}", state.outcome.revenue), Style::default().fg(Color::White)),
        ]));
        lines.push(Line::from(""));
        if human_traded {
            let surplus = state.human_value - clearing;
            lines.push(Line::from(vec![Span::styled(
                format!(
                    "  You traded — paid {}, value {}, surplus {}",
                    clearing, state.human_value, surplus
                ),
                Style::default().fg(Color::Cyan),
            )]));
        } else {
            lines.push(Line::from(vec![Span::styled(
                format!("  You did not trade — bid was below clearing price {}", clearing),
                Style::default().fg(Color::DarkGray),
            )]));
        }
    } else if let Some(alloc) = state.outcome.allocations.first() {
        let winner_name = name_of(alloc.bidder_id, state);
        let payment = winner_payment(&state.outcome, alloc.bidder_id);
        let winner_value = true_value_of(alloc.bidder_id, state);
        let is_human_win = alloc.bidder_id == state.human_id;

        let winner_style = if is_human_win {
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
        };

        lines.push(Line::from(vec![
            Span::styled("  Winner   : ", Style::default().fg(Color::DarkGray)),
            Span::styled(winner_name.clone(), winner_style),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  Paid     : ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}", payment),
                Style::default().fg(Color::White),
            ),
            Span::styled(
                format!("  (true value: {})", winner_value),
                Style::default().fg(Color::DarkGray),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  Surplus  : ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}", winner_value - payment),
                Style::default().fg(Color::Green),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  Revenue  : ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}", state.outcome.revenue),
                Style::default().fg(Color::White),
            ),
        ]));

        if let Some(r) = state.reserve_price {
            lines.push(Line::from(vec![
                Span::styled("  Reserve  : ", Style::default().fg(Color::DarkGray)),
                Span::styled(format!("{} — met", r), Style::default().fg(Color::DarkGray)),
            ]));
        }
        if is_human_win {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![Span::styled(
                "  Congratulations — you won the auction!",
                Style::default().fg(Color::Cyan),
            )]));
        }
    } else if let Some(r) = state.reserve_price {
        lines.push(Line::from(vec![Span::styled(
            format!("  No winner — reserve price {} not met.", r),
            Style::default().fg(Color::DarkGray),
        )]));
    } else {
        lines.push(Line::from(vec![Span::styled(
            "  No winner — reserve price was not met.",
            Style::default().fg(Color::DarkGray),
        )]));
    }

    // ── Analysis ────────────────────────────────────────────────────────────
    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        "  ANALYSIS",
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
    )]));
    lines.push(Line::from(""));

    let ai_info: Vec<(BidderId, &str, Money)> = state
        .ai_info
        .iter()
        .map(|a| (a.id, a.name.as_str(), a.value))
        .collect();
    let insights = auction_education::debrief_insights(
        state.auction_type,
        &state.outcome,
        state.human_id,
        state.human_value,
        &ai_info,
    );
    for line in &insights {
        lines.push(Line::from(vec![Span::styled(
            format!("  {}", line),
            Style::default().fg(Color::White),
        )]));
    }

    // ── Per-bidder summary ───────────────────────────────────────────────────
    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        "  BIDDER SUMMARY  (true values now revealed)",
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
    )]));
    lines.push(Line::from(""));

    let is_sealed = matches!(
        state.auction_type,
        AuctionType::FirstPriceSealedBid | AuctionType::Vickrey | AuctionType::AllPay | AuctionType::Double
    );

    // Human row.
    let human_bid = if is_sealed {
        state
            .sealed_bids
            .iter()
            .find(|(id, _)| *id == state.human_id)
            .map(|(_, amt)| *amt)
    } else {
        state
            .accepted_bids
            .iter()
            .filter(|(_, id, _)| *id == state.human_id)
            .map(|(_, _, amt)| *amt)
            .last()
    };

    lines.push(format_bidder_row("You", state.human_value, human_bid, false, state));

    for ai in &state.ai_info {
        let ai_bid = if is_sealed {
            // For Double auction, sellers' bids are in sealed_asks.
            let from_asks = state
                .sealed_asks
                .iter()
                .find(|(id, _)| *id == ai.id)
                .map(|(_, amt)| *amt);
            let from_bids = state
                .sealed_bids
                .iter()
                .find(|(id, _)| *id == ai.id)
                .map(|(_, amt)| *amt);
            from_bids.or(from_asks)
        } else {
            state
                .accepted_bids
                .iter()
                .filter(|(_, id, _)| *id == ai.id)
                .map(|(_, _, amt)| *amt)
                .last()
        };
        lines.push(format_bidder_row(&ai.name, ai.value, ai_bid, true, state));
    }

    // ── Theory note ─────────────────────────────────────────────────────────
    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        "  THEORY",
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
    )]));
    lines.push(Line::from(""));

    match state.auction_type {
        AuctionType::English => render_theory_english(&mut lines, state),
        AuctionType::Dutch => render_theory_dutch(&mut lines, state),
        AuctionType::FirstPriceSealedBid => render_theory_fpsb(&mut lines, state),
        AuctionType::Vickrey => render_theory_vickrey(&mut lines, state),
        AuctionType::AllPay => render_theory_allpay(&mut lines, state),
        AuctionType::Double => render_theory_double(&mut lines, state),
        _ => {}
    }

    let para = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((state.scroll, 0));
    frame.render_widget(para, chunks[0]);

    let footer = Line::from(vec![
        Span::styled("  ↑/↓  scroll     ", Style::default().fg(Color::DarkGray)),
        Span::styled("any other key", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        Span::styled("  return to menu", Style::default().fg(Color::DarkGray)),
    ]);
    frame.render_widget(Paragraph::new(footer), chunks[1]);
}

// ── Theory sections ───────────────────────────────────────────────────────────

fn render_theory_english(lines: &mut Vec<Line>, state: &DebriefState) {
    if state.outcome.allocations.first().is_some() {
        let payment = state
            .outcome
            .payments
            .first()
            .map(|p| p.amount)
            .unwrap_or(Money::zero());

        let mut all_values: Vec<Money> = state.ai_info.iter().map(|a| a.value).collect();
        all_values.push(state.human_value);
        all_values.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        let second_highest = all_values.get(1).copied().unwrap_or(Money::zero());

        lines.push(Line::from(vec![Span::styled(
            format!(
                "  The winner paid {} — close to the second-highest true value ({}).",
                payment, second_highest
            ),
            Style::default().fg(Color::White),
        )]));
        lines.push(Line::from(""));
    }
    lines.push(Line::from(vec![Span::styled(
        "  Revenue equivalence: an English auction yields the same expected",
        Style::default().fg(Color::DarkGray),
    )]));
    lines.push(Line::from(vec![Span::styled(
        "  revenue as a Vickrey (second-price sealed-bid) auction.",
        Style::default().fg(Color::DarkGray),
    )]));

    if let Some(reserve) = state.reserve_price {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            format!("  Reserve price: {} — the seller's floor.", reserve),
            Style::default().fg(Color::DarkGray),
        )]));

        // Name any bidder whose true value falls below the reserve.
        let excluded: Vec<&str> = state
            .ai_info
            .iter()
            .filter(|a| a.value.0 < reserve.0)
            .map(|a| a.name.as_str())
            .collect();
        if !excluded.is_empty() {
            lines.push(Line::from(vec![Span::styled(
                format!(
                    "  Excluded this run: {} (value < {}).",
                    excluded.join(", "),
                    reserve
                ),
                Style::default().fg(Color::DarkGray),
            )]));
        }
        lines.push(Line::from(vec![Span::styled(
            "  Efficiency tradeoff: if the highest-value bidder were below",
            Style::default().fg(Color::DarkGray),
        )]));
        lines.push(Line::from(vec![Span::styled(
            "  the reserve, the item goes unsold — welfare destroyed to",
            Style::default().fg(Color::DarkGray),
        )]));
        lines.push(Line::from(vec![Span::styled(
            "  raise the seller's expected revenue from high-value buyers.",
            Style::default().fg(Color::DarkGray),
        )]));
    }
}

fn render_theory_dutch(lines: &mut Vec<Line>, state: &DebriefState) {
    if let Some(alloc) = state.outcome.allocations.first() {
        let payment = state
            .outcome
            .payments
            .first()
            .map(|p| p.amount)
            .unwrap_or(Money::zero());
        let winner_value = true_value_of(alloc.bidder_id, state);

        lines.push(Line::from(vec![Span::styled(
            format!(
                "  The caller paid {} — their true value was {}.",
                payment, winner_value
            ),
            Style::default().fg(Color::White),
        )]));
        lines.push(Line::from(""));
    }
    lines.push(Line::from(vec![Span::styled(
        "  Strategic equivalence: a Dutch auction is strategically identical",
        Style::default().fg(Color::DarkGray),
    )]));
    lines.push(Line::from(vec![Span::styled(
        "  to a First-Price Sealed-Bid auction. Both yield the same expected",
        Style::default().fg(Color::DarkGray),
    )]));
    lines.push(Line::from(vec![Span::styled(
        "  revenue. The optimal call price is (n-1)/n × true value.",
        Style::default().fg(Color::DarkGray),
    )]));
}

fn render_theory_fpsb(lines: &mut Vec<Line>, state: &DebriefState) {
    if let Some(alloc) = state.outcome.allocations.first() {
        let payment = state
            .outcome
            .payments
            .first()
            .map(|p| p.amount)
            .unwrap_or(Money::zero());
        let winner_value = true_value_of(alloc.bidder_id, state);

        lines.push(Line::from(vec![Span::styled(
            format!(
                "  The winner paid their own bid of {} (true value: {}).",
                payment, winner_value
            ),
            Style::default().fg(Color::White),
        )]));
        lines.push(Line::from(""));
    }
    lines.push(Line::from(vec![Span::styled(
        "  Bid shading: with 6 bidders and uniform values, the Bayes-Nash",
        Style::default().fg(Color::DarkGray),
    )]));
    lines.push(Line::from(vec![Span::styled(
        "  equilibrium bid is (n-1)/n × value = 5/6 ≈ 83% of true value.",
        Style::default().fg(Color::DarkGray),
    )]));
    lines.push(Line::from(vec![Span::styled(
        "  Bidding your true value yields zero surplus when you win.",
        Style::default().fg(Color::DarkGray),
    )]));
}

fn render_theory_vickrey(lines: &mut Vec<Line>, state: &DebriefState) {
    if let Some(alloc) = state.outcome.allocations.first() {
        let payment = state
            .outcome
            .payments
            .first()
            .map(|p| p.amount)
            .unwrap_or(Money::zero());
        let winner_value = true_value_of(alloc.bidder_id, state);

        // Find second-highest bid for comparison.
        let mut bids: Vec<Money> = state.sealed_bids.iter().map(|(_, m)| *m).collect();
        bids.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        let second_bid = bids.get(1).copied().unwrap_or(Money::zero());

        lines.push(Line::from(vec![Span::styled(
            format!(
                "  Winner's true value: {}  |  Paid: {}  (second-highest bid: {})",
                winner_value, payment, second_bid
            ),
            Style::default().fg(Color::White),
        )]));
        lines.push(Line::from(""));
    }
    lines.push(Line::from(vec![Span::styled(
        "  Truth dominance: bidding your true value is weakly dominant in",
        Style::default().fg(Color::DarkGray),
    )]));
    lines.push(Line::from(vec![Span::styled(
        "  Vickrey. Deviating up risks winning at a loss; deviating down",
        Style::default().fg(Color::DarkGray),
    )]));
    lines.push(Line::from(vec![Span::styled(
        "  risks losing a profitable auction.",
        Style::default().fg(Color::DarkGray),
    )]));
}

fn render_theory_allpay(lines: &mut Vec<Line>, state: &DebriefState) {
    // Summarise all payments.
    let total_paid: Money = state.sealed_bids.iter().fold(Money::zero(), |acc, (_, b)| acc + *b);

    lines.push(Line::from(vec![Span::styled(
        format!("  Total revenue (sum of all bids): {}", total_paid),
        Style::default().fg(Color::White),
    )]));
    lines.push(Line::from(""));

    lines.push(Line::from(vec![Span::styled(
        "  All-pay payments (winner and losers):",
        Style::default().fg(Color::DarkGray),
    )]));
    let mut sorted = state.sealed_bids.clone();
    sorted.sort_by(|a, b| b.1.0.partial_cmp(&a.1.0).unwrap_or(std::cmp::Ordering::Equal));
    let winner = state.outcome.allocations.first().map(|a| a.bidder_id);
    for (id, amt) in &sorted {
        let name = name_of(*id, state);
        let is_winner = winner == Some(*id);
        let marker = if is_winner { "  (WINNER)" } else { "" };
        lines.push(Line::from(vec![
            Span::styled(format!("    {:<8}", name), Style::default().fg(Color::DarkGray)),
            Span::styled(format!("paid: {}{}", amt, marker), Style::default().fg(Color::White)),
        ]));
    }
    lines.push(Line::from(""));

    lines.push(Line::from(vec![Span::styled(
        "  Revenue equivalence: expected all-pay revenue equals expected",
        Style::default().fg(Color::DarkGray),
    )]));
    lines.push(Line::from(vec![Span::styled(
        "  FPSB revenue under symmetric independent private values.",
        Style::default().fg(Color::DarkGray),
    )]));
    lines.push(Line::from(vec![Span::styled(
        "  Equilibrium bid: b(v) = (n-1)/n * v * (v/H)^(n-1)",
        Style::default().fg(Color::DarkGray),
    )]));
}

fn render_theory_double(lines: &mut Vec<Line>, state: &DebriefState) {
    // Number of trades.
    let n_trades = state.outcome.allocations.len();
    let clearing = state.outcome.payments.first().map(|p| p.amount);

    if n_trades > 0 {
        if let Some(price) = clearing {
            lines.push(Line::from(vec![Span::styled(
                format!("  {} trade(s) cleared at uniform price {}.", n_trades, price),
                Style::default().fg(Color::White),
            )]));
        }

        // Buyer surpluses.
        let winner_ids: HashSet<BidderId> =
            state.outcome.allocations.iter().map(|a| a.bidder_id).collect();
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            "  Buyer surpluses:",
            Style::default().fg(Color::DarkGray),
        )]));

        // Human
        let human_surplus = if winner_ids.contains(&state.human_id) {
            state.human_value - clearing.unwrap_or(Money::zero())
        } else {
            Money::zero()
        };
        lines.push(Line::from(vec![
            Span::styled("    You      ", Style::default().fg(Color::Cyan)),
            Span::styled(
                format!("value {} — surplus {}", state.human_value, human_surplus),
                Style::default().fg(Color::DarkGray),
            ),
        ]));
        for ai in &state.ai_info {
            if ai.id.0 >= 4 { continue; } // skip sellers
            let surplus = if winner_ids.contains(&ai.id) {
                ai.value - clearing.unwrap_or(Money::zero())
            } else {
                Money::zero()
            };
            lines.push(Line::from(vec![
                Span::styled(format!("    {:<8}", ai.name), Style::default().fg(Color::White)),
                Span::styled(
                    format!("value {} — surplus {}", ai.value, surplus),
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
        }

        // Seller surpluses.
        let traded_sellers: HashSet<BidderId> =
            state.outcome.receipts.iter().map(|r| r.bidder_id).collect();
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            "  Seller surpluses:",
            Style::default().fg(Color::DarkGray),
        )]));
        for ai in &state.ai_info {
            if ai.id.0 < 4 { continue; } // skip buyers
            let surplus = if traded_sellers.contains(&ai.id) {
                clearing.unwrap_or(Money::zero()) - ai.value
            } else {
                Money::zero()
            };
            lines.push(Line::from(vec![
                Span::styled(format!("    {:<8}", ai.name), Style::default().fg(Color::White)),
                Span::styled(
                    format!("cost {} — surplus {}", ai.value, surplus),
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
        }
        lines.push(Line::from(""));
    } else {
        lines.push(Line::from(vec![Span::styled(
            "  No trades cleared — no bid exceeded any ask.",
            Style::default().fg(Color::DarkGray),
        )]));
        lines.push(Line::from(""));
    }

    lines.push(Line::from(vec![Span::styled(
        "  Budget balance: buyer payments = seller receipts (no external subsidy).",
        Style::default().fg(Color::DarkGray),
    )]));
    lines.push(Line::from(vec![Span::styled(
        "  Myerson-Satterthwaite: no budget-balanced mechanism can be",
        Style::default().fg(Color::DarkGray),
    )]));
    lines.push(Line::from(vec![Span::styled(
        "  simultaneously efficient and incentive-compatible.",
        Style::default().fg(Color::DarkGray),
    )]));
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Look up the payment for a specific bidder — safe for multi-payment auctions
/// (e.g. all-pay, where every bidder has an entry in `payments`).
fn winner_payment(outcome: &AuctionOutcome, winner_id: BidderId) -> Money {
    outcome
        .payments
        .iter()
        .find(|p| p.bidder_id == winner_id)
        .map(|p| p.amount)
        .unwrap_or(Money::zero())
}

fn name_of(id: BidderId, state: &DebriefState) -> String {
    if id == state.human_id {
        return "You".to_string();
    }
    state
        .ai_info
        .iter()
        .find(|a| a.id == id)
        .map(|a| a.name.clone())
        .unwrap_or_else(|| "?".to_string())
}

fn true_value_of(id: BidderId, state: &DebriefState) -> Money {
    if id == state.human_id {
        return state.human_value;
    }
    state
        .ai_info
        .iter()
        .find(|a| a.id == id)
        .map(|a| a.value)
        .unwrap_or(Money::zero())
}

fn format_bidder_row(
    name: &str,
    value: Money,
    last_bid: Option<Money>,
    is_ai: bool,
    state: &DebriefState,
) -> Line<'static> {
    let bid_str = match last_bid {
        Some(b) => match state.auction_type {
            AuctionType::Dutch => format!("called at {}", b),
            AuctionType::FirstPriceSealedBid | AuctionType::Vickrey | AuctionType::AllPay => {
                format!("sealed bid: {}", b)
            }
            AuctionType::Double => format!("bid/ask: {}", b),
            _ => format!("bid up to {}", b),
        },
        None => "did not bid".to_string(),
    };

    let won = state
        .outcome
        .allocations
        .iter()
        .any(|a| name_of(a.bidder_id, state) == name);

    let name_style = if !is_ai {
        Style::default().fg(Color::Cyan)
    } else if won {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };

    let marker = if won { "  WON" } else { "" };

    Line::from(vec![
        Span::styled(format!("  {:<8}", name), name_style),
        Span::styled(
            format!("  value: {}   {}", value, bid_str),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled(
            marker,
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        ),
    ])
}
