use auction_core::types::AuctionType;
use yew::prelude::*;

use crate::app::Screen;

#[derive(Properties, PartialEq)]
pub struct MenuScreenProps {
    pub on_navigate: Callback<Screen>,
}

/// Data for one row in the mechanism list.
struct Mechanism {
    auction_type: Option<AuctionType>,
    name: &'static str,
    tagline: &'static str,
    dot_type: &'static str, // matches data-type CSS attribute
    available: bool,
}

const MECHANISMS: &[Mechanism] = &[
    Mechanism {
        auction_type: Some(AuctionType::English),
        name: "English",
        tagline: "Open ascending price: last bidder standing wins",
        dot_type: "english",
        available: true,
    },
    Mechanism {
        auction_type: Some(AuctionType::Dutch),
        name: "Dutch",
        tagline: "Descending clock: first caller wins",
        dot_type: "dutch",
        available: true,
    },
    Mechanism {
        auction_type: Some(AuctionType::FirstPriceSealedBid),
        name: "First-Price Sealed-Bid",
        tagline: "Sealed bids: winner pays their own bid",
        dot_type: "fpsb",
        available: true,
    },
    Mechanism {
        auction_type: Some(AuctionType::Vickrey),
        name: "Vickrey",
        tagline: "Sealed bids: winner pays the second-highest bid",
        dot_type: "vickrey",
        available: true,
    },
    Mechanism {
        auction_type: Some(AuctionType::AllPay),
        name: "All-Pay",
        tagline: "Everyone pays their bid, highest bid wins",
        dot_type: "allpay",
        available: true,
    },
    Mechanism {
        auction_type: Some(AuctionType::Double),
        name: "Double (k-DA)",
        tagline: "Two-sided market: buyers and sellers, uniform clearing price",
        dot_type: "double",
        available: true,
    },
    Mechanism {
        auction_type: None,
        name: "Combinatorial",
        tagline: "Bundle bidding: VCG payments, Clarke pivot rule",
        dot_type: "stub",
        available: false,
    },
    Mechanism {
        auction_type: None,
        name: "Clock Auction",
        tagline: "Ascending multi-item: simultaneous demand reduction",
        dot_type: "stub",
        available: false,
    },
];

#[function_component]
pub fn MenuScreen(props: &MenuScreenProps) -> Html {
    html! {
        <div class="page">
            <div class="content">
                <header class="menu-header">
                    <h1 class="menu-title">{"Auction Theory Simulator"}</h1>
                    <hr class="menu-rule" />
                    <p class="menu-subtitle">
                        {"Six mechanisms. One item. You against five AI bidders. \
                          Each auction closes with a debrief explaining the mechanism, \
                          optimal strategy, and how your outcome compares to theory."}
                    </p>
                </header>

                <p class="section-label">{"Mechanisms"}</p>
                <ul class="mechanism-list" role="list">
                    { for MECHANISMS.iter().map(|m| mechanism_row(m, props.on_navigate.clone())) }
                </ul>

                <p class="menu-footnote">
                    {"Keyboard: ↑ ↓ to navigate, Enter to select, Esc to return."}
                </p>
            </div>
        </div>
    }
}

fn mechanism_row(m: &Mechanism, on_navigate: Callback<Screen>) -> Html {
    let dot = html! {
        <span class="mechanism-dot" data-type={m.dot_type}>{"●"}</span>
    };

    if m.available {
        let auction_type = m.auction_type.unwrap();
        let onclick = Callback::from(move |_: MouseEvent| {
            on_navigate.emit(Screen::Intro(auction_type));
        });

        html! {
            <li>
                <button class="mechanism-row" {onclick}>
                    { dot }
                    <span class="mechanism-name">{ m.name }</span>
                    <span class="mechanism-desc">{ m.tagline }</span>
                    <span class="mechanism-arrow">{"→"}</span>
                </button>
            </li>
        }
    } else {
        html! {
            <li>
                <div class="mechanism-row mechanism-row--unavailable" aria-disabled="true">
                    <span class="mechanism-dot" data-type="stub">{"○"}</span>
                    <span class="mechanism-name">{ m.name }</span>
                    <span class="mechanism-desc">{ m.tagline }</span>
                    <span class="mechanism-badge">{"Coming soon"}</span>
                </div>
            </li>
        }
    }
}
