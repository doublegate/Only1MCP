//! UI rendering logic

use super::{app::TuiApp, tabs::TabId};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Tabs},
    Frame,
};

pub fn draw(f: &mut Frame, app: &TuiApp) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Tab bar
            Constraint::Min(0),    // Content area
            Constraint::Length(1), // Status bar
        ])
        .split(f.size());

    // Tab bar
    draw_tabs(f, chunks[0], app);

    // Content area (tab-specific)
    match app.active_tab {
        TabId::Overview => super::tabs::overview::draw(f, chunks[1], app),
        TabId::Servers => super::tabs::servers::draw(f, chunks[1], app),
        TabId::Requests => super::tabs::requests::draw(f, chunks[1], app),
        TabId::Cache => super::tabs::cache::draw(f, chunks[1], app),
        TabId::Logs => super::tabs::logs::draw(f, chunks[1], app),
    }

    // Status bar
    draw_status_bar(f, chunks[2], app);
}

fn draw_tabs(f: &mut Frame, area: Rect, app: &TuiApp) {
    let titles = vec!["1:Overview", "2:Servers", "3:Requests", "4:Cache", "5:Logs"];

    let selected = match app.active_tab {
        TabId::Overview => 0,
        TabId::Servers => 1,
        TabId::Requests => 2,
        TabId::Cache => 3,
        TabId::Logs => 4,
    };

    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL).title("Only1MCP TUI"))
        .select(selected)
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));

    f.render_widget(tabs, area);
}

fn draw_status_bar(f: &mut Frame, area: Rect, _app: &TuiApp) {
    let status = Paragraph::new(Line::from(vec![
        Span::raw("Press "),
        Span::styled("q", Style::default().fg(Color::Yellow)),
        Span::raw(" to quit | "),
        Span::styled("Tab", Style::default().fg(Color::Yellow)),
        Span::raw(" to switch tabs | "),
        Span::styled("↑↓", Style::default().fg(Color::Yellow)),
        Span::raw(" to scroll"),
    ]))
    .style(Style::default().bg(Color::DarkGray));

    f.render_widget(status, area);
}
