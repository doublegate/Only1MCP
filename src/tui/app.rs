//! TUI application state and main loop

use super::{event::Event, tabs::TabId, ui};
use crate::{config::Config, error::Result};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

/// Main TUI application state
pub struct TuiApp {
    // Navigation
    pub active_tab: TabId,

    // Data snapshots (updated via channels)
    pub metrics_snapshot: MetricsSnapshot,
    pub servers_snapshot: Vec<ServerInfo>,
    pub request_log: Vec<RequestEntry>,
    pub cache_stats: CacheStats,
    pub log_buffer: Vec<LogEntry>,

    // UI state
    pub scroll_offset: usize,
    pub filter_query: String,

    // Control
    pub should_quit: bool,
    pub last_update: Instant,
}

impl TuiApp {
    pub fn new(_config: Arc<Config>) -> Self {
        Self {
            active_tab: TabId::Overview,
            metrics_snapshot: MetricsSnapshot::default(),
            servers_snapshot: Vec::new(),
            request_log: Vec::new(),
            cache_stats: CacheStats::default(),
            log_buffer: Vec::new(),
            scroll_offset: 0,
            filter_query: String::new(),
            should_quit: false,
            last_update: Instant::now(),
        }
    }

    pub fn on_tick(&mut self) {
        // Called every 100ms
        self.last_update = Instant::now();
    }

    pub fn on_key(&mut self, key: crossterm::event::KeyEvent) {
        use crossterm::event::{KeyCode, KeyModifiers};

        match (key.code, key.modifiers) {
            (KeyCode::Char('q'), _) => self.should_quit = true,
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => self.should_quit = true,
            (KeyCode::Tab, _) => self.next_tab(),
            (KeyCode::Char('1'), _) => self.active_tab = TabId::Overview,
            (KeyCode::Char('2'), _) => self.active_tab = TabId::Servers,
            (KeyCode::Char('3'), _) => self.active_tab = TabId::Requests,
            (KeyCode::Char('4'), _) => self.active_tab = TabId::Cache,
            (KeyCode::Char('5'), _) => self.active_tab = TabId::Logs,
            (KeyCode::Up, _) => self.scroll_up(),
            (KeyCode::Down, _) => self.scroll_down(),
            _ => {},
        }
    }

    pub fn next_tab(&mut self) {
        self.active_tab = match self.active_tab {
            TabId::Overview => TabId::Servers,
            TabId::Servers => TabId::Requests,
            TabId::Requests => TabId::Cache,
            TabId::Cache => TabId::Logs,
            TabId::Logs => TabId::Overview,
        };
        self.scroll_offset = 0; // Reset scroll on tab change
    }

    pub fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    pub fn scroll_down(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_add(1);
    }
}

/// Run the TUI in a dedicated tokio task
pub async fn run_tui(config: Arc<Config>, event_rx: mpsc::UnboundedReceiver<Event>) -> Result<()> {
    // Spawn blocking task for terminal I/O
    tokio::task::spawn_blocking(move || run_tui_blocking(config, event_rx))
        .await
        .map_err(|e| crate::error::Error::Server(format!("TUI task failed: {}", e)))??;

    Ok(())
}

fn run_tui_blocking(config: Arc<Config>, event_rx: mpsc::UnboundedReceiver<Event>) -> Result<()> {
    let mut event_rx = event_rx;
    use crossterm::{
        event::{DisableMouseCapture, EnableMouseCapture},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    };

    // Setup terminal
    enable_raw_mode().map_err(|e| crate::error::Error::Server(format!("Terminal error: {}", e)))?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)
        .map_err(|e| crate::error::Error::Server(format!("Terminal error: {}", e)))?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)
        .map_err(|e| crate::error::Error::Server(format!("Terminal error: {}", e)))?;

    // Create app
    let mut app = TuiApp::new(config);
    let tick_duration = Duration::from_millis(100); // 10 FPS

    // Event loop
    loop {
        // Render
        terminal
            .draw(|f| ui::draw(f, &app))
            .map_err(|e| crate::error::Error::Server(format!("Render error: {}", e)))?;

        // Handle events
        if crossterm::event::poll(tick_duration)
            .map_err(|e| crate::error::Error::Server(format!("Event poll error: {}", e)))?
        {
            if let crossterm::event::Event::Key(key) = crossterm::event::read()
                .map_err(|e| crate::error::Error::Server(format!("Event read error: {}", e)))?
            {
                app.on_key(key);
            }
        }

        // Check channel for updates
        while let Ok(event) = event_rx.try_recv() {
            match event {
                Event::MetricsUpdate(snapshot) => {
                    app.metrics_snapshot = snapshot;
                },
                Event::ServersUpdate(servers) => {
                    app.servers_snapshot = servers;
                },
                Event::LogMessage(entry) => {
                    app.log_buffer.push(entry);
                    if app.log_buffer.len() > 1000 {
                        app.log_buffer.remove(0); // Keep last 1000
                    }
                },
                Event::Quit => {
                    app.should_quit = true;
                },
            }
        }

        app.on_tick();

        if app.should_quit {
            break;
        }
    }

    // Cleanup terminal
    disable_raw_mode()
        .map_err(|e| crate::error::Error::Server(format!("Terminal error: {}", e)))?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )
    .map_err(|e| crate::error::Error::Server(format!("Terminal error: {}", e)))?;
    terminal
        .show_cursor()
        .map_err(|e| crate::error::Error::Server(format!("Terminal error: {}", e)))?;

    Ok(())
}

// Placeholder types (will be implemented in later phases)
#[derive(Default, Clone)]
pub struct MetricsSnapshot {
    pub uptime_seconds: u64,
    pub requests_per_second: f64,
    pub latency_p50: f64,
    pub latency_p95: f64,
    pub latency_p99: f64,
    pub active_servers: usize,
    pub total_servers: usize,
    pub cache_hit_rate: f64,
    pub error_rate: f64,
    pub active_batches: usize,
}

#[derive(Clone)]
pub struct ServerInfo {
    pub id: String,
    pub name: String,
    pub status: ServerStatus,
    pub health_percentage: u8,
    pub requests_per_second: u32,
}

#[derive(Clone, PartialEq, Debug)]
pub enum ServerStatus {
    Up,
    Degraded,
    Down,
}

#[derive(Clone)]
pub struct LogEntry {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub level: LogLevel,
    pub message: String,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Clone)]
pub struct RequestEntry {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub method: String,
    pub server_id: String,
    pub latency_ms: f64,
    pub status_code: u16,
}

#[derive(Default, Clone)]
pub struct CacheStats {
    pub l1: CacheLayerStats,
    pub l2: CacheLayerStats,
    pub l3: CacheLayerStats,
}

#[derive(Default, Clone)]
pub struct CacheLayerStats {
    pub name: String,
    pub current_entries: usize,
    pub max_entries: usize,
    pub hit_rate: f64,
    pub ttl_seconds: u64,
    pub evictions: u64,
}
