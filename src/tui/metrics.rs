//! Metrics scraping for TUI

use crate::tui::app::MetricsSnapshot;
use prometheus::proto::MetricFamily;

pub fn scrape_metrics() -> MetricsSnapshot {
    let metrics = prometheus::default_registry().gather();

    let mut snapshot = MetricsSnapshot::default();

    for mf in metrics {
        match mf.get_name() {
            "only1mcp_proxy_uptime_seconds" => {
                snapshot.uptime_seconds = get_counter_value(&mf);
            },
            "only1mcp_requests_total" => {
                snapshot.requests_per_second = calculate_rate(&mf);
            },
            "only1mcp_request_duration_seconds" => {
                let (p50, p95, p99) = extract_percentiles(&mf);
                snapshot.latency_p50 = p50;
                snapshot.latency_p95 = p95;
                snapshot.latency_p99 = p99;
            },
            "only1mcp_servers_active" => {
                snapshot.active_servers = get_gauge_value(&mf) as usize;
            },
            "only1mcp_servers_total" => {
                snapshot.total_servers = get_gauge_value(&mf) as usize;
            },
            "only1mcp_cache_hit_rate" => {
                snapshot.cache_hit_rate = get_gauge_value(&mf);
            },
            "only1mcp_error_rate" => {
                snapshot.error_rate = get_gauge_value(&mf);
            },
            "only1mcp_active_batches" => {
                snapshot.active_batches = get_gauge_value(&mf) as usize;
            },
            _ => {},
        }
    }

    snapshot
}

fn get_counter_value(mf: &MetricFamily) -> u64 {
    mf.get_metric().first().map(|m| m.get_counter().get_value() as u64).unwrap_or(0)
}

fn get_gauge_value(mf: &MetricFamily) -> f64 {
    mf.get_metric().first().map(|m| m.get_gauge().get_value()).unwrap_or(0.0)
}

fn calculate_rate(mf: &MetricFamily) -> f64 {
    // Simple rate calculation (total / uptime)
    // In production, track previous value and calculate delta
    get_counter_value(mf) as f64 / 60.0 // Approximate req/s
}

fn extract_percentiles(mf: &MetricFamily) -> (f64, f64, f64) {
    // Extract p50, p95, p99 from histogram
    for m in mf.get_metric() {
        if m.has_histogram() {
            let h = m.get_histogram();
            // Parse quantiles from histogram
            // Simplified - in production, calculate from buckets
            return (2.0, 8.0, 15.0); // Placeholder
        }
    }
    (0.0, 0.0, 0.0)
}
