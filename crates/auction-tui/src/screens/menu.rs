use auction_core::types::AuctionType;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

pub struct MenuItem {
    pub label: &'static str,
    pub short: &'static str,
    pub description: &'static str,
    pub available: bool,
    pub phase_note: &'static str,
    /// The auction type launched when this item is selected (None if unavailable).
    pub auction_type: Option<AuctionType>,
}

pub struct MenuState {
    pub items: Vec<MenuItem>,
    pub selected: usize,
}

impl MenuState {
    pub fn new() -> Self {
        MenuState {
            selected: 0,
            items: vec![
                MenuItem {
                    label: "English Auction",
                    short: "EN",
                    description: "Open ascending-bid — stay in while price < your value",
                    available: true,
                    phase_note: "",
                    auction_type: Some(AuctionType::English),
                },
                MenuItem {
                    label: "Dutch Auction",
                    short: "DU",
                    description: "Open descending-clock — call when price meets your value",
                    available: true,
                    phase_note: "",
                    auction_type: Some(AuctionType::Dutch),
                },
                MenuItem {
                    label: "First-Price Sealed-Bid",
                    short: "FP",
                    description: "Highest bid wins and pays their own bid",
                    available: true,
                    phase_note: "",
                    auction_type: Some(AuctionType::FirstPriceSealedBid),
                },
                MenuItem {
                    label: "Vickrey (Second-Price)",
                    short: "VK",
                    description: "Highest bid wins, pays second-highest — truth is dominant",
                    available: true,
                    phase_note: "",
                    auction_type: Some(AuctionType::Vickrey),
                },
                MenuItem {
                    label: "All-Pay Auction",
                    short: "AP",
                    description: "Everyone pays their bid; highest bidder wins",
                    available: true,
                    phase_note: "",
                    auction_type: Some(AuctionType::AllPay),
                },
                MenuItem {
                    label: "Double Auction (k-DA)",
                    short: "DA",
                    description: "Two-sided market: buyers bid, sellers ask, uniform price clears",
                    available: true,
                    phase_note: "",
                    auction_type: Some(AuctionType::Double),
                },
                MenuItem {
                    label: "Combinatorial Auction",
                    short: "CB",
                    description: "Package bids on bundles of multiple items",
                    available: false,
                    phase_note: "Coming in Phase 7",
                    auction_type: None,
                },
                MenuItem {
                    label: "VCG Mechanism",
                    short: "VG",
                    description: "Strategy-proof and efficient — you pay your externality",
                    available: false,
                    phase_note: "Coming in Phase 7",
                    auction_type: None,
                },
            ],
        }
    }

    pub fn next(&mut self) {
        self.selected = (self.selected + 1) % self.items.len();
    }

    pub fn prev(&mut self) {
        if self.selected == 0 {
            self.selected = self.items.len() - 1;
        } else {
            self.selected -= 1;
        }
    }
}

pub fn render(frame: &mut Frame, state: &MenuState) {
    let area = frame.size();

    let outer = Block::default()
        .title(" Auction Theory Simulator ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // subtitle
            Constraint::Length(1), // spacer
            Constraint::Min(1),    // menu list
            Constraint::Length(1), // footer
        ])
        .split(inner);

    let subtitle = Paragraph::new("  Select an auction type to simulate and study")
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(subtitle, chunks[0]);

    let rows: Vec<ListItem> = state
        .items
        .iter()
        .map(|item| {
            let (tag_style, label_style, status_text, status_style) = if item.available {
                (
                    Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                    Style::default().fg(Color::White),
                    "  ready  ",
                    Style::default().fg(Color::Green),
                )
            } else {
                (
                    Style::default().fg(Color::DarkGray),
                    Style::default().fg(Color::DarkGray),
                    "  ─────  ",
                    Style::default().fg(Color::DarkGray),
                )
            };

            let line = Line::from(vec![
                Span::styled(format!("  {:>2}  ", item.short), tag_style),
                Span::styled(format!("{:<26}", item.label), label_style),
                Span::styled(status_text, status_style),
                Span::styled(
                    format!("  {}", item.description),
                    Style::default().fg(Color::DarkGray),
                ),
            ]);
            ListItem::new(line)
        })
        .collect();

    let list = List::new(rows)
        .highlight_symbol("▶")
        .highlight_style(
            Style::default()
                .bg(Color::Blue)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        );

    let mut list_state = ListState::default();
    list_state.select(Some(state.selected));
    frame.render_stateful_widget(list, chunks[2], &mut list_state);

    let footer = Paragraph::new("  ↑/↓  j/k  navigate     Enter  select     q  quit")
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(footer, chunks[3]);
}
