use auction_core::types::AuctionType;
use yew::prelude::*;

use crate::screens::{intro::IntroScreen, menu::MenuScreen};

/// Top-level screen state, mirroring the TUI's Screen enum.
/// Auction and Debrief are stubs until those screens are implemented.
#[derive(Clone, PartialEq)]
pub enum Screen {
    Menu,
    Intro(AuctionType),
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
    }
}
