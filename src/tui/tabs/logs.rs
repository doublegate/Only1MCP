use crate::tui::app::{LogEntry, LogLevel, TuiApp};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

pub fn draw(f: &mut Frame, area: Rect, app: &TuiApp) {
    let logs = &app.log_buffer;
    let filter = &app.filter_query;

    // Split into filter input and logs list
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Filter input
            Constraint::Min(0),    // Logs list
        ])
        .split(area);

    // Draw filter input
    draw_filter_input(f, chunks[0], filter);

    // Filter logs based on query
    let filtered_logs: Vec<&LogEntry> = if filter.is_empty() {
        logs.iter().collect()
    } else {
        logs.iter()
            .filter(|log| {
                log.message.to_lowercase().contains(&filter.to_lowercase())
                    || format!("{:?}", log.level).to_lowercase().contains(&filter.to_lowercase())
            })
            .collect()
    };

    // Calculate visible window
    let visible_height = chunks[1].height.saturating_sub(2) as usize;
    let total_items = filtered_logs.len();
    let scroll_offset = app.scroll_offset.min(total_items.saturating_sub(visible_height));

    let visible_logs: Vec<&LogEntry> = filtered_logs
        .iter()
        .rev() // Most recent first
        .skip(scroll_offset)
        .take(visible_height)
        .copied()
        .collect();

    // Create log items
    let items: Vec<ListItem> = visible_logs
        .iter()
        .map(|log| {
            let time_str = log.timestamp.format("%H:%M:%S").to_string();
            let level_color = match log.level {
                LogLevel::Error => Color::Red,
                LogLevel::Warn => Color::Yellow,
                LogLevel::Info => Color::Blue,
                LogLevel::Debug => Color::Gray,
                LogLevel::Trace => Color::DarkGray,
            };
            let level_str = format!("{:5}", format!("{:?}", log.level).to_uppercase());

            let line = Line::from(vec![
                Span::styled(time_str, Style::default().fg(Color::Cyan)),
                Span::raw(" "),
                Span::styled(
                    level_str,
                    Style::default().fg(level_color).add_modifier(Modifier::BOLD),
                ),
                Span::raw(" "),
                Span::raw(&log.message),
            ]);

            ListItem::new(line)
        })
        .collect();

    let list = List::new(items).block(Block::default().borders(Borders::ALL).title(format!(
        "Logs ({} total, {} filtered, showing {}-{}) - Use ↑↓ to scroll, / to filter",
        logs.len(),
        total_items,
        scroll_offset + 1,
        (scroll_offset + visible_height).min(total_items)
    )));

    f.render_widget(list, chunks[1]);
}

fn draw_filter_input(f: &mut Frame, area: Rect, filter: &str) {
    let input_text = if filter.is_empty() {
        "Press '/' to filter logs...".to_string()
    } else {
        format!("Filter: {}", filter)
    };

    let input = Paragraph::new(input_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Filter")
                .style(Style::default().fg(Color::Yellow)),
        )
        .style(Style::default().fg(Color::White));

    f.render_widget(input, area);
}
