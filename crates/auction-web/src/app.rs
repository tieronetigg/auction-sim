use std::rc::Rc;

use auction_core::event::AuctionEvent;
use auction_core::outcome::AuctionOutcome;
use auction_core::types::{AuctionType, BidderId, Money};
use yew::prelude::*;

use crate::screens::{
    auction::AuctionScreen,
    combinatorial::CombinatorialAuctionScreen,
    debrief::DebriefScreen,
    intro::IntroScreen,
    menu::MenuScreen,
};

/// Data carried to the debrief screen after an auction completes.
/// Held in an Rc so Screen stays cheap to clone.
#[derive(Clone)]
pub struct DebriefInfo {
    pub auction_type: AuctionType,
    pub item_name: String,
    pub outcome: AuctionOutcome,
    pub human_id: BidderId,
    pub human_value: Money,
    /// Names for every participant, index = BidderId.0 (0 = "You").
    pub bidder_names: Vec<String>,
    /// True values, index-parallel to bidder_names.
    pub bidder_values: Vec<Money>,
    pub event_log: Vec<(f64, AuctionEvent)>,
    pub reserve_price: Option<Money>,
    /// Package bids from combinatorial/VCG auctions — (bidder, package_desc, amount).
    pub package_bids: Vec<(BidderId, String, Money)>,
}

/// Debrief data is write-once; treat any two instances as equal to suppress
/// spurious re-renders of DebriefScreen when the parent re-renders.
impl PartialEq for DebriefInfo {
    fn eq(&self, _: &Self) -> bool { true }
}

/// Top-level screen state.
#[derive(Clone)]
pub enum Screen {
    Menu,
    Intro(AuctionType),
    Auction(AuctionType),
    CombinatorialAuction(AuctionType),
    Debrief(Rc<DebriefInfo>),
}

#[function_component]
pub fn App() -> Html {
    let screen = use_state(|| Screen::Menu);

    let navigate = {
        let screen = screen.clone();
        Callback::from(move |next: Screen| screen.set(next))
    };

    match (*screen).clone() {
        Screen::Menu => html! {
            <MenuScreen on_navigate={navigate} />
        },
        Screen::Intro(auction_type) => html! {
            <IntroScreen {auction_type} on_navigate={navigate} />
        },
        Screen::Auction(auction_type) => html! {
            <AuctionScreen {auction_type} on_navigate={navigate} />
        },
        Screen::CombinatorialAuction(auction_type) => html! {
            <CombinatorialAuctionScreen {auction_type} on_navigate={navigate} />
        },
        Screen::Debrief(info) => html! {
            <DebriefScreen {info} on_navigate={navigate} />
        },
    }
}
