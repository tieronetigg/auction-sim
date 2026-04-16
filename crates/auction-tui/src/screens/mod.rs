pub mod auction;
pub mod debrief;
pub mod intro;
pub mod menu;

use ratatui::{
    layout::Alignment,
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, Screen};

pub fn render(frame: &mut Frame, app: &App) {
    match &app.screen {
        Screen::MainMenu(state) => menu::render(frame, state),
        Screen::AuctionIntro(state) => intro::render(frame, state),
        Screen::LiveAuction(state) => auction::render(frame, state),
        Screen::Debrief(state) => debrief::render(frame, state),
        Screen::Placeholder { title, message } => render_placeholder(frame, title, message),
    }
}

fn render_placeholder(frame: &mut Frame, title: &str, message: &str) {
    let area = frame.size();

    let block = Block::default()
        .title(format!(" {} ", title))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let indented: String = message
        .lines()
        .map(|l| if l.is_empty() { String::new() } else { format!("  {}", l) })
        .collect::<Vec<_>>()
        .join("\n");

    let para = Paragraph::new(format!("\n{}", indented))
        .style(Style::default().fg(Color::White))
        .wrap(Wrap { trim: false });

    frame.render_widget(para, inner);
}
