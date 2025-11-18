//! Simplified GUI Backend for Only1MCP
//!
//! Provides a clean, dependency-free API for building GUI applications.
//! This module is designed to be easily integrated with Tauri, web dashboards, or other frontends.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Simplified GUI state without complex dependencies
pub struct GuiBackend {
    /// Proxy running status
    is_running: Arc<RwLock<bool>>,

    /// Active connections count
    active_connections: Arc<RwLock<u32>>,

    /// Start time
    start_time: DateTime<Utc>,

    /// Server list
    servers: Arc<RwLock<Vec<ServerInfo>>>,

    /// Metrics cache
    metrics: Arc<RwLock<HashMap<String, Vec<MetricPoint>>>>,

    /// System events log
    events: Arc<RwLock<Vec<SystemEvent>>>,
}

impl GuiBackend {
    pub fn new() -> Self {
        Self {
            is_running: Arc::new(RwLock::new(false)),
            active_connections: Arc::new(RwLock::new(0)),
            start_time: Utc::now(),
            servers: Arc::new(RwLock::new(Vec::new())),
            metrics: Arc::new(RwLock::new(HashMap::new())),
            events: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Get dashboard overview
    pub async fn get_dashboard(&self) -> DashboardData {
        let is_running = *self.is_running.read().await;
        let active_connections = *self.active_connections.read().await;
        let servers = self.servers.read().await;
        let uptime = (Utc::now() - self.start_time).num_seconds() as u64;

        let total_servers = servers.len();
        let healthy_servers = servers.iter().filter(|s| s.healthy).count();

        DashboardData {
            is_running,
            uptime_seconds: uptime,
            active_connections,
            total_servers,
            healthy_servers,
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    /// Set running status
    pub async fn set_running(&self, running: bool) {
        let mut is_running = self.is_running.write().await;
        *is_running = running;

        self.log_event(SystemEvent {
            timestamp: Utc::now(),
            event_type: if running {
                "proxy_started".to_string()
            } else {
                "proxy_stopped".to_string()
            },
            message: format!("Proxy {}", if running { "started" } else { "stopped" }),
            severity: EventSeverity::Info,
        })
        .await;
    }

    /// Update active connections
    pub async fn set_active_connections(&self, count: u32) {
        let mut connections = self.active_connections.write().await;
        *connections = count;
    }

    /// Register a server
    pub async fn add_server(&self, server: ServerInfo) {
        let mut servers = self.servers.write().await;
        servers.push(server);
    }

    /// Get all servers
    pub async fn get_servers(&self) -> Vec<ServerInfo> {
        let servers = self.servers.read().await;
        servers.clone()
    }

    /// Update server health
    pub async fn update_server_health(&self, server_id: &str, healthy: bool) {
        let mut servers = self.servers.write().await;
        if let Some(server) = servers.iter_mut().find(|s| s.id == server_id) {
            server.healthy = healthy;
        }
    }

    /// Record metric
    pub async fn record_metric(&self, name: String, value: f64) {
        let mut metrics = self.metrics.write().await;
        let points = metrics.entry(name).or_insert_with(Vec::new);

        points.push(MetricPoint {
            timestamp: Utc::now(),
            value,
        });

        // Keep only recent 1000 points
        if points.len() > 1000 {
            points.remove(0);
        }
    }

    /// Get metric data
    pub async fn get_metric(&self, name: &str) -> Option<Vec<MetricPoint>> {
        let metrics = self.metrics.read().await;
        metrics.get(name).cloned()
    }

    /// Get all metric names
    pub async fn list_metrics(&self) -> Vec<String> {
        let metrics = self.metrics.read().await;
        metrics.keys().cloned().collect()
    }

    /// Log system event
    pub async fn log_event(&self, event: SystemEvent) {
        let mut events = self.events.write().await;
        events.push(event);

        // Keep only recent 500 events
        if events.len() > 500 {
            events.remove(0);
        }
    }

    /// Get recent events
    pub async fn get_events(&self, limit: usize) -> Vec<SystemEvent> {
        let events = self.events.read().await;
        events.iter().rev().take(limit).cloned().collect()
    }

    /// Get statistics
    pub async fn get_stats(&self) -> SystemStats {
        let dashboard = self.get_dashboard().await;
        let metrics = self.metrics.read().await;
        let events = self.events.read().await;

        SystemStats {
            uptime_seconds: dashboard.uptime_seconds,
            total_servers: dashboard.total_servers,
            healthy_servers: dashboard.healthy_servers,
            active_connections: dashboard.active_connections,
            total_metrics: metrics.len(),
            total_events: events.len(),
        }
    }
}

impl Default for GuiBackend {
    fn default() -> Self {
        Self::new()
    }
}

/// Dashboard data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardData {
    pub is_running: bool,
    pub uptime_seconds: u64,
    pub active_connections: u32,
    pub total_servers: usize,
    pub healthy_servers: usize,
    pub version: String,
}

/// Server information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub id: String,
    pub name: String,
    pub transport_type: String,
    pub healthy: bool,
    pub enabled: bool,
}

/// Metric data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricPoint {
    pub timestamp: DateTime<Utc>,
    pub value: f64,
}

/// System event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemEvent {
    pub timestamp: DateTime<Utc>,
    pub event_type: String,
    pub message: String,
    pub severity: EventSeverity,
}

/// Event severity
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EventSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// System statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStats {
    pub uptime_seconds: u64,
    pub total_servers: usize,
    pub healthy_servers: usize,
    pub active_connections: u32,
    pub total_metrics: usize,
    pub total_events: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_gui_backend_creation() {
        let backend = GuiBackend::new();
        let dashboard = backend.get_dashboard().await;

        assert_eq!(dashboard.is_running, false);
        assert_eq!(dashboard.active_connections, 0);
        assert_eq!(dashboard.total_servers, 0);
    }

    #[tokio::test]
    async fn test_running_status() {
        let backend = GuiBackend::new();

        backend.set_running(true).await;
        let dashboard = backend.get_dashboard().await;
        assert_eq!(dashboard.is_running, true);

        backend.set_running(false).await;
        let dashboard = backend.get_dashboard().await;
        assert_eq!(dashboard.is_running, false);
    }

    #[tokio::test]
    async fn test_server_management() {
        let backend = GuiBackend::new();

        let server = ServerInfo {
            id: "test-1".to_string(),
            name: "Test Server".to_string(),
            transport_type: "stdio".to_string(),
            healthy: true,
            enabled: true,
        };

        backend.add_server(server).await;

        let servers = backend.get_servers().await;
        assert_eq!(servers.len(), 1);
        assert_eq!(servers[0].id, "test-1");

        backend.update_server_health("test-1", false).await;

        let servers = backend.get_servers().await;
        assert_eq!(servers[0].healthy, false);
    }

    #[tokio::test]
    async fn test_metrics() {
        let backend = GuiBackend::new();

        backend.record_metric("cpu_usage".to_string(), 75.5).await;
        backend.record_metric("cpu_usage".to_string(), 80.0).await;

        let metrics = backend.get_metric("cpu_usage").await.unwrap();
        assert_eq!(metrics.len(), 2);
        assert_eq!(metrics[0].value, 75.5);
        assert_eq!(metrics[1].value, 80.0);

        let names = backend.list_metrics().await;
        assert_eq!(names.len(), 1);
        assert!(names.contains(&"cpu_usage".to_string()));
    }

    #[tokio::test]
    async fn test_events() {
        let backend = GuiBackend::new();

        backend
            .log_event(SystemEvent {
                timestamp: Utc::now(),
                event_type: "test".to_string(),
                message: "Test event".to_string(),
                severity: EventSeverity::Info,
            })
            .await;

        let events = backend.get_events(10).await;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "test");
    }

    #[tokio::test]
    async fn test_stats() {
        let backend = GuiBackend::new();

        backend
            .add_server(ServerInfo {
                id: "test".to_string(),
                name: "Test".to_string(),
                transport_type: "stdio".to_string(),
                healthy: true,
                enabled: true,
            })
            .await;

        backend.record_metric("test_metric".to_string(), 42.0).await;

        let stats = backend.get_stats().await;
        assert_eq!(stats.total_servers, 1);
        assert_eq!(stats.total_metrics, 1);
    }
}
