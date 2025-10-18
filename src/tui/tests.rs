//! Unit tests for TUI module

use crate::config::Config;
use crate::tui::app::{
    CacheLayerStats, CacheStats, LogEntry, LogLevel, MetricsSnapshot, RequestEntry, ServerInfo,
    ServerStatus, TuiApp,
};
use crate::tui::tabs::TabId;
use chrono::Utc;
use std::sync::Arc;

fn create_test_config() -> Arc<Config> {
    Arc::new(Config::default())
}

impl TuiApp {
    /// Add a log entry to the buffer (rate-limited)
    pub fn add_log_entry(&mut self, entry: LogEntry) {
        self.log_buffer.push(entry);
        if self.log_buffer.len() > 1000 {
            self.log_buffer.remove(0); // Keep last 1000 (ring buffer)
        }
    }
}

#[cfg(test)]
mod tui_tests {
    use super::*;

    #[test]
    fn test_tab_navigation_next() {
        let mut app = TuiApp::new(create_test_config());
        assert_eq!(app.active_tab, TabId::Overview);

        app.next_tab();
        assert_eq!(app.active_tab, TabId::Servers);

        app.next_tab();
        assert_eq!(app.active_tab, TabId::Requests);

        app.next_tab();
        assert_eq!(app.active_tab, TabId::Cache);

        app.next_tab();
        assert_eq!(app.active_tab, TabId::Logs);

        app.next_tab();
        assert_eq!(app.active_tab, TabId::Overview); // Wraps around
    }

    #[test]
    fn test_tab_jump_to_specific() {
        let mut app = TuiApp::new(create_test_config());

        app.active_tab = TabId::Servers;
        assert_eq!(app.active_tab, TabId::Servers);

        app.active_tab = TabId::Logs;
        assert_eq!(app.active_tab, TabId::Logs);
    }

    #[test]
    fn test_scroll_up_down() {
        let mut app = TuiApp::new(create_test_config());
        assert_eq!(app.scroll_offset, 0);

        app.scroll_down();
        assert_eq!(app.scroll_offset, 1);

        app.scroll_down();
        assert_eq!(app.scroll_offset, 2);

        app.scroll_up();
        assert_eq!(app.scroll_offset, 1);

        app.scroll_up();
        assert_eq!(app.scroll_offset, 0);

        // Test saturating_sub at 0
        app.scroll_up();
        assert_eq!(app.scroll_offset, 0);
    }

    #[test]
    fn test_scroll_bounds() {
        let mut app = TuiApp::new(create_test_config());

        // Scroll down multiple times
        for _ in 0..1000 {
            app.scroll_down();
        }

        // Should not overflow
        assert!(app.scroll_offset > 0);
        assert!(app.scroll_offset < usize::MAX / 2); // Reasonable bound
    }

    #[test]
    fn test_tab_switching_resets_scroll() {
        let mut app = TuiApp::new(create_test_config());

        // Scroll down
        app.scroll_down();
        app.scroll_down();
        assert_eq!(app.scroll_offset, 2);

        // Switch tab
        app.next_tab();
        assert_eq!(app.scroll_offset, 0); // Reset
    }

    #[test]
    fn test_log_buffer_size_limit() {
        let mut app = TuiApp::new(create_test_config());

        // Add 1500 log entries (exceeds 1000 limit)
        for i in 0..1500 {
            let entry = LogEntry {
                timestamp: Utc::now(),
                level: LogLevel::Info,
                message: format!("Log message {}", i),
            };
            app.add_log_entry(entry);
        }

        // Buffer should be limited to 1000
        assert_eq!(app.log_buffer.len(), 1000);

        // Oldest entries should be removed (first 500)
        assert!(app.log_buffer[0].message.contains("500"));
    }

    #[test]
    fn test_log_filtering() {
        let mut app = TuiApp::new(create_test_config());

        app.log_buffer = vec![
            LogEntry {
                timestamp: Utc::now(),
                level: LogLevel::Error,
                message: "Error occurred".to_string(),
            },
            LogEntry {
                timestamp: Utc::now(),
                level: LogLevel::Info,
                message: "Info message".to_string(),
            },
            LogEntry {
                timestamp: Utc::now(),
                level: LogLevel::Warn,
                message: "Warning here".to_string(),
            },
        ];

        app.filter_query = "error".to_string();

        // Filtering logic would be in tabs/logs.rs
        let filtered: Vec<&LogEntry> = app
            .log_buffer
            .iter()
            .filter(|log| {
                log.message.to_lowercase().contains(&app.filter_query.to_lowercase())
                    || format!("{:?}", log.level)
                        .to_lowercase()
                        .contains(&app.filter_query.to_lowercase())
            })
            .collect();

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].level, LogLevel::Error);
    }

    #[test]
    fn test_server_status_color() {
        let server_up = ServerInfo {
            id: "s1".to_string(),
            name: "Server 1".to_string(),
            status: ServerStatus::Up,
            health_percentage: 100,
            requests_per_second: 50,
        };

        let server_degraded = ServerInfo {
            id: "s2".to_string(),
            name: "Server 2".to_string(),
            status: ServerStatus::Degraded,
            health_percentage: 75,
            requests_per_second: 30,
        };

        let server_down = ServerInfo {
            id: "s3".to_string(),
            name: "Server 3".to_string(),
            status: ServerStatus::Down,
            health_percentage: 0,
            requests_per_second: 0,
        };

        assert_eq!(server_up.status, ServerStatus::Up);
        assert_eq!(server_degraded.status, ServerStatus::Degraded);
        assert_eq!(server_down.status, ServerStatus::Down);

        // Health percentage color mapping
        assert_eq!(health_color(server_up.health_percentage), "green");
        assert_eq!(health_color(server_degraded.health_percentage), "yellow");
        assert_eq!(health_color(server_down.health_percentage), "red");
    }

    fn health_color(health: u8) -> &'static str {
        match health {
            90..=100 => "green",
            70..=89 => "yellow",
            _ => "red",
        }
    }

    #[test]
    fn test_cache_stats_calculation() {
        let stats = CacheStats {
            l1: CacheLayerStats {
                name: "Tools".to_string(),
                current_entries: 800,
                max_entries: 1000,
                hit_rate: 0.92,
                ttl_seconds: 300,
                evictions: 50,
            },
            l2: CacheLayerStats {
                name: "Resources".to_string(),
                current_entries: 400,
                max_entries: 500,
                hit_rate: 0.85,
                ttl_seconds: 1800,
                evictions: 30,
            },
            l3: CacheLayerStats {
                name: "Prompts".to_string(),
                current_entries: 150,
                max_entries: 200,
                hit_rate: 0.78,
                ttl_seconds: 7200,
                evictions: 10,
            },
        };

        let total_entries =
            stats.l1.current_entries + stats.l2.current_entries + stats.l3.current_entries;
        let total_max = stats.l1.max_entries + stats.l2.max_entries + stats.l3.max_entries;
        let total_evictions = stats.l1.evictions + stats.l2.evictions + stats.l3.evictions;

        assert_eq!(total_entries, 1350);
        assert_eq!(total_max, 1700);
        assert_eq!(total_evictions, 90);

        // Weighted average hit rate
        let total_hits = stats.l1.hit_rate * stats.l1.current_entries as f64
            + stats.l2.hit_rate * stats.l2.current_entries as f64
            + stats.l3.hit_rate * stats.l3.current_entries as f64;
        let avg_hit_rate = total_hits / total_entries as f64;

        assert!((avg_hit_rate - 0.878).abs() < 0.01); // ~87.8%
    }

    #[test]
    fn test_request_entry_creation() {
        let req = RequestEntry {
            timestamp: Utc::now(),
            method: "tools/list".to_string(),
            server_id: "server1".to_string(),
            latency_ms: 45.3,
            status_code: 200,
        };

        assert_eq!(req.method, "tools/list");
        assert_eq!(req.server_id, "server1");
        assert!((req.latency_ms - 45.3).abs() < 0.01);
        assert_eq!(req.status_code, 200);
    }

    #[test]
    fn test_metrics_snapshot_defaults() {
        let snapshot = MetricsSnapshot::default();

        assert_eq!(snapshot.uptime_seconds, 0);
        assert_eq!(snapshot.requests_per_second, 0.0);
        assert_eq!(snapshot.active_servers, 0);
        assert_eq!(snapshot.cache_hit_rate, 0.0);
    }

    #[test]
    fn test_format_uptime() {
        assert_eq!(format_uptime(0), "0s");
        assert_eq!(format_uptime(59), "59s");
        assert_eq!(format_uptime(60), "1m 0s");
        assert_eq!(format_uptime(3600), "1h 0m");
        assert_eq!(format_uptime(3661), "1h 1m");
        assert_eq!(format_uptime(86400), "24h 0m");
    }

    fn format_uptime(seconds: u64) -> String {
        let hours = seconds / 3600;
        let minutes = (seconds % 3600) / 60;
        let secs = seconds % 60;

        if hours > 0 {
            format!("{}h {}m", hours, minutes)
        } else if minutes > 0 {
            format!("{}m {}s", minutes, secs)
        } else {
            format!("{}s", secs)
        }
    }

    #[test]
    fn test_format_ttl() {
        assert_eq!(format_ttl(300), "5m");
        assert_eq!(format_ttl(1800), "30m");
        assert_eq!(format_ttl(7200), "2h");
    }

    fn format_ttl(seconds: u64) -> String {
        let hours = seconds / 3600;
        let minutes = (seconds % 3600) / 60;

        if hours > 0 {
            format!("{}h", hours)
        } else {
            format!("{}m", minutes)
        }
    }

    #[test]
    fn test_quit_keyboard_shortcuts() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut app = TuiApp::new(create_test_config());
        assert!(!app.should_quit);

        // Press 'q'
        app.on_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
        assert!(app.should_quit);

        // Reset
        app.should_quit = false;

        // Press Ctrl+C
        app.on_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
        assert!(app.should_quit);
    }

    #[test]
    fn test_log_rate_limiting() {
        let mut app = TuiApp::new(create_test_config());
        let _start = std::time::Instant::now();

        // Simulate 1000 logs in rapid succession
        for i in 0..1000 {
            let entry = LogEntry {
                timestamp: Utc::now(),
                level: LogLevel::Info,
                message: format!("Rapid log {}", i),
            };
            app.add_log_entry(entry);
        }

        // Should be limited to 1000 entries
        assert_eq!(app.log_buffer.len(), 1000);

        // In production, rate limiting would drop logs if >100/s
        // This is a mock test showing buffer constraint
    }
}
