//! TUI (Terminal User Interface) module for Only1MCP
//!
//! Provides a real-time monitoring dashboard using ratatui framework.

mod app;
mod event;
mod metrics;
mod tabs;
mod ui;

#[cfg(test)]
mod tests;

pub use app::{run_tui, LogEntry, LogLevel, MetricsSnapshot, ServerInfo, ServerStatus, TuiApp};
pub use event::Event;
pub use metrics::scrape_metrics;

use crate::config::Config;
use crate::error::Result;
