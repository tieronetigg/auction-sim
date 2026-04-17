use std::rc::Rc;
use std::cell::RefCell;

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
use auction_core::outcome::AuctionOutcome;
use auction_core::types::{AuctionPhase, AuctionType, BidderId, ItemId, Money};
use auction_education::{live_hint, HintLevel};
use auction_engine::engine::{BidderConfig, SimulationEngine};
use gloo_timers::callback::Timeout;
use web_sys::HtmlInputElement;
use yew::prelude::*;

use crate::app::{DebriefInfo, Screen};

// ── Shared AI setup ────────────────────────────────────────────────────────────

const AI_DATA: &[(&str, f64, u32)] = &[
    ("Alice", 420.0, 1),
    ("Bob",   380.0, 2),
    ("Carol", 310.0, 3),
    ("Dave",  450.0, 4),
    ("Eve",   290.0, 5),
];

// ── Internal auction state (non-Clone — lives in Rc<RefCell>) ──────────────────

struct AuctionState {
    engine: SimulationEngine,
    auction_type: AuctionType,
    human_id: BidderId,
    human_value: Money,
    ai_names: Vec<String>,
    ai_values: Vec<Money>,
    reserve_price: Option<Money>,
    /// Initial deadline for sealed-bid auctions (seconds), used to draw the progress bar.
    initial_deadline: Option<f64>,
    input_error: Option<String>,
    bid_submitted: bool,
}

impl AuctionState {
    fn try_english_bid(&mut self, raw: &str) -> bool {
        match raw.trim().parse::<f64>() {
            Ok(v) if v > 0.0 => match self.engine.submit_bid_for(self.human_id, Money(v)) {
                Ok(_) => { self.input_error = None; true }
                Err(BidError::BelowMinimum { minimum }) => {
                    self.input_error = Some(format!("Minimum bid: {}", minimum));
                    false
                }
                Err(_) => { self.input_error = Some("Bid rejected.".into()); false }
            },
            _ => { self.input_error = Some("Enter a valid positive number.".into()); false }
        }
    }

    fn try_dutch_call(&mut self) -> bool {
        let price = self.engine.auction.visible_state().current_price.unwrap_or(Money(0.0));
        match self.engine.submit_bid_for(self.human_id, price) {
            Ok(_) => { self.input_error = None; true }
            Err(_) => { self.input_error = Some("Could not call.".into()); false }
        }
    }

    fn try_sealed_bid(&mut self, raw: &str) -> bool {
        if self.bid_submitted { return false; }
        match raw.trim().parse::<f64>() {
            Ok(v) if v > 0.0 => match self.engine.submit_bid_for(self.human_id, Money(v)) {
                Ok(_) => {
                    self.input_error = None;
                    self.bid_submitted = true;
                    true
                }
                Err(_) => { self.input_error = Some("Bid rejected.".into()); false }
            },
            _ => { self.input_error = Some("Enter a valid positive number.".into()); false }
        }
    }
}

// ── Game constructors ──────────────────────────────────────────────────────────

fn make_watch_item() -> Item {
    Item {
        id: ItemId(0),
        name: "Vintage Chronograph Watch".to_string(),
        reserve_price: Some(Money(300.0)),
    }
}

fn standard_ai_bidders<F>(make_strategy: F) -> Vec<BidderConfig>
where
    F: Fn(&str, f64, u32) -> Box<dyn BidderStrategy>,
{
    AI_DATA
        .iter()
        .map(|&(name, value, id)| BidderConfig {
            id: BidderId(id),
            name: name.to_string(),
            strategy: make_strategy(name, value, id),
            value: Money(value),
        })
        .collect()
}

fn ai_names_values() -> (Vec<String>, Vec<Money>) {
    let names = AI_DATA.iter().map(|&(n, _, _)| n.to_string()).collect();
    let values = AI_DATA.iter().map(|&(_, v, _)| Money(v)).collect();
    (names, values)
}

fn new_english_game() -> AuctionState {
    let item = make_watch_item();
    let human_id = BidderId(0);
    let all_ids: Vec<BidderId> = std::iter::once(human_id)
        .chain(AI_DATA.iter().map(|&(_, _, id)| BidderId(id)))
        .collect();
    let auction = EnglishAuction::new(
        EnglishConfig { start_price: Money(100.0), min_increment: Money(10.0), activity_timeout: 25.0 },
        item, all_ids,
    );
    let ai = standard_ai_bidders(|name, _, id| {
        Box::new(TruthfulBidder::new(BidderId(id), name)) as Box<dyn BidderStrategy>
    });
    let mut engine = SimulationEngine::new(Box::new(auction), ai, 1.0, 3.0);
    engine.stagger_starts();
    let (ai_names, ai_values) = ai_names_values();
    AuctionState {
        engine, auction_type: AuctionType::English,
        human_id, human_value: Money(350.0),
        ai_names, ai_values,
        reserve_price: Some(Money(300.0)), initial_deadline: None,
        input_error: None, bid_submitted: false,
    }
}

fn new_dutch_game() -> AuctionState {
    let item = make_watch_item();
    let human_id = BidderId(0);
    let all_ids: Vec<BidderId> = std::iter::once(human_id)
        .chain(AI_DATA.iter().map(|&(_, _, id)| BidderId(id)))
        .collect();
    let auction = DutchAuction::new(
        DutchConfig { start_price: Money(550.0), decrement_per_second: Money(8.0), floor_price: Money(50.0) },
        item, all_ids,
    );
    let ai = standard_ai_bidders(|name, _, id| {
        Box::new(TruthfulBidder::new(BidderId(id), name)) as Box<dyn BidderStrategy>
    });
    let mut engine = SimulationEngine::new(Box::new(auction), ai, 1.0, 0.3);
    engine.stagger_starts();
    let (ai_names, ai_values) = ai_names_values();
    AuctionState {
        engine, auction_type: AuctionType::Dutch,
        human_id, human_value: Money(350.0),
        ai_names, ai_values,
        reserve_price: None, initial_deadline: None,
        input_error: None, bid_submitted: false,
    }
}

fn new_fpsb_game() -> AuctionState {
    let item = make_watch_item();
    let human_id = BidderId(0);
    let all_ids: Vec<BidderId> = std::iter::once(human_id)
        .chain(AI_DATA.iter().map(|&(_, _, id)| BidderId(id)))
        .collect();
    let auction = SealedBidAuction::new(
        SealedBidConfig { mechanism: SealedMechanism::FirstPrice, deadline: 30.0, reserve_price: Some(Money(80.0)) },
        item, all_ids,
    );
    // Alice/Carol/Dave shade at 0.80; Bob/Eve bid truthfully
    let shade_map: &[bool] = &[true, false, true, true, false]; // index-parallel to AI_DATA
    let ai: Vec<BidderConfig> = AI_DATA.iter().zip(shade_map.iter())
        .map(|(&(name, value, id), &shade)| BidderConfig {
            id: BidderId(id),
            name: name.to_string(),
            strategy: if shade {
                Box::new(BidShadingBidder::new(BidderId(id), name, 0.80)) as Box<dyn BidderStrategy>
            } else {
                Box::new(TruthfulBidder::new(BidderId(id), name)) as Box<dyn BidderStrategy>
            },
            value: Money(value),
        })
        .collect();
    let mut engine = SimulationEngine::new(Box::new(auction), ai, 1.0, 3.0);
    engine.stagger_starts();
    let (ai_names, ai_values) = ai_names_values();
    AuctionState {
        engine, auction_type: AuctionType::FirstPriceSealedBid,
        human_id, human_value: Money(350.0),
        ai_names, ai_values,
        reserve_price: None, initial_deadline: Some(30.0),
        input_error: None, bid_submitted: false,
    }
}

fn new_vickrey_game() -> AuctionState {
    let item = make_watch_item();
    let human_id = BidderId(0);
    let all_ids: Vec<BidderId> = std::iter::once(human_id)
        .chain(AI_DATA.iter().map(|&(_, _, id)| BidderId(id)))
        .collect();
    let auction = SealedBidAuction::new(
        SealedBidConfig { mechanism: SealedMechanism::SecondPrice, deadline: 30.0, reserve_price: Some(Money(80.0)) },
        item, all_ids,
    );
    let ai = standard_ai_bidders(|name, _, id| {
        Box::new(TruthfulBidder::new(BidderId(id), name)) as Box<dyn BidderStrategy>
    });
    let mut engine = SimulationEngine::new(Box::new(auction), ai, 1.0, 3.0);
    engine.stagger_starts();
    let (ai_names, ai_values) = ai_names_values();
    AuctionState {
        engine, auction_type: AuctionType::Vickrey,
        human_id, human_value: Money(350.0),
        ai_names, ai_values,
        reserve_price: None, initial_deadline: Some(30.0),
        input_error: None, bid_submitted: false,
    }
}

fn new_allpay_game() -> AuctionState {
    let item = Item {
        id: ItemId(0),
        name: "Vintage Chronograph Watch".to_string(),
        reserve_price: None,
    };
    let human_id = BidderId(0);
    let all_ids: Vec<BidderId> = std::iter::once(human_id)
        .chain(AI_DATA.iter().map(|&(_, _, id)| BidderId(id)))
        .collect();
    let auction = AllPayAuction::new(
        AllPayConfig { deadline: 30.0, reserve_price: None },
        item, all_ids,
    );
    let ai = standard_ai_bidders(|name, _, id| {
        Box::new(AllPayBidder::new(BidderId(id), name, 6, Money(500.0))) as Box<dyn BidderStrategy>
    });
    let mut engine = SimulationEngine::new(Box::new(auction), ai, 1.0, 3.0);
    engine.stagger_starts();
    let (ai_names, ai_values) = ai_names_values();
    AuctionState {
        engine, auction_type: AuctionType::AllPay,
        human_id, human_value: Money(350.0),
        ai_names, ai_values,
        reserve_price: None, initial_deadline: Some(30.0),
        input_error: None, bid_submitted: false,
    }
}

fn new_double_game() -> AuctionState {
    let item = Item { id: ItemId(0), name: "Research Report".to_string(), reserve_price: None };
    let human_id = BidderId(0);
    let buyer_ids = vec![BidderId(0), BidderId(1), BidderId(2), BidderId(3)];
    let seller_ids = vec![BidderId(4), BidderId(5), BidderId(6), BidderId(7)];

    let auction = DoubleAuction::new(
        DoubleAuctionConfig { deadline: 45.0 },
        item, buyer_ids, seller_ids,
    );

    let buyer_data: &[(&str, f64, u32)] = &[
        ("Alice", 120.0, 1), ("Bob", 110.0, 2), ("Carol", 90.0, 3),
    ];
    let seller_data: &[(&str, f64, u32)] = &[
        ("Dave", 35.0, 4), ("Eve", 60.0, 5), ("Fiona", 80.0, 6), ("Grant", 105.0, 7),
    ];

    let mut ai: Vec<BidderConfig> = buyer_data.iter()
        .map(|&(name, value, id)| BidderConfig {
            id: BidderId(id), name: name.to_string(),
            strategy: Box::new(TruthfulBidder::new(BidderId(id), name)) as Box<dyn BidderStrategy>,
            value: Money(value),
        })
        .collect();
    ai.extend(seller_data.iter().map(|&(name, value, id)| BidderConfig {
        id: BidderId(id), name: name.to_string(),
        strategy: Box::new(TruthfulSellerBidder::new(BidderId(id), name)) as Box<dyn BidderStrategy>,
        value: Money(value),
    }));

    let mut engine = SimulationEngine::new(Box::new(auction), ai, 1.0, 3.0);
    engine.stagger_starts();

    // Build name/value lists covering all bidder IDs 0..7
    let all_names = vec![
        "You".to_string(), "Alice".to_string(), "Bob".to_string(), "Carol".to_string(),
        "Dave".to_string(), "Eve".to_string(), "Fiona".to_string(), "Grant".to_string(),
    ];
    let all_values = vec![
        Money(100.0), Money(120.0), Money(110.0), Money(90.0),
        Money(35.0),  Money(60.0),  Money(80.0),  Money(105.0),
    ];
    let ai_names = all_names[1..].to_vec();
    let ai_values = all_values[1..].to_vec();

    AuctionState {
        engine, auction_type: AuctionType::Double,
        human_id, human_value: Money(100.0),
        ai_names, ai_values,
        reserve_price: None, initial_deadline: Some(45.0),
        input_error: None, bid_submitted: false,
    }
}

fn make_game(t: AuctionType) -> AuctionState {
    match t {
        AuctionType::English           => new_english_game(),
        AuctionType::Dutch             => new_dutch_game(),
        AuctionType::FirstPriceSealedBid => new_fpsb_game(),
        AuctionType::Vickrey           => new_vickrey_game(),
        AuctionType::AllPay            => new_allpay_game(),
        AuctionType::Double            => new_double_game(),
        _ => panic!("auction type not supported in web UI"),
    }
}

// ── Debrief builder ────────────────────────────────────────────────────────────

fn build_debrief_info(s: &AuctionState) -> DebriefInfo {
    let outcome = s.engine.outcome().cloned().unwrap_or(AuctionOutcome {
        allocations: vec![],
        payments: vec![],
        receipts: vec![],
        revenue: Money(0.0),
        social_welfare: None,
        efficiency: None,
    });

    let mut bidder_names = vec!["You".to_string()];
    bidder_names.extend(s.ai_names.iter().cloned());
    let mut bidder_values = vec![s.human_value];
    bidder_values.extend(s.ai_values.iter().cloned());

    DebriefInfo {
        auction_type: s.auction_type,
        item_name: s.engine.auction.item_name().to_string(),
        outcome,
        human_id: s.human_id,
        human_value: s.human_value,
        bidder_names,
        bidder_values,
        event_log: s.engine.event_log.clone(),
        reserve_price: s.reserve_price,
    }
}

// ── Component ──────────────────────────────────────────────────────────────────

#[derive(Properties, PartialEq)]
pub struct AuctionScreenProps {
    pub auction_type: AuctionType,
    pub on_navigate: Callback<Screen>,
}

#[function_component]
pub fn AuctionScreen(props: &AuctionScreenProps) -> Html {
    let auction_type = props.auction_type;
    let inner: Rc<RefCell<AuctionState>> = use_mut_ref(move || make_game(auction_type));
    let render_tick = use_state(|| 0u32);
    let tick_val = *render_tick;
    let input_ref = use_node_ref();

    // ── Tick loop ────────────────────────────────────────────────────────────
    use_effect_with(tick_val, {
        let inner = inner.clone();
        let render_tick = render_tick.clone();
        let on_navigate = props.on_navigate.clone();
        move |_| {
            let timeout = Timeout::new(100, move || {
                inner.borrow_mut().engine.tick(0.1);
                let phase = inner.borrow().engine.auction.phase();
                if phase == AuctionPhase::Complete {
                    let info = build_debrief_info(&inner.borrow());
                    on_navigate.emit(Screen::Debrief(Rc::new(info)));
                } else {
                    render_tick.set(tick_val.wrapping_add(1));
                }
            });
            move || drop(timeout)
        }
    });

    // ── Back button ──────────────────────────────────────────────────────────
    let on_back = {
        let on_navigate = props.on_navigate.clone();
        Callback::from(move |_: MouseEvent| on_navigate.emit(Screen::Menu))
    };

    // ── Typed-input submit (English raise + sealed submit) ───────────────────
    let do_submit = {
        let inner = inner.clone();
        let input_ref = input_ref.clone();
        let render_tick = render_tick.clone();
        Rc::new(move || {
            if let Some(el) = input_ref.cast::<HtmlInputElement>() {
                let val = el.value();
                let mut g = inner.borrow_mut();
                let at = g.auction_type;
                let ok = match at {
                    AuctionType::English => g.try_english_bid(&val),
                    _ => g.try_sealed_bid(&val),
                };
                drop(g);
                if ok { el.set_value(""); }
            }
            render_tick.set(tick_val.wrapping_add(1));
        })
    };
    let on_raise: Callback<MouseEvent> = {
        let f = do_submit.clone();
        Callback::from(move |_: MouseEvent| f())
    };
    let on_keydown: Callback<KeyboardEvent> = {
        let f = do_submit.clone();
        Callback::from(move |e: KeyboardEvent| { if e.key() == "Enter" { f(); } })
    };

    // ── Dutch call button ────────────────────────────────────────────────────
    let on_call = {
        let inner = inner.clone();
        let render_tick = render_tick.clone();
        Callback::from(move |_: MouseEvent| {
            inner.borrow_mut().try_dutch_call();
            render_tick.set(tick_val.wrapping_add(1));
        })
    };

    // ── Extract display data ─────────────────────────────────────────────────
    let guard = inner.borrow();
    let at = guard.auction_type;
    let item_name = guard.engine.auction.item_name().to_string();
    let vis = guard.engine.auction.visible_state();
    let phase = vis.phase;
    let human_value = guard.human_value;
    let human_id = guard.human_id;
    let bid_submitted = guard.bid_submitted;
    let input_error = guard.input_error.clone();

    let hint = live_hint(at, &vis, human_id, human_value, bid_submitted);

    let dot_type: &str = match at {
        AuctionType::English           => "english",
        AuctionType::Dutch             => "dutch",
        AuctionType::FirstPriceSealedBid => "fpsb",
        AuctionType::Vickrey           => "vickrey",
        AuctionType::AllPay            => "allpay",
        AuctionType::Double            => "double",
        _                              => "stub",
    };
    let mechanism_label = dot_type.to_uppercase();

    // ── Bid log (English / Dutch: BidAccepted events) ────────────────────────
    let bid_log: Vec<Html> = guard.engine.event_log.iter().rev()
        .filter_map(|(_, e)| match e {
            AuctionEvent::BidAccepted { bid, new_standing } => {
                let is_human = bid.bidder_id == human_id;
                let name = if is_human {
                    "You".to_string()
                } else {
                    guard.engine.name_of(bid.bidder_id).to_string()
                };
                let cls = if is_human { "is-human" } else { "" };
                Some(html! {
                    <li class={cls}>
                        <span class="auction-log-name">{ name }</span>
                        <span class="auction-log-amount num">{ format!("{}", new_standing) }</span>
                    </li>
                })
            }
            _ => None,
        })
        .collect();

    // ── Deadline bar (sealed-bid auctions) ───────────────────────────────────
    let deadline_remaining = vis.deadline_remaining.unwrap_or(0.0);
    let initial_dl = guard.initial_deadline.unwrap_or(30.0);
    let deadline_pct = ((deadline_remaining / initial_dl) * 100.0).clamp(0.0, 100.0);
    let deadline_urgent = deadline_remaining < 8.0;

    drop(guard);

    // ── Hint rendering ───────────────────────────────────────────────────────
    let (hint_text, hint_cls) = match hint {
        Some((HintLevel::Urgent,  s)) => (s, "auction-hint auction-hint--urgent"),
        Some((HintLevel::Caution, s)) => (s, "auction-hint auction-hint--caution"),
        Some((HintLevel::Info,    s)) => (s, "auction-hint auction-hint--info"),
        None => (String::new(), "auction-hint"),
    };

    // ── Closed overlay ───────────────────────────────────────────────────────
    if phase == AuctionPhase::Complete {
        return html! {
            <div class="page">
                <div class="content">
                    <p class="auction-closing">{ "Auction closed — computing result…" }</p>
                </div>
            </div>
        };
    }

    // ── Per-type render ──────────────────────────────────────────────────────
    match at {
        AuctionType::Dutch => html! {
            <div class="page">
                <div class="content">
                    { header_html(&on_back, dot_type, &mechanism_label, &item_name) }

                    <div class="auction-dutch-main">
                        <p class="auction-price-label">{ "Current price" }</p>
                        <p class="auction-clock-price num">
                            { vis.current_price.map(|p| format!("{}", p)).unwrap_or_else(|| "—".to_string()) }
                        </p>
                        <p class="auction-clock-meta">{ "dropping $8 / second" }</p>
                        <p class="auction-value-row">{ format!("Your value: {}", human_value) }</p>
                        if !hint_text.is_empty() {
                            <p class={hint_cls}>{ &hint_text }</p>
                        }
                        <button class="btn-call" onclick={on_call}>
                            { format!("Call at {}", vis.current_price.unwrap_or(Money(0.0))) }
                        </button>
                    </div>
                </div>
            </div>
        },

        AuctionType::English => {
            let standing_name = vis.standing_bidder
                .map(|id| if id == human_id { "You".to_string() } else {
                    let guard2 = inner.borrow();
                    guard2.engine.name_of(id).to_string()
                })
                .unwrap_or_else(|| "No bids yet".to_string());

            let silence_remaining = (25.0_f64 - vis.time_since_last_bid).max(0.0);
            let timer_cls = if silence_remaining < 8.0 {
                "auction-timer-value auction-timer-value--urgent"
            } else {
                "auction-timer-value"
            };

            html! {
                <div class="page">
                    <div class="content">
                        { header_html(&on_back, dot_type, &mechanism_label, &item_name) }

                        <div class="auction-english-status">
                            <div class="auction-price-block">
                                <p class="auction-price-label">{ "Standing bid" }</p>
                                <p class="auction-price num">
                                    { vis.current_price.map(|p| format!("{}", p)).unwrap_or_else(|| "None".to_string()) }
                                </p>
                                <p class="auction-standing-bidder">{ standing_name }</p>
                            </div>
                            <div class="auction-timer">
                                <p class="auction-price-label">{ "Silence" }</p>
                                <p class={timer_cls}>{ format!("{:.0}s", silence_remaining) }</p>
                            </div>
                        </div>

                        <p class="auction-value-row">
                            { format!("Your value: {} — min bid: {}", human_value, vis.min_bid) }
                        </p>
                        if !hint_text.is_empty() {
                            <p class={hint_cls}>{ &hint_text }</p>
                        }
                        { input_row_html(&input_ref, &on_raise, &on_keydown, "Raise", false) }
                        if let Some(err) = &input_error {
                            <p class="auction-error">{ err }</p>
                        }
                        { bid_log_html(bid_log) }
                    </div>
                </div>
            }
        },

        _ => {
            // FPSB / Vickrey / AllPay / Double — sealed-bid layout
            let submit_label = if at == AuctionType::Double { "Submit bid" } else { "Submit bid" };
            let context_line = match at {
                AuctionType::FirstPriceSealedBid =>
                    format!("Your value: {} — equilibrium ≈ {} (5/6 × value)", human_value, Money(human_value.0 * 5.0/6.0)),
                AuctionType::Vickrey =>
                    format!("Your value: {} — bid your true value (dominant strategy)", human_value),
                AuctionType::AllPay =>
                    format!("Your value: {} — equilibrium ≈ {} (BNE formula)", human_value, Money(human_value.0 * (5.0/6.0) * (human_value.0/500.0_f64).powi(5))),
                AuctionType::Double =>
                    format!("Your value: {} (buyer) — submit your willingness-to-pay", human_value),
                _ => format!("Your value: {}", human_value),
            };

            html! {
                <div class="page">
                    <div class="content">
                        { header_html(&on_back, dot_type, &mechanism_label, &item_name) }

                        <div class="auction-deadline">
                            <div class="auction-deadline-row">
                                <span class="auction-price-label">{ "Deadline" }</span>
                                <span class={if deadline_urgent { "auction-deadline-value auction-deadline-value--urgent" } else { "auction-deadline-value num" }}>
                                    { format!("{:.0}s", deadline_remaining) }
                                </span>
                            </div>
                            <div class="auction-deadline-track">
                                <div class="auction-deadline-fill" style={format!("width: {:.1}%", deadline_pct)}></div>
                            </div>
                        </div>

                        <p class="auction-value-row">{ &context_line }</p>
                        if !hint_text.is_empty() {
                            <p class={hint_cls}>{ &hint_text }</p>
                        }

                        if bid_submitted {
                            <p class="auction-bid-locked">{ "Bid locked in. Waiting for the deadline…" }</p>
                        } else {
                            { input_row_html(&input_ref, &on_raise, &on_keydown, submit_label, false) }
                            if let Some(err) = &input_error {
                                <p class="auction-error">{ err }</p>
                            }
                        }
                    </div>
                </div>
            }
        },
    }
}

// ── Shared HTML fragments ──────────────────────────────────────────────────────

fn header_html(on_back: &Callback<MouseEvent>, dot_type: &str, label: &str, item_name: &str) -> Html {
    html! {
        <header class="auction-header">
            <button class="intro-back" onclick={on_back.clone()}>{ "← Back" }</button>
            <p class="intro-mechanism-label" data-type={dot_type.to_string()}>{ label }</p>
            <p class="auction-item-name">{ item_name }</p>
        </header>
    }
}

fn input_row_html(
    input_ref: &NodeRef,
    on_click: &Callback<MouseEvent>,
    on_keydown: &Callback<KeyboardEvent>,
    label: &str,
    disabled: bool,
) -> Html {
    html! {
        <div class="auction-input-row">
            <input
                ref={input_ref.clone()}
                type="number"
                class="auction-input"
                placeholder="Amount"
                disabled={disabled}
                onkeydown={on_keydown.clone()}
            />
            <button class="btn-raise" onclick={on_click.clone()} disabled={disabled}>
                { label }
            </button>
        </div>
    }
}

fn bid_log_html(items: Vec<Html>) -> Html {
    html! {
        <div class="auction-log">
            <p class="auction-log-label">{ "Bids" }</p>
            if items.is_empty() {
                <p class="auction-no-bids">{ "No bids yet" }</p>
            } else {
                <ul class="auction-log-list">{ for items.into_iter() }</ul>
            }
        </div>
    }
}
