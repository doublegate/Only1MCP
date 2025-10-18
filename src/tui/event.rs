//! Event types for TUI updates

use super::app::{LogEntry, MetricsSnapshot, ServerInfo};

#[derive(Clone)]
pub enum Event {
    /// Metrics snapshot updated
    MetricsUpdate(MetricsSnapshot),

    /// Server list updated
    ServersUpdate(Vec<ServerInfo>),

    /// New log message
    LogMessage(LogEntry),

    /// Quit signal
    Quit,
}
