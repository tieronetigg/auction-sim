use std::rc::Rc;
use std::cell::RefCell;

use auction_core::auction::combinatorial::{
    CombinatorialAuction, CombinatorialConfig, CombinatorialPaymentRule,
};
use auction_core::event::AuctionEvent;
use auction_core::outcome::AuctionOutcome;
use auction_core::package::Package;
use auction_core::types::{AuctionType, BidderId, ItemId, Money};
use gloo_timers::callback::Timeout;
use web_sys::HtmlInputElement;
use yew::prelude::*;

use crate::app::{DebriefInfo, Screen};

const DEADLINE: f64 = 30.0;

// ── Internal state ─────────────────────────────────────────────────────────────

struct CombState {
    auction: CombinatorialAuction,
    auction_type: AuctionType,
    human_id: BidderId,
    human_value: Money,
    ai_info: Vec<(String, String, Money)>,   // (name, pkg_desc, value)
    packages: Vec<(String, Package)>,         // (label, Package)
    selected_pkg_idx: usize,
    bid_submitted: bool,
    input_error: Option<String>,
    event_log: Vec<(f64, AuctionEvent)>,
    elapsed: f64,
}

fn pkg_description(pkg: &Package) -> String {
    let items: Vec<&str> = pkg.0.iter().map(|i| match i.0 {
        0 => "N", 1 => "S", _ => "?",
    }).collect();
    format!("{{{}}}", items.join(","))
}

fn new_comb_state(auction_type: AuctionType) -> CombState {
    let rule = match auction_type {
        AuctionType::Vcg => CombinatorialPaymentRule::Vcg,
        _ => CombinatorialPaymentRule::PayAsBid,
    };
    let human_id = BidderId(0);
    let all_ids = vec![BidderId(0), BidderId(1), BidderId(2), BidderId(3)];
    let mut auction = CombinatorialAuction::new(
        CombinatorialConfig { payment_rule: rule, deadline: DEADLINE },
        all_ids,
    );

    let mut event_log = Vec::new();
    let ai_bids: &[(BidderId, &[u32], f64)] = &[
        (BidderId(1), &[0], 20.0),
        (BidderId(2), &[1], 15.0),
        (BidderId(3), &[0, 1], 40.0),
    ];
    for &(id, items, value) in ai_bids {
        let pkg = Package(items.iter().copied().map(ItemId).collect());
        if let Ok(event) = auction.submit_package_bid(id, pkg, Money(value)) {
            event_log.push((0.0, event));
        }
    }

    let ai_info = vec![
        ("Alice".into(), "North Wing".into(), Money(20.0)),
        ("Bob".into(),   "South Wing".into(), Money(15.0)),
        ("Carol".into(), "Both Wings".into(), Money(40.0)),
    ];
    let packages = vec![
        ("North Wing {N}".into(), Package([ItemId(0)].iter().copied().collect())),
        ("South Wing {S}".into(), Package([ItemId(1)].iter().copied().collect())),
        ("Both Wings {N,S}".into(), Package([ItemId(0), ItemId(1)].iter().copied().collect())),
    ];

    CombState {
        auction, auction_type, human_id,
        human_value: Money(100.0),
        ai_info, packages,
        selected_pkg_idx: 2,
        bid_submitted: false,
        input_error: None,
        event_log,
        elapsed: 0.0,
    }
}

fn build_comb_debrief(s: &CombState) -> DebriefInfo {
    let outcome = s.auction.outcome()
        .map(|co| co.outcome.clone())
        .unwrap_or(AuctionOutcome {
            allocations: vec![],
            payments: vec![],
            receipts: vec![],
            revenue: Money(0.0),
            social_welfare: None,
            efficiency: None,
        });

    let package_bids: Vec<(BidderId, String, Money)> = s.event_log.iter()
        .filter_map(|(_, e)| match e {
            AuctionEvent::PackageBidSubmitted(pb) => {
                Some((pb.bidder_id, pkg_description(&pb.package), pb.value))
            }
            _ => None,
        })
        .collect();

    // bidder_names/values indexed by BidderId.0
    let bidder_names = vec![
        "You".to_string(),
        "Alice".to_string(), "Bob".to_string(), "Carol".to_string(),
    ];
    let bidder_values = vec![
        s.human_value, Money(20.0), Money(15.0), Money(40.0),
    ];

    DebriefInfo {
        auction_type: s.auction_type,
        item_name: "Real Estate Wings".to_string(),
        outcome,
        human_id: s.human_id,
        human_value: s.human_value,
        bidder_names,
        bidder_values,
        event_log: s.event_log.clone(),
        reserve_price: None,
        package_bids,
    }
}

// ── Component ──────────────────────────────────────────────────────────────────

#[derive(Properties, PartialEq)]
pub struct CombinatorialAuctionScreenProps {
    pub auction_type: AuctionType,
    pub on_navigate: Callback<Screen>,
}

#[function_component]
pub fn CombinatorialAuctionScreen(props: &CombinatorialAuctionScreenProps) -> Html {
    let auction_type = props.auction_type;
    let inner: Rc<RefCell<CombState>> = use_mut_ref(move || new_comb_state(auction_type));
    let render_tick = use_state(|| 0u32);
    let tick_val = *render_tick;
    let input_ref = use_node_ref();

    // ── Tick loop ─────────────────────────────────────────────────────────────
    use_effect_with(tick_val, {
        let inner = inner.clone();
        let render_tick = render_tick.clone();
        let on_navigate = props.on_navigate.clone();
        move |_| {
            let timeout = Timeout::new(100, move || {
                let mut g = inner.borrow_mut();
                g.elapsed += 0.1;
                g.auction.tick(0.1);
                if g.auction.outcome().is_some() {
                    let info = build_comb_debrief(&g);
                    drop(g);
                    on_navigate.emit(Screen::Debrief(Rc::new(info)));
                } else {
                    drop(g);
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

    // ── Package selection ─────────────────────────────────────────────────────
    let select_pkg = {
        let inner = inner.clone();
        let render_tick = render_tick.clone();
        Rc::new(move |idx: usize| {
            inner.borrow_mut().selected_pkg_idx = idx;
            render_tick.set(tick_val.wrapping_add(1));
        })
    };

    // ── Bid submit ────────────────────────────────────────────────────────────
    let do_submit = {
        let inner = inner.clone();
        let input_ref = input_ref.clone();
        let render_tick = render_tick.clone();
        Rc::new(move || {
            if let Some(el) = input_ref.cast::<HtmlInputElement>() {
                let val = el.value();
                let mut g = inner.borrow_mut();
                if g.bid_submitted {
                    g.input_error = Some("You have already submitted a bid.".into());
                } else {
                    match val.trim().parse::<f64>() {
                        Ok(v) if v > 0.0 => {
                            let pkg = g.packages[g.selected_pkg_idx].1.clone();
                            let human_id = g.human_id;
                            match g.auction.submit_package_bid(human_id, pkg, Money(v)) {
                                Ok(event) => {
                                    let t = g.elapsed;
                                    g.event_log.push((t, event));
                                    g.bid_submitted = true;
                                    g.input_error = None;
                                    drop(g);
                                    el.set_value("");
                                }
                                Err(_) => {
                                    g.input_error = Some("Bid rejected.".into());
                                }
                            }
                        }
                        _ => {
                            g.input_error = Some("Enter a valid positive number.".into());
                        }
                    }
                }
            }
            render_tick.set(tick_val.wrapping_add(1));
        })
    };

    let on_submit: Callback<MouseEvent> = {
        let f = do_submit.clone();
        Callback::from(move |_: MouseEvent| f())
    };
    let on_keydown: Callback<KeyboardEvent> = {
        let f = do_submit.clone();
        Callback::from(move |e: KeyboardEvent| { if e.key() == "Enter" { f(); } })
    };

    // ── Extract display data ──────────────────────────────────────────────────
    let g = inner.borrow();
    let remaining = (DEADLINE - g.elapsed).max(0.0);
    let closed = g.auction.outcome().is_some();
    let bid_submitted = g.bid_submitted;
    let input_error = g.input_error.clone();
    let selected_idx = g.selected_pkg_idx;
    let human_value = g.human_value;

    let mechanism_label = match auction_type {
        AuctionType::Vcg => "VCG",
        _ => "CB",
    };
    let _mechanism_name = match auction_type {
        AuctionType::Vcg => "VCG Mechanism",
        _ => "Combinatorial Auction",
    };
    let dot_type = match auction_type {
        AuctionType::Vcg => "vcg",
        _ => "combinatorial",
    };

    let pkg_buttons: Vec<Html> = g.packages.iter().enumerate().map(|(i, (label, _))| {
        let is_selected = i == selected_idx;
        let f = select_pkg.clone();
        let onclick = Callback::from(move |_: MouseEvent| f(i));
        let cls = if is_selected {
            "comb-pkg-btn comb-pkg-btn--selected"
        } else {
            "comb-pkg-btn"
        };
        html! {
            <button class={cls} {onclick} disabled={bid_submitted}>
                { label.clone() }
            </button>
        }
    }).collect();

    let ai_rows: Vec<Html> = g.ai_info.iter().map(|(name, pkg_desc, _value)| {
        html! {
            <tr>
                <td>{ name }</td>
                <td>{ pkg_desc }</td>
                <td class="num">{ "sealed" }</td>
            </tr>
        }
    }).collect();

    drop(g);

    if closed {
        return html! {
            <div class="page">
                <div class="content">
                    <p class="auction-closing">{ "Auction closed — computing result…" }</p>
                </div>
            </div>
        };
    }

    let deadline_pct = ((remaining / DEADLINE) * 100.0).clamp(0.0, 100.0);
    let deadline_urgent = remaining < 8.0;

    html! {
        <div class="page">
            <div class="content">
                <header class="auction-header">
                    <button class="intro-back" onclick={on_back}>{ "← Back" }</button>
                    <p class="intro-mechanism-label" data-type={dot_type.to_string()}>
                        { mechanism_label }
                    </p>
                    <p class="auction-item-name">{ "Real Estate Wings — North Wing & South Wing" }</p>
                </header>

                <div class="auction-deadline">
                    <div class="auction-deadline-row">
                        <span class="auction-price-label">{ "Deadline" }</span>
                        <span class={if deadline_urgent { "auction-deadline-value auction-deadline-value--urgent" } else { "auction-deadline-value num" }}>
                            { format!("{:.0}s", remaining) }
                        </span>
                    </div>
                    <div class="auction-deadline-track">
                        <div class="auction-deadline-fill" style={format!("width: {:.1}%", deadline_pct)}></div>
                    </div>
                </div>

                <p class="auction-value-row">
                    { format!("Your value: {} for {{N,S}} bundle", human_value) }
                </p>

                <div class="comb-section">
                    <p class="debrief-section-label">{ "Select your package" }</p>
                    <div class="comb-pkg-list">
                        { for pkg_buttons.into_iter() }
                    </div>
                </div>

                <div class="comb-section">
                    <p class="debrief-section-label">{ "AI bidders (sealed)" }</p>
                    <table class="debrief-bidder-table">
                        <thead>
                            <tr>
                                <th>{ "Bidder" }</th>
                                <th>{ "Package" }</th>
                                <th>{ "Bid" }</th>
                            </tr>
                        </thead>
                        <tbody>{ for ai_rows.into_iter() }</tbody>
                    </table>
                </div>

                if bid_submitted {
                    <p class="auction-bid-locked">
                        { "Your bid is sealed. Waiting for the deadline…" }
                    </p>
                } else {
                    <div class="auction-input-row">
                        <input
                            ref={input_ref}
                            type="number"
                            class="auction-input"
                            placeholder="Amount"
                            onkeydown={on_keydown}
                        />
                        <button class="btn-raise" onclick={on_submit}>
                            { "Submit bid" }
                        </button>
                    </div>
                }
                if let Some(err) = &input_error {
                    <p class="auction-error">{ err }</p>
                }
            </div>
        </div>
    }
}
