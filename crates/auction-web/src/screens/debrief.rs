use std::rc::Rc;

use auction_core::types::{AuctionType, BidderId, Money};
use auction_core::event::AuctionEvent;
use yew::prelude::*;

use crate::app::{DebriefInfo, Screen};

#[derive(Properties, PartialEq)]
pub struct DebriefScreenProps {
    pub info: Rc<DebriefInfo>,
    pub on_navigate: Callback<Screen>,
}

#[function_component]
pub fn DebriefScreen(props: &DebriefScreenProps) -> Html {
    let info = &*props.info;

    let on_menu = {
        let on_navigate = props.on_navigate.clone();
        Callback::from(move |_: MouseEvent| on_navigate.emit(Screen::Menu))
    };

    let dot_type: &str = match info.auction_type {
        AuctionType::English           => "english",
        AuctionType::Dutch             => "dutch",
        AuctionType::FirstPriceSealedBid => "fpsb",
        AuctionType::Vickrey           => "vickrey",
        AuctionType::AllPay            => "allpay",
        AuctionType::Double            => "double",
        _                              => "stub",
    };
    let mechanism_label = dot_type.to_uppercase();

    // ── RESULT ───────────────────────────────────────────────────────────────
    let winner_id = info.outcome.allocations.first().map(|a| a.bidder_id);
    let winner_payment = info.outcome.payments.iter()
        .find(|p| Some(p.bidder_id) == winner_id)
        .map(|p| p.amount);

    let winner_name = winner_id.map(|id| {
        if id == info.human_id { "You".to_string() }
        else { info.bidder_names.get(id.0 as usize).cloned().unwrap_or_else(|| format!("Bidder {}", id.0)) }
    });

    let human_won = winner_id == Some(info.human_id);
    let human_payment = info.outcome.payments.iter()
        .find(|p| p.bidder_id == info.human_id)
        .map(|p| p.amount)
        .unwrap_or(Money(0.0));

    // For all-pay: everyone pays, so show actual payment
    let human_surplus = if human_won {
        info.human_value - human_payment
    } else if info.auction_type == AuctionType::AllPay {
        Money(0.0) - human_payment  // negative surplus
    } else {
        Money(0.0)
    };

    let result_summary = match &winner_name {
        Some(name) => format!("{} won", name),
        None => "No winner — reserve not met".to_string(),
    };

    let price_display = winner_payment
        .map(|p| format!("{}", p))
        .unwrap_or_else(|| "—".to_string());

    // ── ANALYSIS (education layer) ────────────────────────────────────────────
    // debrief_insights wants (BidderId, display_name, value) for each AI bidder
    let ai_info: Vec<(BidderId, &str, Money)> = info.bidder_values[1..]
        .iter()
        .enumerate()
        .map(|(i, &v)| {
            let id = BidderId(i as u32 + 1);
            let name = info.bidder_names.get(i + 1).map(|s| s.as_str()).unwrap_or("");
            (id, name, v)
        })
        .collect();
    let insights = auction_education::debrief_insights(
        info.auction_type,
        &info.outcome,
        info.human_id,
        info.human_value,
        &ai_info,
    );

    // ── THEORY key result ─────────────────────────────────────────────────────
    let theory_note = theory_for(info.auction_type);

    // ── BIDDER REVEAL ─────────────────────────────────────────────────────────
    // Collect what each bidder submitted from the event log
    let sealed_bids: Vec<(BidderId, Money)> = info.event_log.iter()
        .filter_map(|(_, e)| match e {
            AuctionEvent::BidSubmitted(bid) => Some((bid.bidder_id, bid.amount)),
            _ => None,
        })
        .collect();
    let sealed_asks: Vec<(BidderId, Money)> = info.event_log.iter()
        .filter_map(|(_, e)| match e {
            AuctionEvent::AskSubmitted(bid) => Some((bid.bidder_id, bid.amount)),
            _ => None,
        })
        .collect();
    // For open auctions (English/Dutch): highest accepted bid per bidder
    let accepted: Vec<(BidderId, Money)> = {
        let mut map: std::collections::HashMap<BidderId, Money> = std::collections::HashMap::new();
        for (_, e) in &info.event_log {
            if let AuctionEvent::BidAccepted { bid, .. } = e {
                map.insert(bid.bidder_id, bid.amount);
            }
        }
        let mut v: Vec<_> = map.into_iter().collect();
        v.sort_by_key(|(id, _)| id.0);
        v
    };

    let is_sealed = matches!(
        info.auction_type,
        AuctionType::FirstPriceSealedBid | AuctionType::Vickrey | AuctionType::AllPay | AuctionType::Double
    );
    let is_double = info.auction_type == AuctionType::Double;

    html! {
        <div class="page">
            <div class="content">
                <header class="auction-header">
                    <p class="intro-mechanism-label" data-type={dot_type.to_string()}>
                        { &mechanism_label }
                    </p>
                    <h1 class="intro-title">{ "Auction Result" }</h1>
                    <p class="auction-item-name">{ &info.item_name }</p>
                    <hr class="intro-rule" />
                </header>

                // ── RESULT ─────────────────────────────────────────────────
                <div class="debrief-section">
                    <p class="debrief-section-label">{ "Result" }</p>
                    <p class="debrief-result-winner">{ &result_summary }</p>
                    if winner_payment.is_some() {
                        <p class="debrief-result-price num">{ &price_display }</p>
                    }
                    if let Some(r) = info.reserve_price {
                        <p class="debrief-surplus">{ format!("Reserve: {}", r) }</p>
                    }
                    if human_won {
                        <p class="debrief-surplus">
                            { format!("Your surplus: {} − {} = {}",
                                info.human_value, human_payment, human_surplus) }
                        </p>
                    } else if info.auction_type == AuctionType::AllPay && human_payment.0 > 0.0 {
                        <p class="debrief-surplus">
                            { format!("You paid {} and did not win.", human_payment) }
                        </p>
                    } else {
                        <p class="debrief-surplus">{ "You did not win." }</p>
                    }
                </div>

                // ── ANALYSIS ───────────────────────────────────────────────
                if !insights.is_empty() {
                    <div class="debrief-section">
                        <p class="debrief-section-label">{ "Analysis" }</p>
                        <ul class="debrief-analysis-list">
                            { for insights.iter().map(|s| html! { <li>{ s }</li> }) }
                        </ul>
                    </div>
                }

                // ── THEORY ─────────────────────────────────────────────────
                <div class="debrief-section debrief-theory">
                    <p class="debrief-section-label">{ "Theory" }</p>
                    <p>{ theory_note }</p>
                </div>

                // ── BIDDER REVEAL ──────────────────────────────────────────
                <div class="debrief-section">
                    <p class="debrief-section-label">
                        { if is_sealed { "Sealed bids revealed" } else { "Bids placed" } }
                    </p>
                    { bidder_table(
                        info,
                        winner_id,
                        if is_double { &sealed_asks } else { &[] },
                        if is_sealed { &sealed_bids } else { &accepted },
                    ) }
                </div>

                <div class="debrief-actions">
                    <button class="btn-menu" onclick={on_menu}>{ "← Return to menu" }</button>
                </div>
            </div>
        </div>
    }
}

// ── Bidder table ──────────────────────────────────────────────────────────────

fn bidder_table(
    info: &DebriefInfo,
    winner_id: Option<BidderId>,
    asks: &[(BidderId, Money)],
    bids: &[(BidderId, Money)],
) -> Html {
    let is_double = info.auction_type == AuctionType::Double;

    let rows: Vec<Html> = info.bidder_names.iter().enumerate().map(|(i, name)| {
        let id = BidderId(i as u32);
        let is_winner = winner_id == Some(id);
        let is_human = id == info.human_id;
        let value = info.bidder_values.get(i).copied().unwrap_or(Money(0.0));
        let is_seller = is_double && id.0 >= 4;

        // Find what this bidder submitted
        let submitted = if is_seller {
            asks.iter().find(|(bid_id, _)| *bid_id == id).map(|(_, a)| *a)
        } else {
            bids.iter().find(|(bid_id, _)| *bid_id == id).map(|(_, a)| *a)
        };

        let mut row_cls = "".to_string();
        if is_winner { row_cls.push_str(" is-winner"); }
        if is_human  { row_cls.push_str(" is-human"); }

        html! {
            <tr class={row_cls.trim().to_string()}>
                <td>{ if is_human { format!("{} (you)", name) } else { name.clone() } }</td>
                <td class="num">{ format!("{}", value) }</td>
                <td class="num">
                    { submitted.map(|a| format!("{}", a)).unwrap_or_else(|| "—".to_string()) }
                </td>
                <td>{ if is_winner { "winner" } else { "" } }</td>
            </tr>
        }
    }).collect();

    let bid_col_label = if is_double { "Bid / Ask" } else { "Bid" };

    html! {
        <table class="debrief-bidder-table">
            <thead>
                <tr>
                    <th>{ "Bidder" }</th>
                    <th>{ "Value" }</th>
                    <th>{ bid_col_label }</th>
                    <th></th>
                </tr>
            </thead>
            <tbody>{ for rows.into_iter() }</tbody>
        </table>
    }
}

// ── Theory snippets ───────────────────────────────────────────────────────────

fn theory_for(t: AuctionType) -> &'static str {
    match t {
        AuctionType::English =>
            "The dominant strategy under IPV is to stay in as long as the price is below \
             your value and drop out the moment it crosses it. The English auction is \
             efficient: the highest-value bidder always wins. Revenue equivalence predicts \
             the same expected revenue as Dutch, FPSB, and Vickrey under symmetric IPV.",
        AuctionType::Dutch =>
            "A Dutch auction is strategically equivalent to a First-Price Sealed-Bid \
             auction. The optimal call price is (n−1)/n × your value — the same \
             Bayes-Nash equilibrium bid you would submit in FPSB. Waiting for a lower \
             price increases surplus if you win, but risks losing to a faster bidder.",
        AuctionType::FirstPriceSealedBid =>
            "Bidding your true value guarantees zero surplus if you win. The optimal \
             strategy is bid shading: the Bayes-Nash equilibrium bid is (n−1)/n × value. \
             With 6 bidders, the equilibrium shade is 5/6 ≈ 83%. Revenue equivalence \
             holds: expected seller revenue equals that of the English or Vickrey auction.",
        AuctionType::Vickrey =>
            "Bidding your true value is a weakly dominant strategy. No matter what others \
             bid, you cannot improve your outcome by misreporting. Overbidding risks paying \
             more than your value; underbidding risks losing an auction you should win. \
             Vickrey is the only standard single-item mechanism where truthful bidding is \
             dominant — not just an equilibrium.",
        AuctionType::AllPay =>
            "Every bidder pays their bid regardless of outcome. The equilibrium strategy \
             involves more aggressive shading than FPSB: b(v) = (n−1)/n · v · (v/H)^(n−1). \
             Despite the different payment structure, revenue equivalence holds — the \
             seller's expected revenue equals that of English, Dutch, FPSB, and Vickrey \
             under symmetric IPV.",
        AuctionType::Double =>
            "In a k-double auction, bids and asks are sorted and all crossing pairs trade \
             at a uniform clearing price. With k = 0.5, the price is the midpoint of the \
             last matching pair. Myerson-Satterthwaite shows no budget-balanced mechanism \
             can be simultaneously efficient and incentive-compatible in bilateral trade — \
             the double auction trades off these properties.",
        _ => "Theory note not available for this mechanism.",
    }
}
