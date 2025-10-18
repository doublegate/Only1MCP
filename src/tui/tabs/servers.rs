use crate::tui::app::{ServerStatus, TuiApp};
use ratatui::{
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table},
    Frame,
};

pub fn draw(f: &mut Frame, area: Rect, app: &TuiApp) {
    let servers = &app.servers_snapshot;

    // Table headers
    let headers = Row::new(vec![
        Cell::from("ID").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Cell::from("Name").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Cell::from("Status").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Cell::from("Health").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Cell::from("RPS").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
    ])
    .height(1);

    // Table rows
    let rows: Vec<Row> = servers
        .iter()
        .map(|server| {
            let status_cell = match server.status {
                ServerStatus::Up => Cell::from("✅ UP").style(Style::default().fg(Color::Green)),
                ServerStatus::Degraded => {
                    Cell::from("⚠️  DEGRADED").style(Style::default().fg(Color::Yellow))
                },
                ServerStatus::Down => Cell::from("🔴 DOWN").style(Style::default().fg(Color::Red)),
            };

            let health_text = format!("{}%", server.health_percentage);
            let health_color = match server.health_percentage {
                90..=100 => Color::Green,
                70..=89 => Color::Yellow,
                _ => Color::Red,
            };
            let health_cell = Cell::from(health_text).style(Style::default().fg(health_color));

            Row::new(vec![
                Cell::from(server.id.clone()),
                Cell::from(server.name.clone()),
                status_cell,
                health_cell,
                Cell::from(format!("{}", server.requests_per_second)),
            ])
        })
        .collect();

    // Create table
    let table = Table::new(
        rows,
        [
            Constraint::Length(8),  // ID
            Constraint::Length(20), // Name
            Constraint::Length(12), // Status
            Constraint::Length(8),  // Health
            Constraint::Length(8),  // RPS
        ],
    )
    .header(headers)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!("Servers ({} total)", servers.len())),
    )
    .column_spacing(2);

    f.render_widget(table, area);
}
