use auction_ai::all_pay::AllPayBidder;
use auction_ai::seller::TruthfulSellerBidder;
use auction_ai::shading::BidShadingBidder;
use auction_ai::truthful::TruthfulBidder;
use auction_core::auction::all_pay::{AllPayAuction, AllPayConfig};
use auction_core::auction::double::{DoubleAuction, DoubleAuctionConfig};
use auction_core::auction::dutch::{DutchAuction, DutchConfig};
use auction_core::auction::english::{EnglishAuction, EnglishConfig};
use auction_core::auction::sealed_bid::{SealedBidAuction, SealedBidConfig, SealedMechanism};
use auction_core::bid::BidError;
use auction_core::bidder::BidderStrategy;
use auction_core::event::AuctionEvent;
use auction_core::item::Item;
use auction_core::types::{AuctionPhase, AuctionType, BidderId, ItemId, Money};
use auction_education::{live_hint, price_series, HintLevel};
use auction_engine::engine::{BidderConfig, SimulationEngine};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Sparkline},
    Frame,
};

// ─────────────────────────────────────────────
// State
// ─────────────────────────────────────────────

/// Information about one AI bidder, kept for display and debrief.
pub struct AiInfo {
    pub id: BidderId,
    pub name: String,
    pub value: Money,
}

pub struct LiveAuctionState {
    pub engine: SimulationEngine,
    pub auction_type: AuctionType,
    pub human_id: BidderId,
    pub human_value: Money,
    pub ai_info: Vec<AiInfo>,
    pub bid_input: String,
    pub input_error: Option<String>,
    /// Set to true after the human submits a sealed bid.
    pub bid_submitted: bool,
}

impl LiveAuctionState {
    // ── English: type amount + Enter ────────────────────────────────────────

    pub fn submit_human_bid(&mut self) -> bool {
        let trimmed = self.bid_input.trim().to_string();
        if trimmed.is_empty() {
            self.input_error = Some("Enter an amount first.".to_string());
            return false;
        }
        match trimmed.parse::<f64>() {
            Ok(amount) if amount > 0.0 => {
                match self.engine.submit_bid_for(self.human_id, Money(amount)) {
                    Ok(_) => {
                        self.bid_input.clear();
                        self.input_error = None;
                        true
                    }
                    Err(BidError::BelowMinimum { minimum }) => {
                        self.input_error =
                            Some(format!("Bid must be at least {}.", minimum));
                        false
                    }
                    Err(BidError::AuctionNotActive) => {
                        self.input_error = Some("Auction is not active.".to_string());
                        false
                    }
                    Err(_) => {
                        self.input_error = Some("Bid rejected.".to_string());
                        false
                    }
                }
            }
            _ => {
                self.input_error = Some("Enter a valid positive number.".to_string());
                false
            }
        }
    }

    // ── Dutch: press Enter to call at current price ─────────────────────────

    pub fn submit_human_call(&mut self) -> bool {
        let vis = self.engine.auction.visible_state();
        let price = match vis.current_price {
            Some(p) => p,
            None => {
                self.input_error = Some("No current price available.".to_string());
                return false;
            }
        };
        match self.engine.submit_bid_for(self.human_id, price) {
            Ok(_) => {
                self.input_error = None;
                true
            }
            Err(BidError::AuctionNotActive) => {
                self.input_error = Some("Auction is not active.".to_string());
                false
            }
            Err(_) => {
                self.input_error = Some("Bid rejected.".to_string());
                false
            }
        }
    }

    // ── Sealed-bid: type amount + Enter (once only) ─────────────────────────

    pub fn submit_human_sealed_bid(&mut self) -> bool {
        if self.bid_submitted {
            self.input_error = Some("You have already submitted a bid.".to_string());
            return false;
        }
        let trimmed = self.bid_input.trim().to_string();
        if trimmed.is_empty() {
            self.input_error = Some("Enter an amount first.".to_string());
            return false;
        }
        match trimmed.parse::<f64>() {
            Ok(amount) if amount > 0.0 => {
                match self.engine.submit_bid_for(self.human_id, Money(amount)) {
                    Ok(_) => {
                        self.bid_input.clear();
                        self.input_error = None;
                        self.bid_submitted = true;
                        true
                    }
                    Err(BidError::AuctionNotActive) => {
                        self.input_error = Some("Auction is not active.".to_string());
                        false
                    }
                    Err(_) => {
                        self.input_error = Some("Bid rejected.".to_string());
                        false
                    }
                }
            }
            _ => {
                self.input_error = Some("Enter a valid positive number.".to_string());
                false
            }
        }
    }
}

// ─────────────────────────────────────────────
// Game constructors
// ─────────────────────────────────────────────

/// Shared item and bidder setup used by all four game types.
fn make_item() -> Item {
    Item {
        id: ItemId(0),
        name: "Vintage Chronograph Watch".to_string(),
        reserve_price: Some(Money(80.0)),
    }
}

const AI_DATA: &[(&str, f64, u32)] = &[
    ("Alice", 420.0, 1),
    ("Bob",   380.0, 2),
    ("Carol", 310.0, 3),
    ("Dave",  450.0, 4),
    ("Eve",   290.0, 5),
];

fn ai_info_vec() -> Vec<AiInfo> {
    AI_DATA
        .iter()
        .map(|&(name, value, id)| AiInfo {
            id: BidderId(id),
            name: name.to_string(),
            value: Money(value),
        })
        .collect()
}

/// English auction: human vs. five truthful AI bidders.
pub fn new_english_game() -> LiveAuctionState {
    let human_id = BidderId(0);
    let human_value = Money(350.0);
    let item = make_item();

    let all_ids: Vec<BidderId> = std::iter::once(human_id)
        .chain(AI_DATA.iter().map(|&(_, _, id)| BidderId(id)))
        .collect();

    let auction = EnglishAuction::new(
        EnglishConfig {
            start_price: Money(100.0),
            min_increment: Money(10.0),
            activity_timeout: 15.0,
        },
        item,
        all_ids,
    );

    let ai_bidders: Vec<BidderConfig> = AI_DATA
        .iter()
        .map(|&(name, value, id)| BidderConfig {
            id: BidderId(id),
            name: name.to_string(),
            strategy: Box::new(TruthfulBidder::new(BidderId(id), name))
                as Box<dyn BidderStrategy>,
            value: Money(value),
        })
        .collect();

    let mut engine = SimulationEngine::new(Box::new(auction), ai_bidders, 1.0, 3.0);
    engine.stagger_starts();

    LiveAuctionState {
        engine,
        auction_type: AuctionType::English,
        human_id,
        human_value,
        ai_info: ai_info_vec(),
        bid_input: String::new(),
        input_error: None,
        bid_submitted: false,
    }
}

/// Dutch auction: human vs. five truthful AI bidders (they call at true value).
pub fn new_dutch_game() -> LiveAuctionState {
    let human_id = BidderId(0);
    let human_value = Money(350.0);
    let item = make_item();

    let all_ids: Vec<BidderId> = std::iter::once(human_id)
        .chain(AI_DATA.iter().map(|&(_, _, id)| BidderId(id)))
        .collect();

    let auction = DutchAuction::new(
        DutchConfig {
            start_price: Money(550.0),
            decrement_per_second: Money(8.0),
            floor_price: Money(50.0),
        },
        item,
        all_ids,
    );

    let ai_bidders: Vec<BidderConfig> = AI_DATA
        .iter()
        .map(|&(name, value, id)| BidderConfig {
            id: BidderId(id),
            name: name.to_string(),
            strategy: Box::new(TruthfulBidder::new(BidderId(id), name))
                as Box<dyn BidderStrategy>,
            value: Money(value),
        })
        .collect();

    // Short think_time so AI bidders react quickly when clock hits their value.
    let mut engine = SimulationEngine::new(Box::new(auction), ai_bidders, 1.0, 0.3);
    engine.stagger_starts();

    LiveAuctionState {
        engine,
        auction_type: AuctionType::Dutch,
        human_id,
        human_value,
        ai_info: ai_info_vec(),
        bid_input: String::new(),
        input_error: None,
        bid_submitted: false,
    }
}

/// FPSB auction: mix of shading and truthful AI bidders to show bid shading.
/// AI shade factors are ~0.75 (below the 5/6 ≈ 0.833 equilibrium) so an
/// informed human can compete.
pub fn new_fpsb_game() -> LiveAuctionState {
    let human_id = BidderId(0);
    let human_value = Money(350.0);
    let item = make_item();

    let all_ids: Vec<BidderId> = std::iter::once(human_id)
        .chain(AI_DATA.iter().map(|&(_, _, id)| BidderId(id)))
        .collect();

    let auction = SealedBidAuction::new(
        SealedBidConfig {
            mechanism: SealedMechanism::FirstPrice,
            deadline: 30.0,
            reserve_price: Some(Money(80.0)),
        },
        item,
        all_ids,
    );

    // Three shading bidders, two truthful, to illustrate mixed strategies.
    let shade_factors: &[(&str, f64, u32, bool)] = &[
        ("Alice", 420.0, 1, true),   // shade 0.75 → $315
        ("Bob",   380.0, 2, false),  // truthful → $380
        ("Carol", 310.0, 3, true),   // shade 0.75 → $232.5
        ("Dave",  450.0, 4, true),   // shade 0.75 → $337.5
        ("Eve",   290.0, 5, false),  // truthful → $290
    ];

    let ai_bidders: Vec<BidderConfig> = shade_factors
        .iter()
        .map(|&(name, value, id, shading)| {
            let strategy: Box<dyn BidderStrategy> = if shading {
                Box::new(BidShadingBidder::new(BidderId(id), name, 0.75))
            } else {
                Box::new(TruthfulBidder::new(BidderId(id), name))
            };
            BidderConfig {
                id: BidderId(id),
                name: name.to_string(),
                strategy,
                value: Money(value),
            }
        })
        .collect();

    // AI submits early; stagger_starts so they don't all submit at t=0.
    let mut engine = SimulationEngine::new(Box::new(auction), ai_bidders, 1.0, 3.0);
    engine.stagger_starts();

    LiveAuctionState {
        engine,
        auction_type: AuctionType::FirstPriceSealedBid,
        human_id,
        human_value,
        ai_info: ai_info_vec(),
        bid_input: String::new(),
        input_error: None,
        bid_submitted: false,
    }
}

/// Vickrey auction: all AI bidders bid truthfully (dominant strategy).
pub fn new_vickrey_game() -> LiveAuctionState {
    let human_id = BidderId(0);
    let human_value = Money(350.0);
    let item = make_item();

    let all_ids: Vec<BidderId> = std::iter::once(human_id)
        .chain(AI_DATA.iter().map(|&(_, _, id)| BidderId(id)))
        .collect();

    let auction = SealedBidAuction::new(
        SealedBidConfig {
            mechanism: SealedMechanism::SecondPrice,
            deadline: 30.0,
            reserve_price: Some(Money(80.0)),
        },
        item,
        all_ids,
    );

    let ai_bidders: Vec<BidderConfig> = AI_DATA
        .iter()
        .map(|&(name, value, id)| BidderConfig {
            id: BidderId(id),
            name: name.to_string(),
            strategy: Box::new(TruthfulBidder::new(BidderId(id), name))
                as Box<dyn BidderStrategy>,
            value: Money(value),
        })
        .collect();

    let mut engine = SimulationEngine::new(Box::new(auction), ai_bidders, 1.0, 3.0);
    engine.stagger_starts();

    LiveAuctionState {
        engine,
        auction_type: AuctionType::Vickrey,
        human_id,
        human_value,
        ai_info: ai_info_vec(),
        bid_input: String::new(),
        input_error: None,
        bid_submitted: false,
    }
}

/// All-pay auction: all AI bidders use the BNE equilibrium formula.
pub fn new_allpay_game() -> LiveAuctionState {
    let human_id = BidderId(0);
    let human_value = Money(350.0);
    let item = make_item();

    let all_ids: Vec<BidderId> = std::iter::once(human_id)
        .chain(AI_DATA.iter().map(|&(_, _, id)| BidderId(id)))
        .collect();

    // 6 bidders, values drawn from [0, 500] for the BNE formula.
    let auction = AllPayAuction::new(
        AllPayConfig {
            deadline: 30.0,
            reserve_price: None, // all-pay: no reserve (everyone loses their bid anyway)
        },
        item,
        all_ids,
    );

    let ai_bidders: Vec<BidderConfig> = AI_DATA
        .iter()
        .map(|&(name, value, id)| BidderConfig {
            id: BidderId(id),
            name: name.to_string(),
            strategy: Box::new(AllPayBidder::new(BidderId(id), name, 6, Money(500.0)))
                as Box<dyn BidderStrategy>,
            value: Money(value),
        })
        .collect();

    let mut engine = SimulationEngine::new(Box::new(auction), ai_bidders, 1.0, 3.0);
    engine.stagger_starts();

    LiveAuctionState {
        engine,
        auction_type: AuctionType::AllPay,
        human_id,
        human_value,
        ai_info: ai_info_vec(),
        bid_input: String::new(),
        input_error: None,
        bid_submitted: false,
    }
}

/// Double (k-DA) auction: 3 AI buyers + 4 AI sellers; human is a buyer.
pub fn new_double_game() -> LiveAuctionState {
    let human_id = BidderId(0);
    let human_value = Money(100.0);

    let item = auction_core::item::Item {
        id: ItemId(0),
        name: "Research Report".to_string(),
        reserve_price: None,
    };

    // Buyer IDs: human (0), Alice (1), Bob (2), Carol (3)
    // Seller IDs: Dave (4), Eve (5), Fiona (6), Grant (7)
    let buyer_ids = vec![BidderId(0), BidderId(1), BidderId(2), BidderId(3)];
    let seller_ids = vec![BidderId(4), BidderId(5), BidderId(6), BidderId(7)];

    let all_ids: Vec<BidderId> = buyer_ids.iter().chain(seller_ids.iter()).copied().collect();

    let auction = DoubleAuction::new(
        DoubleAuctionConfig { deadline: 45.0 },
        item,
        buyer_ids,
        seller_ids,
    );

    // AI buyers bid truthfully (analogous to Vickrey).
    let buyer_data: &[(&str, f64, u32)] = &[
        ("Alice", 120.0, 1),
        ("Bob",   110.0, 2),
        ("Carol",  90.0, 3),
    ];
    // AI sellers submit cost as ask.
    let seller_data: &[(&str, f64, u32)] = &[
        ("Dave",   35.0, 4),
        ("Eve",    60.0, 5),
        ("Fiona",  80.0, 6),
        ("Grant", 105.0, 7),
    ];

    let mut ai_bidders: Vec<BidderConfig> = Vec::new();
    for &(name, value, id) in buyer_data {
        ai_bidders.push(BidderConfig {
            id: BidderId(id),
            name: name.to_string(),
            strategy: Box::new(TruthfulBidder::new(BidderId(id), name)) as Box<dyn BidderStrategy>,
            value: Money(value),
        });
    }
    for &(name, cost, id) in seller_data {
        ai_bidders.push(BidderConfig {
            id: BidderId(id),
            name: name.to_string(),
            strategy: Box::new(TruthfulSellerBidder::new(BidderId(id), name))
                as Box<dyn BidderStrategy>,
            value: Money(cost),
        });
    }

    let mut engine = SimulationEngine::new(Box::new(auction), ai_bidders, 1.0, 3.0);
    engine.stagger_starts();

    // Double auction has a different ai_info set — include both buyers and sellers.
    let ai_info = {
        let mut v: Vec<AiInfo> = Vec::new();
        for &(name, value, id) in buyer_data {
            v.push(AiInfo { id: BidderId(id), name: name.to_string(), value: Money(value) });
        }
        for &(name, cost, id) in seller_data {
            v.push(AiInfo { id: BidderId(id), name: name.to_string(), value: Money(cost) });
        }
        v
    };

    // Track which IDs are sellers for the TUI.
    let _ = all_ids; // used above, suppress warning

    LiveAuctionState {
        engine,
        auction_type: AuctionType::Double,
        human_id,
        human_value,
        ai_info,
        bid_input: String::new(),
        input_error: None,
        bid_submitted: false,
    }
}

// ─────────────────────────────────────────────
// Rendering dispatch
// ─────────────────────────────────────────────

pub fn render(frame: &mut Frame, state: &LiveAuctionState) {
    match state.auction_type {
        AuctionType::Dutch => render_dutch(frame, state),
        AuctionType::FirstPriceSealedBid | AuctionType::Vickrey | AuctionType::AllPay => {
            render_sealed(frame, state)
        }
        AuctionType::Double => render_double(frame, state),
        _ => render_english(frame, state),
    }
}

// ─────────────────────────────────────────────
// English auction renderer (original layout)
// ─────────────────────────────────────────────

fn render_english(frame: &mut Frame, state: &LiveAuctionState) {
    let area = frame.size();
    let phase = state.engine.auction.phase();

    let title = if phase == AuctionPhase::Complete {
        " English Auction — CLOSED ".to_string()
    } else {
        format!(" English Auction — {} ", state.engine.auction.item_name())
    };
    let outer = Block::default()
        .title(title)
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    let vert = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(4),
            Constraint::Length(3),
            Constraint::Length(2), // hints
            Constraint::Length(3),
        ])
        .split(inner);

    let horiz = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(62), Constraint::Percentage(38)])
        .split(vert[0]);

    let left_cols = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(3)])
        .split(horiz[0]);

    render_bid_history(frame, state, left_cols[0]);
    render_sparkline(frame, state, left_cols[1]);
    render_participants_english(frame, state, horiz[1]);
    render_status_english(frame, state, vert[1]);
    render_hint(frame, state, vert[2]);
    render_input_english(frame, state, vert[3]);
}

fn render_bid_history(frame: &mut Frame, state: &LiveAuctionState, area: Rect) {
    let block = Block::default()
        .title(" Bid History ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let standing = state.engine.auction.visible_state().standing_bidder;

    let rows: Vec<ListItem> = state
        .engine
        .event_log
        .iter()
        .filter_map(|(t, e)| match e {
            AuctionEvent::BidAccepted { bid, new_standing } => {
                let is_high = standing == Some(bid.bidder_id);
                let is_human = bid.bidder_id == state.human_id;

                let name = if is_human {
                    "You".to_string()
                } else {
                    state
                        .ai_info
                        .iter()
                        .find(|a| a.id == bid.bidder_id)
                        .map(|a| a.name.clone())
                        .unwrap_or_else(|| "?".to_string())
                };

                let name_style = if is_human {
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                let marker = if is_high { " ◀" } else { "  " };
                let marker_style = if is_high {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                let line = Line::from(vec![
                    Span::styled(
                        format!("  t={:5.1}s  ", t),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::styled(format!("{:<7}", name), name_style),
                    Span::styled(
                        format!("  {}", new_standing),
                        Style::default().fg(Color::White),
                    ),
                    Span::styled(marker, marker_style),
                ]);
                Some(ListItem::new(line))
            }
            _ => None,
        })
        .collect();

    let n = rows.len();
    let list = List::new(rows);
    let mut ls = ListState::default();
    if n > 0 {
        ls.select(Some(n - 1));
    }
    frame.render_stateful_widget(list, inner, &mut ls);
}

fn render_participants_english(frame: &mut Frame, state: &LiveAuctionState, area: Rect) {
    let block = Block::default()
        .title(" Participants ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let vis = state.engine.auction.visible_state();
    let standing = vis.standing_bidder;
    let min_bid = vis.min_bid;

    let mut lines: Vec<Line> = Vec::new();

    let human_is_high = standing == Some(state.human_id);
    let human_marker = if human_is_high { " ◀HIGH" } else { "" };
    lines.push(Line::from(vec![
        Span::styled(
            "  YOU  ",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("  value: {}", state.human_value),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled(
            human_marker,
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        ),
    ]));
    lines.push(Line::from(Span::styled(
        "  ──────────────────────",
        Style::default().fg(Color::DarkGray),
    )));

    for ai in &state.ai_info {
        let is_high = standing == Some(ai.id);
        let dropped = min_bid > ai.value;

        let (name_style, status) = if is_high {
            (
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                Span::styled(
                    " ◀HIGH",
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                ),
            )
        } else if dropped {
            (
                Style::default().fg(Color::DarkGray),
                Span::styled("  out", Style::default().fg(Color::Red)),
            )
        } else {
            (
                Style::default().fg(Color::White),
                Span::styled("  in", Style::default().fg(Color::Green)),
            )
        };

        lines.push(Line::from(vec![
            Span::styled(format!("  {:<7}", ai.name), name_style),
            Span::styled(
                format!("  {}", ai.value),
                Style::default().fg(Color::DarkGray),
            ),
            status,
        ]));
    }

    let para = Paragraph::new(lines);
    frame.render_widget(para, inner);
}

fn render_status_english(frame: &mut Frame, state: &LiveAuctionState, area: Rect) {
    let block = Block::default()
        .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let vis = state.engine.auction.visible_state();

    let price_line = Line::from(vec![
        Span::styled("  Current price: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{}", vis.current_price.unwrap_or(Money(0.0))),
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
        ),
        Span::styled("    Min bid: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{}", vis.min_bid),
            Style::default().fg(Color::Green),
        ),
        Span::styled("    Silence: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{:.1}s / 15s", vis.time_since_last_bid),
            Style::default().fg(if vis.time_since_last_bid > 10.0 {
                Color::Red
            } else {
                Color::White
            }),
        ),
    ]);

    let bids_line = Line::from(vec![
        Span::styled("  Bids placed: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{}", vis.bid_count),
            Style::default().fg(Color::White),
        ),
        Span::styled("    Sim time: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{:.1}s", state.engine.current_time),
            Style::default().fg(Color::DarkGray),
        ),
    ]);

    let para = Paragraph::new(vec![price_line, bids_line]);
    frame.render_widget(para, inner);
}

fn render_input_english(frame: &mut Frame, state: &LiveAuctionState, area: Rect) {
    let phase = state.engine.auction.phase();
    let border_color = if phase == AuctionPhase::Complete {
        Color::DarkGray
    } else {
        Color::Blue
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if phase == AuctionPhase::Complete {
        let para = Paragraph::new("  Auction over — press any key to see results")
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(para, inner);
        return;
    }

    let prompt = if let Some(err) = &state.input_error {
        Line::from(vec![
            Span::styled("  ! ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::styled(err.as_str(), Style::default().fg(Color::Red)),
        ])
    } else {
        Line::from(Span::styled(
            "  Your bid (press Enter to submit, Esc to quit):",
            Style::default().fg(Color::DarkGray),
        ))
    };

    let input_text = format!("  $ {}|", state.bid_input);
    let input_line = Line::from(Span::styled(
        input_text,
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
    ));

    let para = Paragraph::new(vec![prompt, input_line]);
    frame.render_widget(para, inner);
}

// ─────────────────────────────────────────────
// Dutch auction renderer
// ─────────────────────────────────────────────

fn render_dutch(frame: &mut Frame, state: &LiveAuctionState) {
    let area = frame.size();
    let phase = state.engine.auction.phase();

    let title = if phase == AuctionPhase::Complete {
        " Dutch Auction — CLOSED ".to_string()
    } else {
        format!(" Dutch Auction — {} ", state.engine.auction.item_name())
    };
    let outer = Block::default()
        .title(title)
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red));
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    let vert = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(4),
            Constraint::Length(3),
            Constraint::Length(2), // hints
            Constraint::Length(3),
        ])
        .split(inner);

    let horiz = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(62), Constraint::Percentage(38)])
        .split(vert[0]);

    let left_cols = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(3)])
        .split(horiz[0]);

    render_dutch_clock(frame, state, left_cols[0]);
    render_sparkline(frame, state, left_cols[1]);
    render_participants_dutch(frame, state, horiz[1]);
    render_status_dutch(frame, state, vert[1]);
    render_hint(frame, state, vert[2]);
    render_input_dutch(frame, state, vert[3]);
}

fn render_dutch_clock(frame: &mut Frame, state: &LiveAuctionState, area: Rect) {
    let block = Block::default()
        .title(" Price Clock ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let vis = state.engine.auction.visible_state();
    let price = vis.current_price.unwrap_or(Money(0.0));
    let phase = state.engine.auction.phase();

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(""));
    lines.push(Line::from(""));

    if phase == AuctionPhase::Complete {
        // Show the PriceChanged events as a history of price drops.
        let price_steps: Vec<Money> = state
            .engine
            .event_log
            .iter()
            .filter_map(|(_, e)| match e {
                AuctionEvent::PriceChanged { new, .. } => Some(*new),
                _ => None,
            })
            .collect();

        lines.push(Line::from(vec![Span::styled(
            "  Auction closed.",
            Style::default().fg(Color::DarkGray),
        )]));
        if let Some(&final_price) = price_steps.last() {
            lines.push(Line::from(vec![
                Span::styled("  Final price: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!("{}", final_price),
                    Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                ),
            ]));
        }
    } else {
        lines.push(Line::from(vec![
            Span::styled("         ", Style::default()),
            Span::styled(
                format!("{:>10}", format!("{}", price)),
                Style::default()
                    .fg(Color::Red)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "   ▼ dropping",
                Style::default().fg(Color::DarkGray),
            ),
        ]));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("  Your value: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}", state.human_value),
                Style::default().fg(Color::Cyan),
            ),
        ]));
        if price > state.human_value {
            lines.push(Line::from(vec![Span::styled(
                "  Price still above your value — wait or call early to win.",
                Style::default().fg(Color::DarkGray),
            )]));
        } else {
            lines.push(Line::from(vec![Span::styled(
                "  Price is at or below your value — press Enter to CALL!",
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            )]));
        }
    }

    let para = Paragraph::new(lines);
    frame.render_widget(para, inner);
}

fn render_participants_dutch(frame: &mut Frame, state: &LiveAuctionState, area: Rect) {
    let block = Block::default()
        .title(" Participants ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let vis = state.engine.auction.visible_state();
    let phase = vis.phase;

    // After close: reveal who called (from BidAccepted event).
    let caller: Option<BidderId> = state
        .engine
        .event_log
        .iter()
        .find_map(|(_, e)| match e {
            AuctionEvent::BidAccepted { bid, .. } => Some(bid.bidder_id),
            _ => None,
        });

    let mut lines: Vec<Line> = Vec::new();

    let human_called = caller == Some(state.human_id);
    let human_status = if phase == AuctionPhase::Complete {
        if human_called {
            Span::styled(" CALLED", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        } else {
            Span::styled("  —", Style::default().fg(Color::DarkGray))
        }
    } else {
        Span::styled("  watching", Style::default().fg(Color::DarkGray))
    };

    lines.push(Line::from(vec![
        Span::styled(
            "  YOU  ",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("  {}", state.human_value),
            Style::default().fg(Color::DarkGray),
        ),
        human_status,
    ]));
    lines.push(Line::from(Span::styled(
        "  ──────────────────────",
        Style::default().fg(Color::DarkGray),
    )));

    for ai in &state.ai_info {
        let ai_called = caller == Some(ai.id);
        let (name_style, status) = if ai_called {
            (
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                Span::styled(" CALLED", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            )
        } else if phase == AuctionPhase::Complete {
            (
                Style::default().fg(Color::DarkGray),
                Span::styled("  —", Style::default().fg(Color::DarkGray)),
            )
        } else {
            (
                Style::default().fg(Color::White),
                Span::styled("  watching", Style::default().fg(Color::DarkGray)),
            )
        };

        lines.push(Line::from(vec![
            Span::styled(format!("  {:<7}", ai.name), name_style),
            Span::styled(
                format!("  {}", ai.value),
                Style::default().fg(Color::DarkGray),
            ),
            status,
        ]));
    }

    let para = Paragraph::new(lines);
    frame.render_widget(para, inner);
}

fn render_status_dutch(frame: &mut Frame, state: &LiveAuctionState, area: Rect) {
    let block = Block::default()
        .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let vis = state.engine.auction.visible_state();
    let price = vis.current_price.unwrap_or(Money(0.0));

    let line1 = Line::from(vec![
        Span::styled("  Clock price: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{}", price),
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ),
        Span::styled("    Drop rate: ", Style::default().fg(Color::DarkGray)),
        Span::styled("$8.00/s", Style::default().fg(Color::White)),
        Span::styled("    Elapsed: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{:.1}s", state.engine.current_time),
            Style::default().fg(Color::DarkGray),
        ),
    ]);

    let line2 = Line::from(vec![
        Span::styled("  Your value: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{}", state.human_value),
            Style::default().fg(Color::Cyan),
        ),
        Span::styled("    Gap: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            if price > state.human_value {
                format!("+{} above your value", price - state.human_value)
            } else {
                format!("{} below your value", state.human_value - price)
            },
            Style::default().fg(if price <= state.human_value {
                Color::Yellow
            } else {
                Color::DarkGray
            }),
        ),
    ]);

    frame.render_widget(Paragraph::new(vec![line1, line2]), inner);
}

fn render_input_dutch(frame: &mut Frame, state: &LiveAuctionState, area: Rect) {
    let phase = state.engine.auction.phase();
    let border_color = if phase == AuctionPhase::Complete {
        Color::DarkGray
    } else {
        Color::Red
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if phase == AuctionPhase::Complete {
        let para = Paragraph::new("  Auction over — press any key to see results")
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(para, inner);
        return;
    }

    let prompt = if let Some(err) = &state.input_error {
        Line::from(vec![
            Span::styled("  ! ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::styled(err.as_str(), Style::default().fg(Color::Red)),
        ])
    } else {
        Line::from(Span::styled(
            "  Press Enter or Space to CALL the current price  |  Esc to quit",
            Style::default().fg(Color::Yellow),
        ))
    };

    let vis = state.engine.auction.visible_state();
    let price = vis.current_price.unwrap_or(Money(0.0));
    let call_line = Line::from(vec![
        Span::styled("  → Call at: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{}", price),
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ),
    ]);

    frame.render_widget(Paragraph::new(vec![prompt, call_line]), inner);
}

// ─────────────────────────────────────────────
// Sealed-bid renderer (FPSB + Vickrey)
// ─────────────────────────────────────────────

fn render_sealed(frame: &mut Frame, state: &LiveAuctionState) {
    let area = frame.size();
    let phase = state.engine.auction.phase();

    let (label, border_color) = match state.auction_type {
        AuctionType::Vickrey => ("Vickrey (Second-Price)", Color::Green),
        _ => ("First-Price Sealed-Bid", Color::Magenta),
    };
    let title = if phase == AuctionPhase::Complete {
        format!(" {} — REVEALED ", label)
    } else {
        format!(" {} — {} ", label, state.engine.auction.item_name())
    };
    let outer = Block::default()
        .title(title)
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    let vert = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(4),
            Constraint::Length(3),
            Constraint::Length(2), // hints
            Constraint::Length(3),
        ])
        .split(inner);

    let horiz = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(62), Constraint::Percentage(38)])
        .split(vert[0]);

    render_bid_tracker(frame, state, horiz[0]);
    render_participants_sealed(frame, state, horiz[1]);
    render_status_sealed(frame, state, vert[1]);
    render_hint(frame, state, vert[2]);
    render_input_sealed(frame, state, vert[3]);
}

fn render_bid_tracker(frame: &mut Frame, state: &LiveAuctionState, area: Rect) {
    let block = Block::default()
        .title(" Bid Tracker ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let phase = state.engine.auction.phase();
    let vis = state.engine.auction.visible_state();

    let mut lines: Vec<Line> = Vec::new();

    if phase == AuctionPhase::Complete {
        // Reveal all sealed bids from the event log.
        lines.push(Line::from(vec![Span::styled(
            "  BIDS REVEALED",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )]));
        lines.push(Line::from(""));

        let mut sealed: Vec<(BidderId, Money)> = state
            .engine
            .event_log
            .iter()
            .filter_map(|(_, e)| match e {
                AuctionEvent::BidSubmitted(bid) => Some((bid.bidder_id, bid.amount)),
                _ => None,
            })
            .collect();
        // Sort descending by amount.
        sealed.sort_by(|a, b| b.1.0.partial_cmp(&a.1.0).unwrap_or(std::cmp::Ordering::Equal));

        // Get winner.
        let winner: Option<BidderId> = state
            .engine
            .event_log
            .iter()
            .find_map(|(_, e)| match e {
                AuctionEvent::AllocationDecided(o) => {
                    o.allocations.first().map(|a| a.bidder_id)
                }
                _ => None,
            });

        for (id, amount) in &sealed {
            let is_human = *id == state.human_id;
            let is_winner = winner == Some(*id);
            let name = if is_human {
                "You".to_string()
            } else {
                state
                    .ai_info
                    .iter()
                    .find(|a| a.id == *id)
                    .map(|a| a.name.clone())
                    .unwrap_or_else(|| "?".to_string())
            };

            let name_style = if is_human {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default().fg(Color::White)
            };
            let marker = if is_winner {
                Span::styled(
                    "  WON",
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                )
            } else {
                Span::styled("", Style::default())
            };

            lines.push(Line::from(vec![
                Span::styled(format!("  {:<8}", name), name_style),
                Span::styled(
                    format!("  bid: {}", amount),
                    Style::default().fg(Color::White),
                ),
                marker,
            ]));
        }
    } else {
        lines.push(Line::from(vec![Span::styled(
            "  Bids are sealed until deadline.",
            Style::default().fg(Color::DarkGray),
        )]));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("  Submitted: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{} / {}", vis.bid_count, vis.bid_count + vis.active_bidders.len()),
                Style::default().fg(Color::White),
            ),
        ]));
        lines.push(Line::from(""));

        if vis.active_bidders.is_empty() {
            lines.push(Line::from(vec![Span::styled(
                "  All bids in. Waiting for deadline.",
                Style::default().fg(Color::Green),
            )]));
        }
    }

    frame.render_widget(Paragraph::new(lines), inner);
}

fn render_participants_sealed(frame: &mut Frame, state: &LiveAuctionState, area: Rect) {
    let block = Block::default()
        .title(" Participants ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let vis = state.engine.auction.visible_state();
    let waiting: &[BidderId] = &vis.active_bidders;

    let mut lines: Vec<Line> = Vec::new();

    let human_waiting = waiting.contains(&state.human_id);
    let human_status = if human_waiting {
        Span::styled("  waiting", Style::default().fg(Color::Yellow))
    } else {
        Span::styled("  sealed ✓", Style::default().fg(Color::Green))
    };
    lines.push(Line::from(vec![
        Span::styled(
            "  YOU  ",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ),
        human_status,
    ]));
    lines.push(Line::from(Span::styled(
        "  ──────────────────────",
        Style::default().fg(Color::DarkGray),
    )));

    for ai in &state.ai_info {
        let ai_waiting = waiting.contains(&ai.id);
        let (name_style, status) = if ai_waiting {
            (
                Style::default().fg(Color::DarkGray),
                Span::styled("  waiting", Style::default().fg(Color::Yellow)),
            )
        } else {
            (
                Style::default().fg(Color::White),
                Span::styled("  sealed ✓", Style::default().fg(Color::Green)),
            )
        };

        lines.push(Line::from(vec![
            Span::styled(format!("  {:<7}", ai.name), name_style),
            status,
        ]));
    }

    frame.render_widget(Paragraph::new(lines), inner);
}

fn render_status_sealed(frame: &mut Frame, state: &LiveAuctionState, area: Rect) {
    let block = Block::default()
        .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let vis = state.engine.auction.visible_state();
    let deadline = vis.deadline_remaining.unwrap_or(0.0);
    let phase = vis.phase;

    let timer_line = Line::from(vec![
        Span::styled("  Time remaining: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            if phase == AuctionPhase::Complete {
                "REVEALED".to_string()
            } else {
                format!("{:.1}s", deadline)
            },
            Style::default()
                .fg(if deadline < 5.0 && phase != AuctionPhase::Complete {
                    Color::Red
                } else {
                    Color::White
                })
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("    Bids in: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{}", vis.bid_count),
            Style::default().fg(Color::White),
        ),
    ]);

    let info_line = Line::from(vec![
        Span::styled("  Your value: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{}", state.human_value),
            Style::default().fg(Color::Cyan),
        ),
        Span::styled("    Mechanism: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            match state.auction_type {
                AuctionType::Vickrey => "pay 2nd-highest",
                _ => "pay your bid",
            },
            Style::default().fg(Color::White),
        ),
    ]);

    frame.render_widget(Paragraph::new(vec![timer_line, info_line]), inner);
}

// ─────────────────────────────────────────────
// Shared: hints panel and sparkline
// ─────────────────────────────────────────────

fn render_hint(frame: &mut Frame, state: &LiveAuctionState, area: Rect) {
    let vis = state.engine.auction.visible_state();
    let hint = live_hint(
        state.auction_type,
        &vis,
        state.human_id,
        state.human_value,
        state.bid_submitted,
    );

    let (text, color) = match hint {
        Some((HintLevel::Urgent, s)) => (format!("  ! {}", s), Color::Red),
        Some((HintLevel::Caution, s)) => (format!("  ~ {}", s), Color::Yellow),
        Some((HintLevel::Info, s)) => (format!("    {}", s), Color::Cyan),
        None => (String::new(), Color::DarkGray),
    };

    let para = Paragraph::new(Line::from(Span::styled(
        text,
        Style::default().fg(color),
    )));
    frame.render_widget(para, area);
}

fn render_sparkline(frame: &mut Frame, state: &LiveAuctionState, area: Rect) {
    let series = price_series(&state.engine.event_log, state.auction_type);

    let block = Block::default()
        .title(" Price history ")
        .borders(Borders::TOP)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if series.is_empty() {
        let para = Paragraph::new(Line::from(Span::styled(
            "  waiting for first bid...",
            Style::default().fg(Color::DarkGray),
        )));
        frame.render_widget(para, inner);
        return;
    }

    let sparkline = Sparkline::default()
        .data(&series)
        .style(Style::default().fg(Color::Green));
    frame.render_widget(sparkline, inner);
}

// ─────────────────────────────────────────────
// Double auction renderer (k-DA)
// ─────────────────────────────────────────────

fn render_double(frame: &mut Frame, state: &LiveAuctionState) {
    let area = frame.size();
    let phase = state.engine.auction.phase();

    let title = if phase == AuctionPhase::Complete {
        " Double Auction (k-DA) — REVEALED ".to_string()
    } else {
        format!(" Double Auction (k-DA) — {} ", state.engine.auction.item_name())
    };
    let outer = Block::default()
        .title(title)
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Blue));
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    let vert = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(4),
            Constraint::Length(3),
            Constraint::Length(2), // hints
            Constraint::Length(3),
        ])
        .split(inner);

    let horiz = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(vert[0]);

    render_double_buyers(frame, state, horiz[0]);
    render_double_sellers(frame, state, horiz[1]);
    render_status_double(frame, state, vert[1]);
    render_hint(frame, state, vert[2]);
    render_input_sealed(frame, state, vert[3]);
}

// Seller IDs used in the double game (indices 4-7 = Dave, Eve, Fiona, Grant).
fn is_double_seller(id: BidderId) -> bool {
    id.0 >= 4
}

fn render_double_buyers(frame: &mut Frame, state: &LiveAuctionState, area: Rect) {
    let block = Block::default()
        .title(" Buyers ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let phase = state.engine.auction.phase();
    let vis = state.engine.auction.visible_state();

    // After reveal: show buy bids sorted descending.
    if phase == AuctionPhase::Complete {
        let mut buy_bids: Vec<(BidderId, Money)> = state
            .engine
            .event_log
            .iter()
            .filter_map(|(_, e)| match e {
                AuctionEvent::BidSubmitted(bid) if !is_double_seller(bid.bidder_id) => {
                    Some((bid.bidder_id, bid.amount))
                }
                _ => None,
            })
            .collect();
        buy_bids.sort_by(|a, b| b.1.0.partial_cmp(&a.1.0).unwrap_or(std::cmp::Ordering::Equal));

        let winner_ids: Vec<BidderId> = state
            .engine
            .event_log
            .iter()
            .find_map(|(_, e)| match e {
                AuctionEvent::AllocationDecided(o) => {
                    Some(o.allocations.iter().map(|a| a.bidder_id).collect())
                }
                _ => None,
            })
            .unwrap_or_default();

        let clearing = state.engine.outcome().and_then(|o| o.payments.first().map(|p| p.amount));

        let mut lines: Vec<Line> = vec![Line::from(vec![Span::styled(
            "  BID REVEALS (descending)",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )])];
        lines.push(Line::from(""));

        for (id, amt) in &buy_bids {
            let is_human = *id == state.human_id;
            let won = winner_ids.contains(id);
            let name = if is_human {
                "You".to_string()
            } else {
                state.ai_info.iter().find(|a| a.id == *id).map(|a| a.name.clone()).unwrap_or_else(|| "?".to_string())
            };
            let name_style = if is_human {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default().fg(Color::White)
            };
            let marker = if won {
                Span::styled(" TRADED", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
            } else {
                Span::styled("", Style::default())
            };
            lines.push(Line::from(vec![
                Span::styled(format!("  {:<7}", name), name_style),
                Span::styled(format!("  bid: {}", amt), Style::default().fg(Color::DarkGray)),
                marker,
            ]));
        }

        if let Some(p) = clearing {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("  Clearing: ", Style::default().fg(Color::DarkGray)),
                Span::styled(format!("{}", p), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            ]));
        }

        frame.render_widget(Paragraph::new(lines), inner);
        return;
    }

    // During bidding: show submission status.
    let mut lines: Vec<Line> = Vec::new();

    let human_pending = vis.active_bidders.contains(&state.human_id);
    let human_status = if human_pending {
        Span::styled("  pending", Style::default().fg(Color::Yellow))
    } else {
        Span::styled("  bid sealed", Style::default().fg(Color::Green))
    };
    lines.push(Line::from(vec![
        Span::styled("  YOU  ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        human_status,
    ]));
    lines.push(Line::from(Span::styled("  ──────────────────────", Style::default().fg(Color::DarkGray))));

    for ai in state.ai_info.iter().filter(|a| !is_double_seller(a.id)) {
        let pending = vis.active_bidders.contains(&ai.id);
        let status = if pending {
            Span::styled("  pending", Style::default().fg(Color::Yellow))
        } else {
            Span::styled("  bid sealed", Style::default().fg(Color::Green))
        };
        lines.push(Line::from(vec![
            Span::styled(format!("  {:<7}", ai.name), Style::default().fg(Color::White)),
            status,
        ]));
    }

    frame.render_widget(Paragraph::new(lines), inner);
}

fn render_double_sellers(frame: &mut Frame, state: &LiveAuctionState, area: Rect) {
    let block = Block::default()
        .title(" Sellers ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let phase = state.engine.auction.phase();
    let vis = state.engine.auction.visible_state();

    if phase == AuctionPhase::Complete {
        let mut ask_bids: Vec<(BidderId, Money)> = state
            .engine
            .event_log
            .iter()
            .filter_map(|(_, e)| match e {
                AuctionEvent::AskSubmitted(bid) => Some((bid.bidder_id, bid.amount)),
                _ => None,
            })
            .collect();
        ask_bids.sort_by(|a, b| a.1.0.partial_cmp(&b.1.0).unwrap_or(std::cmp::Ordering::Equal));

        let traded_sellers: Vec<BidderId> = state
            .engine
            .outcome()
            .map(|o| o.receipts.iter().map(|r| r.bidder_id).collect())
            .unwrap_or_default();

        let mut lines: Vec<Line> = vec![Line::from(vec![Span::styled(
            "  ASK REVEALS (ascending)",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )])];
        lines.push(Line::from(""));

        for (id, amt) in &ask_bids {
            let name = state.ai_info.iter().find(|a| a.id == *id).map(|a| a.name.clone()).unwrap_or_else(|| "?".to_string());
            let traded = traded_sellers.contains(id);
            let marker = if traded {
                Span::styled(" TRADED", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
            } else {
                Span::styled("", Style::default())
            };
            lines.push(Line::from(vec![
                Span::styled(format!("  {:<7}", name), Style::default().fg(Color::White)),
                Span::styled(format!("  ask: {}", amt), Style::default().fg(Color::DarkGray)),
                marker,
            ]));
        }

        frame.render_widget(Paragraph::new(lines), inner);
        return;
    }

    // During bidding.
    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(Span::styled("  Asks are sealed until deadline.", Style::default().fg(Color::DarkGray))));
    lines.push(Line::from(""));

    for ai in state.ai_info.iter().filter(|a| is_double_seller(a.id)) {
        let pending = vis.active_bidders.contains(&ai.id);
        let status = if pending {
            Span::styled("  pending", Style::default().fg(Color::Yellow))
        } else {
            Span::styled("  ask sealed", Style::default().fg(Color::Green))
        };
        lines.push(Line::from(vec![
            Span::styled(format!("  {:<7}", ai.name), Style::default().fg(Color::White)),
            status,
        ]));
    }

    frame.render_widget(Paragraph::new(lines), inner);
}

fn render_status_double(frame: &mut Frame, state: &LiveAuctionState, area: Rect) {
    let block = Block::default()
        .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let vis = state.engine.auction.visible_state();
    let deadline = vis.deadline_remaining.unwrap_or(0.0);
    let phase = vis.phase;

    let n_buyers = state.ai_info.iter().filter(|a| !is_double_seller(a.id)).count() + 1; // +1 human
    let n_sellers = state.ai_info.iter().filter(|a| is_double_seller(a.id)).count();
    let n_total = n_buyers + n_sellers;
    let n_submitted = n_total - vis.active_bidders.len();

    let line1 = Line::from(vec![
        Span::styled("  Time remaining: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            if phase == AuctionPhase::Complete { "REVEALED".to_string() } else { format!("{:.1}s", deadline) },
            Style::default()
                .fg(if deadline < 5.0 && phase != AuctionPhase::Complete { Color::Red } else { Color::White })
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("    Orders in: ", Style::default().fg(Color::DarkGray)),
        Span::styled(format!("{} / {}", n_submitted, n_total), Style::default().fg(Color::White)),
    ]);
    let line2 = Line::from(vec![
        Span::styled("  Your value: ", Style::default().fg(Color::DarkGray)),
        Span::styled(format!("{}", state.human_value), Style::default().fg(Color::Cyan)),
        Span::styled("    Mechanism: ", Style::default().fg(Color::DarkGray)),
        Span::styled("k-DA uniform price (k=0.5)", Style::default().fg(Color::White)),
    ]);

    frame.render_widget(Paragraph::new(vec![line1, line2]), inner);
}

fn render_input_sealed(frame: &mut Frame, state: &LiveAuctionState, area: Rect) {
    let phase = state.engine.auction.phase();
    let border_color = match (phase, state.bid_submitted) {
        (AuctionPhase::Complete, _) => Color::DarkGray,
        (_, true) => Color::Green,
        _ => Color::Magenta,
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if phase == AuctionPhase::Complete {
        let para = Paragraph::new("  Bids revealed — press any key to see full results")
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(para, inner);
        return;
    }

    if state.bid_submitted {
        let para = Paragraph::new(
            "  Your bid is sealed. Waiting for the deadline...",
        )
        .style(Style::default().fg(Color::Green));
        frame.render_widget(para, inner);
        return;
    }

    let prompt = if let Some(err) = &state.input_error {
        Line::from(vec![
            Span::styled("  ! ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::styled(err.as_str(), Style::default().fg(Color::Red)),
        ])
    } else {
        Line::from(Span::styled(
            "  Enter sealed bid (Enter to submit, Esc to quit) — one shot only:",
            Style::default().fg(Color::DarkGray),
        ))
    };

    let input_text = format!("  $ {}|", state.bid_input);
    let input_line = Line::from(Span::styled(
        input_text,
        Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
    ));

    frame.render_widget(Paragraph::new(vec![prompt, input_line]), inner);
}
