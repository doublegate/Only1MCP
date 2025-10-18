use crate::tui::app::{RequestEntry, TuiApp};
use ratatui::{
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table},
    Frame,
};

pub fn draw(f: &mut Frame, area: Rect, app: &TuiApp) {
    let requests = &app.request_log;

    // Table headers
    let headers = Row::new(vec![
        Cell::from("Time").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Cell::from("Method").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Cell::from("Server").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Cell::from("Latency").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Cell::from("Status").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
    ])
    .height(1);

    // Calculate visible window based on scroll offset
    let visible_height = area.height.saturating_sub(3) as usize; // Account for borders and header
    let total_items = requests.len();
    let scroll_offset = app.scroll_offset.min(total_items.saturating_sub(visible_height));

    let visible_requests: Vec<&RequestEntry> = requests
        .iter()
        .rev() // Most recent first
        .skip(scroll_offset)
        .take(visible_height)
        .collect();

    // Table rows
    let rows: Vec<Row> = visible_requests
        .iter()
        .map(|req| {
            let time_str = req.timestamp.format("%H:%M:%S").to_string();

            let status_text = format!("{}", req.status_code);
            let status_color = match req.status_code {
                200..=299 => Color::Green,
                400..=499 => Color::Yellow,
                500..=599 => Color::Red,
                _ => Color::White,
            };
            let status_cell = Cell::from(status_text).style(Style::default().fg(status_color));

            let latency_text = format!("{:.2}ms", req.latency_ms);
            let latency_color = if req.latency_ms < 50.0 {
                Color::Green
            } else if req.latency_ms < 200.0 {
                Color::Yellow
            } else {
                Color::Red
            };
            let latency_cell = Cell::from(latency_text).style(Style::default().fg(latency_color));

            Row::new(vec![
                Cell::from(time_str),
                Cell::from(req.method.clone()),
                Cell::from(req.server_id.clone()),
                latency_cell,
                status_cell,
            ])
        })
        .collect();

    // Create table
    let table = Table::new(
        rows,
        [
            Constraint::Length(10), // Time
            Constraint::Length(12), // Method
            Constraint::Length(20), // Server
            Constraint::Length(12), // Latency
            Constraint::Length(8),  // Status
        ],
    )
    .header(headers)
    .block(Block::default().borders(Borders::ALL).title(format!(
        "Requests ({} total, showing {}-{}) - Use ↑↓ to scroll",
        total_items,
        scroll_offset + 1,
        (scroll_offset + visible_height).min(total_items)
    )))
    .column_spacing(2);

    f.render_widget(table, area);
}
