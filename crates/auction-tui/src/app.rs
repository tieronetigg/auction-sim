use auction_core::outcome::AuctionOutcome;
use auction_core::types::{AuctionPhase, AuctionType, Money};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::screens::auction::{
    new_allpay_game, new_double_game, new_dutch_game, new_english_game, new_fpsb_game,
    new_vickrey_game, LiveAuctionState,
};
use crate::screens::debrief::DebriefState;
use crate::screens::intro::IntroState;
use crate::screens::menu::MenuState;

pub enum Screen {
    MainMenu(MenuState),
    AuctionIntro(IntroState),
    LiveAuction(Box<LiveAuctionState>),
    Debrief(Box<DebriefState>),
    Placeholder { title: String, message: String },
}

pub struct App {
    pub screen: Screen,
    pub should_quit: bool,
}

impl App {
    pub fn new() -> Self {
        App {
            screen: Screen::MainMenu(MenuState::new()),
            should_quit: false,
        }
    }

    // ── Tick ──────────────────────────────────────────────────────────────────

    /// Advance the simulation by `delta` real seconds (called every frame).
    /// Transitions to Debrief when the auction completes.
    pub fn tick(&mut self, delta: f64) {
        if let Screen::LiveAuction(state) = &mut self.screen {
            state.engine.tick(delta);
        }

        let completed = matches!(
            &self.screen,
            Screen::LiveAuction(s) if s.engine.auction.phase() == AuctionPhase::Complete
        );

        if completed {
            if let Screen::LiveAuction(live) =
                std::mem::replace(&mut self.screen, Screen::MainMenu(MenuState::new()))
            {
                self.screen = Screen::Debrief(Box::new(build_debrief(*live)));
            }
        }
    }

    // ── Key handling ─────────────────────────────────────────────────────────

    pub fn handle_key(&mut self, key: KeyEvent) {
        match self.key_transition(key) {
            KeyEffect::Quit => self.should_quit = true,
            KeyEffect::GoTo(screen) => self.screen = screen,
            KeyEffect::None => {}
        }
    }

    fn key_transition(&mut self, key: KeyEvent) -> KeyEffect {
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return KeyEffect::Quit;
        }

        match &mut self.screen {
            // ── Main menu ────────────────────────────────────────────────────
            Screen::MainMenu(state) => match key.code {
                KeyCode::Char('q') | KeyCode::Char('Q') => KeyEffect::Quit,
                KeyCode::Down | KeyCode::Char('j') => {
                    state.next();
                    KeyEffect::None
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    state.prev();
                    KeyEffect::None
                }
                KeyCode::Enter | KeyCode::Char(' ') => {
                    let item = &state.items[state.selected];
                    if item.available {
                        let auction_type = item
                            .auction_type
                            .unwrap_or(AuctionType::English);
                        KeyEffect::GoTo(Screen::AuctionIntro(IntroState::new(auction_type)))
                    } else {
                        let title = item.label.to_string();
                        let message = format!(
                            "{}\n\n{}\n\n{}\n\nPress Esc or q to return to the menu.",
                            item.label, item.description, item.phase_note,
                        );
                        KeyEffect::GoTo(Screen::Placeholder { title, message })
                    }
                }
                _ => KeyEffect::None,
            },

            // ── Auction intro ────────────────────────────────────────────────
            Screen::AuctionIntro(state) => {
                let auction_type = state.auction_type;
                match key.code {
                    KeyCode::Esc => KeyEffect::GoTo(Screen::MainMenu(MenuState::new())),
                    KeyCode::Down | KeyCode::Char('j') => {
                        state.scroll_down();
                        KeyEffect::None
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        state.scroll_up();
                        KeyEffect::None
                    }
                    KeyCode::Enter | KeyCode::Char(' ') => {
                        let game = make_game(auction_type);
                        KeyEffect::GoTo(Screen::LiveAuction(Box::new(game)))
                    }
                    _ => KeyEffect::None,
                }
            }

            // ── Live auction ─────────────────────────────────────────────────
            Screen::LiveAuction(state) => {
                if state.engine.auction.phase() == AuctionPhase::Complete {
                    return KeyEffect::None;
                }

                let auction_type = state.auction_type;
                match auction_type {
                    AuctionType::Dutch => self.handle_dutch_key(key),
                    AuctionType::FirstPriceSealedBid
                    | AuctionType::Vickrey
                    | AuctionType::AllPay
                    | AuctionType::Double => self.handle_sealed_key(key),
                    _ => self.handle_english_key(key),
                }
            }

            // ── Debrief ──────────────────────────────────────────────────────
            Screen::Debrief(state) => match key.code {
                KeyCode::Down | KeyCode::Char('j') => {
                    state.scroll_down();
                    KeyEffect::None
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    state.scroll_up();
                    KeyEffect::None
                }
                _ => KeyEffect::GoTo(Screen::MainMenu(MenuState::new())),
            },

            // ── Placeholder ──────────────────────────────────────────────────
            Screen::Placeholder { .. } => match key.code {
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Backspace => {
                    KeyEffect::GoTo(Screen::MainMenu(MenuState::new()))
                }
                _ => KeyEffect::None,
            },
        }
    }

    // ── Per-mechanism key handlers ────────────────────────────────────────────

    fn handle_english_key(&mut self, key: KeyEvent) -> KeyEffect {
        if let Screen::LiveAuction(state) = &mut self.screen {
            match key.code {
                KeyCode::Esc => return KeyEffect::GoTo(Screen::MainMenu(MenuState::new())),
                KeyCode::Char(c) if c.is_ascii_digit() || c == '.' => {
                    state.bid_input.push(c);
                    state.input_error = None;
                }
                KeyCode::Backspace => {
                    state.bid_input.pop();
                    state.input_error = None;
                }
                KeyCode::Enter => {
                    state.submit_human_bid();
                }
                _ => {}
            }
        }
        KeyEffect::None
    }

    fn handle_dutch_key(&mut self, key: KeyEvent) -> KeyEffect {
        if let Screen::LiveAuction(state) = &mut self.screen {
            match key.code {
                KeyCode::Esc => return KeyEffect::GoTo(Screen::MainMenu(MenuState::new())),
                KeyCode::Enter | KeyCode::Char(' ') => {
                    state.submit_human_call();
                }
                _ => {}
            }
        }
        KeyEffect::None
    }

    fn handle_sealed_key(&mut self, key: KeyEvent) -> KeyEffect {
        if let Screen::LiveAuction(state) = &mut self.screen {
            match key.code {
                KeyCode::Esc => return KeyEffect::GoTo(Screen::MainMenu(MenuState::new())),
                KeyCode::Char(c) if (c.is_ascii_digit() || c == '.') && !state.bid_submitted => {
                    state.bid_input.push(c);
                    state.input_error = None;
                }
                KeyCode::Backspace if !state.bid_submitted => {
                    state.bid_input.pop();
                    state.input_error = None;
                }
                KeyCode::Enter => {
                    state.submit_human_sealed_bid();
                }
                _ => {}
            }
        }
        KeyEffect::None
    }
}

// ── Screen transition enum ────────────────────────────────────────────────────

enum KeyEffect {
    None,
    Quit,
    GoTo(Screen),
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn make_game(auction_type: AuctionType) -> LiveAuctionState {
    match auction_type {
        AuctionType::Dutch => new_dutch_game(),
        AuctionType::FirstPriceSealedBid => new_fpsb_game(),
        AuctionType::Vickrey => new_vickrey_game(),
        AuctionType::AllPay => new_allpay_game(),
        AuctionType::Double => new_double_game(),
        _ => new_english_game(),
    }
}

fn build_debrief(live: LiveAuctionState) -> DebriefState {
    let outcome = live
        .engine
        .outcome()
        .cloned()
        .unwrap_or(AuctionOutcome {
            allocations: vec![],
            payments: vec![],
            receipts: vec![],
            revenue: Money::zero(),
            social_welfare: None,
            efficiency: None,
        });

    DebriefState::build(
        outcome,
        live.auction_type,
        live.human_id,
        live.human_value,
        live.ai_info,
        live.reserve_price,
        &live.engine.event_log,
    )
}
