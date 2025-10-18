use crate::tui::app::TuiApp;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph, Sparkline},
    Frame,
};

pub fn draw(f: &mut Frame, area: Rect, app: &TuiApp) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Uptime + Status
            Constraint::Length(4),  // Requests/sec sparkline
            Constraint::Length(3),  // Latency percentiles
            Constraint::Length(3),  // Servers + Cache
            Constraint::Length(3),  // Error rate + Batches
        ])
        .split(area);

    draw_uptime_status(f, chunks[0], app);
    draw_requests_sparkline(f, chunks[1], app);
    draw_latency_percentiles(f, chunks[2], app);
    draw_servers_cache(f, chunks[3], app);
    draw_error_batches(f, chunks[4], app);
}

fn draw_uptime_status(f: &mut Frame, area: Rect, app: &TuiApp) {
    let uptime = format_uptime(app.metrics_snapshot.uptime_seconds);
    let status = if app.metrics_snapshot.error_rate < 0.01 {
        ("Healthy", Color::Green)
    } else if app.metrics_snapshot.error_rate < 0.05 {
        ("Degraded", Color::Yellow)
    } else {
        ("Unhealthy", Color::Red)
    };

    let text = Paragraph::new(Line::from(vec![
        Span::raw("Uptime: "),
        Span::styled(uptime, Style::default().fg(Color::Cyan)),
        Span::raw("          Status: "),
        Span::styled(status.0, Style::default().fg(status.1).add_modifier(Modifier::BOLD)),
    ]))
    .block(Block::default().borders(Borders::ALL).title("Overview"));

    f.render_widget(text, area);
}

fn draw_requests_sparkline(f: &mut Frame, area: Rect, app: &TuiApp) {
    // TODO: Track request history for sparkline
    // For now, use placeholder data
    let data: Vec<u64> = vec![10, 12, 15, 18, 22, 25, 28, 30, 32, 35, 38, 40];

    let sparkline = Sparkline::default()
        .block(
            Block::default().borders(Borders::ALL).title(format!(
                "Requests/sec: {:.1}",
                app.metrics_snapshot.requests_per_second
            )),
        )
        .data(&data)
        .style(Style::default().fg(Color::Green));

    f.render_widget(sparkline, area);
}

fn draw_latency_percentiles(f: &mut Frame, area: Rect, app: &TuiApp) {
    let text = Paragraph::new(Line::from(vec![
        Span::raw("Latency p50: "),
        Span::styled(
            format!("{:.1}ms", app.metrics_snapshot.latency_p50),
            Style::default().fg(Color::Green),
        ),
        Span::raw("  p95: "),
        Span::styled(
            format!("{:.1}ms", app.metrics_snapshot.latency_p95),
            Style::default().fg(Color::Yellow),
        ),
        Span::raw("  p99: "),
        Span::styled(
            format!("{:.1}ms", app.metrics_snapshot.latency_p99),
            Style::default().fg(Color::Red),
        ),
    ]))
    .block(Block::default().borders(Borders::ALL));

    f.render_widget(text, area);
}

fn draw_servers_cache(f: &mut Frame, area: Rect, app: &TuiApp) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Servers
    let servers_text = Paragraph::new(format!(
        "Active Servers: {}/{}",
        app.metrics_snapshot.active_servers, app.metrics_snapshot.total_servers
    ))
    .block(Block::default().borders(Borders::ALL));
    f.render_widget(servers_text, chunks[0]);

    // Cache hit rate gauge
    let cache_gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title("Cache Hit Rate"))
        .gauge_style(Style::default().fg(Color::Cyan))
        .percent((app.metrics_snapshot.cache_hit_rate * 100.0) as u16);
    f.render_widget(cache_gauge, chunks[1]);
}

fn draw_error_batches(f: &mut Frame, area: Rect, app: &TuiApp) {
    let text = Paragraph::new(Line::from(vec![
        Span::raw("Error Rate: "),
        Span::styled(
            format!("{:.2}%", app.metrics_snapshot.error_rate * 100.0),
            Style::default().fg(Color::Red),
        ),
        Span::raw("       Active Batches: "),
        Span::styled(
            app.metrics_snapshot.active_batches.to_string(),
            Style::default().fg(Color::Yellow),
        ),
    ]))
    .block(Block::default().borders(Borders::ALL));

    f.render_widget(text, area);
}

fn format_uptime(seconds: u64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    format!("{}h {}m", hours, minutes)
}
