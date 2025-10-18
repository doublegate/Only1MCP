use crate::tui::app::{CacheLayerStats, TuiApp};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Gauge, Paragraph},
    Frame,
};

pub fn draw(f: &mut Frame, area: Rect, app: &TuiApp) {
    let stats = &app.cache_stats;

    // Split area into 3 layers + summary
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(25), // L1
            Constraint::Percentage(25), // L2
            Constraint::Percentage(25), // L3
            Constraint::Percentage(25), // Summary
        ])
        .split(area);

    // Draw each layer
    draw_cache_layer(f, chunks[0], &stats.l1, "L1 (Tools)", Color::Green);
    draw_cache_layer(f, chunks[1], &stats.l2, "L2 (Resources)", Color::Blue);
    draw_cache_layer(f, chunks[2], &stats.l3, "L3 (Prompts)", Color::Magenta);

    // Draw summary
    draw_summary(f, chunks[3], stats);
}

fn draw_cache_layer(f: &mut Frame, area: Rect, stats: &CacheLayerStats, title: &str, color: Color) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .style(Style::default().fg(color));

    // Split into info and gauge
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .margin(1)
        .split(area);

    // Info text
    let info_text = format!(
        "Entries: {}/{}  |  Hit Rate: {:.1}%  |  TTL: {}s  |  Evictions: {}",
        stats.current_entries,
        stats.max_entries,
        stats.hit_rate * 100.0,
        stats.ttl_seconds,
        stats.evictions
    );
    let info = Paragraph::new(info_text).style(Style::default().fg(Color::White));

    // Utilization gauge
    let utilization = if stats.max_entries > 0 {
        (stats.current_entries as f64 / stats.max_entries as f64) * 100.0
    } else {
        0.0
    };

    let gauge_color = if utilization > 90.0 {
        Color::Red
    } else if utilization > 70.0 {
        Color::Yellow
    } else {
        Color::Green
    };

    let gauge = Gauge::default()
        .block(Block::default().borders(Borders::NONE))
        .gauge_style(Style::default().fg(gauge_color))
        .percent(utilization as u16)
        .label(format!("{:.1}%", utilization));

    f.render_widget(block, area);
    f.render_widget(info, chunks[0]);
    f.render_widget(gauge, chunks[1]);
}

fn draw_summary(f: &mut Frame, area: Rect, stats: &crate::tui::app::CacheStats) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Total Summary")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));

    let total_entries =
        stats.l1.current_entries + stats.l2.current_entries + stats.l3.current_entries;
    let total_max = stats.l1.max_entries + stats.l2.max_entries + stats.l3.max_entries;
    let total_evictions = stats.l1.evictions + stats.l2.evictions + stats.l3.evictions;

    // Calculate weighted average hit rate
    let total_hits = stats.l1.hit_rate * stats.l1.current_entries as f64
        + stats.l2.hit_rate * stats.l2.current_entries as f64
        + stats.l3.hit_rate * stats.l3.current_entries as f64;
    let avg_hit_rate = if total_entries > 0 { total_hits / total_entries as f64 } else { 0.0 };

    let summary_text = format!(
        "Total Entries: {}/{}  |  Average Hit Rate: {:.1}%  |  Total Evictions: {}",
        total_entries,
        total_max,
        avg_hit_rate * 100.0,
        total_evictions
    );

    let summary = Paragraph::new(summary_text)
        .block(block)
        .style(Style::default().fg(Color::White));

    f.render_widget(summary, area);
}
