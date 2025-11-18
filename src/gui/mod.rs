//! GUI Backend API for Only1MCP
//!
//! Provides a comprehensive API for building graphical interfaces (Tauri, web dashboards, etc.)
//! This module exposes all proxy functionality through a clean, serializable interface.

pub mod api;
pub mod commands;
pub mod state;
pub mod websocket;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::analytics::{AlertEvent, AnalyticsEngine, QueryResult};
use crate::ai::AIEngine;
use crate::audit::AuditEvent;
use crate::config::Config;
use crate::error::Result;
use crate::health::HealthStatus;
use crate::metrics::MetricsSnapshot;
use crate::multiregion::{MultiRegionManager, Region};
use crate::plugins::PluginMetadata;
use crate::tenancy::{Tenant, TenantId};

/// GUI state manager coordinating all subsystems
pub struct GuiState {
    /// Proxy configuration
    config: Arc<RwLock<Config>>,

    /// Analytics engine
    analytics: Arc<AnalyticsEngine>,

    /// AI optimization engine
    ai_engine: Arc<AIEngine>,

    /// Multi-region manager (optional)
    multiregion: Option<Arc<MultiRegionManager>>,

    /// Active connections count
    active_connections: Arc<RwLock<u32>>,

    /// Server health statuses
    server_health: Arc<RwLock<HashMap<String, HealthStatus>>>,

    /// Recent audit events
    audit_events: Arc<RwLock<Vec<AuditEvent>>>,

    /// Loaded plugins
    plugins: Arc<RwLock<Vec<PluginMetadata>>>,

    /// Registered tenants
    tenants: Arc<RwLock<HashMap<TenantId, Tenant>>>,

    /// Proxy start time
    start_time: DateTime<Utc>,

    /// Proxy running status
    is_running: Arc<RwLock<bool>>,
}

impl GuiState {
    pub fn new(
        config: Config,
        analytics: Arc<AnalyticsEngine>,
        ai_engine: Arc<AIEngine>,
        multiregion: Option<Arc<MultiRegionManager>>,
    ) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            analytics,
            ai_engine,
            multiregion,
            active_connections: Arc::new(RwLock::new(0)),
            server_health: Arc::new(RwLock::new(HashMap::new())),
            audit_events: Arc::new(RwLock::new(Vec::new())),
            plugins: Arc::new(RwLock::new(Vec::new())),
            tenants: Arc::new(RwLock::new(HashMap::new())),
            start_time: Utc::now(),
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    /// Get dashboard overview data
    pub async fn get_dashboard_overview(&self) -> DashboardOverview {
        let is_running = *self.is_running.read().await;
        let active_connections = *self.active_connections.read().await;
        let config = self.config.read().await;
        let server_health = self.server_health.read().await;
        let uptime = Utc::now() - self.start_time;

        let total_servers = config.servers.len();
        let healthy_servers = server_health
            .values()
            .filter(|h| matches!(h, HealthStatus::Healthy))
            .count();

        DashboardOverview {
            is_running,
            uptime_seconds: uptime.num_seconds() as u64,
            active_connections,
            total_servers,
            healthy_servers,
            version: env!("CARGO_PKG_VERSION").to_string(),
            start_time: self.start_time,
        }
    }

    /// Get all server statuses
    pub async fn get_server_statuses(&self) -> Vec<ServerStatus> {
        let config = self.config.read().await;
        let health = self.server_health.read().await;

        config
            .servers
            .iter()
            .map(|(id, server_config)| {
                let health_status = health
                    .get(id)
                    .cloned()
                    .unwrap_or(HealthStatus::Unknown);

                ServerStatus {
                    id: id.clone(),
                    name: server_config.name.clone(),
                    transport: format!("{:?}", server_config.transport),
                    health: health_status,
                    enabled: server_config.enabled,
                    url: server_config.url.clone(),
                }
            })
            .collect()
    }

    /// Get metrics data for charts
    pub async fn get_metrics(&self, metric_names: Vec<String>) -> Vec<MetricData> {
        let mut result = Vec::new();
        let repo = self.analytics.repository();

        for name in metric_names {
            if let Some(series) = repo.get(&name).await {
                let points: Vec<DataPointDto> = series
                    .points
                    .values()
                    .map(|p| DataPointDto {
                        timestamp: p.timestamp,
                        value: p.value,
                    })
                    .collect();

                result.push(MetricData {
                    name: name.clone(),
                    points,
                });
            }
        }

        result
    }

    /// Get active alerts
    pub async fn get_active_alerts(&self) -> Vec<AlertEvent> {
        let alert_manager = self.analytics.alert_manager();
        let active_alert_ids = alert_manager.get_active_alerts().await;

        // In production, fetch full alert details
        // For now, return empty vec as alerts are stored separately
        Vec::new()
    }

    /// Get recent audit log
    pub async fn get_audit_log(&self, limit: usize) -> Vec<AuditEvent> {
        let events = self.audit_events.read().await;
        events.iter().rev().take(limit).cloned().collect()
    }

    /// Get plugin list
    pub async fn get_plugins(&self) -> Vec<PluginMetadata> {
        let plugins = self.plugins.read().await;
        plugins.clone()
    }

    /// Get tenant list
    pub async fn get_tenants(&self) -> Vec<Tenant> {
        let tenants = self.tenants.read().await;
        tenants.values().cloned().collect()
    }

    /// Get AI optimization status
    pub async fn get_ai_status(&self) -> AiStatus {
        let ml_router = self.ai_engine.ml_router();
        let predictive_cache = self.ai_engine.predictive_cache();
        let anomaly_detector = self.ai_engine.anomaly_detector();

        let server_scores = ml_router.get_server_stats().await;
        let cache_stats = predictive_cache.get_stats().await;
        let recent_anomalies = anomaly_detector.get_recent_anomalies(10).await;

        AiStatus {
            ml_router_servers: server_scores.len(),
            cache_hit_rate: cache_stats.hit_rate,
            recent_anomalies_count: recent_anomalies.len(),
            predictions_active: true,
        }
    }

    /// Get multi-region status (if enabled)
    pub async fn get_region_status(&self) -> Option<Vec<RegionStatus>> {
        if let Some(ref manager) = self.multiregion {
            let regions = manager.list_regions().await;
            Some(
                regions
                    .into_iter()
                    .map(|r| RegionStatus {
                        id: r.id.0,
                        name: r.name,
                        healthy: r.healthy,
                        servers: r.servers.len(),
                        load: r.current_load,
                        capacity: r.capacity,
                        latency_ms: r.avg_latency_ms,
                    })
                    .collect(),
            )
        } else {
            None
        }
    }

    /// Update server health
    pub async fn update_server_health(&self, server_id: String, health: HealthStatus) {
        let mut health_map = self.server_health.write().await;
        health_map.insert(server_id, health);
    }

    /// Update active connections count
    pub async fn set_active_connections(&self, count: u32) {
        let mut connections = self.active_connections.write().await;
        *connections = count;
    }

    /// Set proxy running status
    pub async fn set_running(&self, running: bool) {
        let mut is_running = self.is_running.write().await;
        *is_running = running;
    }

    /// Add audit event
    pub async fn add_audit_event(&self, event: AuditEvent) {
        let mut events = self.audit_events.write().await;
        events.push(event);

        // Keep only recent 1000 events
        if events.len() > 1000 {
            events.remove(0);
        }
    }

    /// Reload configuration
    pub async fn reload_config(&self, new_config: Config) -> Result<()> {
        let mut config = self.config.write().await;
        *config = new_config;
        Ok(())
    }

    /// Get current configuration
    pub async fn get_config(&self) -> Config {
        let config = self.config.read().await;
        config.clone()
    }
}

/// Dashboard overview data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardOverview {
    pub is_running: bool,
    pub uptime_seconds: u64,
    pub active_connections: u32,
    pub total_servers: usize,
    pub healthy_servers: usize,
    pub version: String,
    pub start_time: DateTime<Utc>,
}

/// Server status for GUI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerStatus {
    pub id: String,
    pub name: String,
    pub transport: String,
    pub health: HealthStatus,
    pub enabled: bool,
    pub url: Option<String>,
}

/// Metric data for charts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricData {
    pub name: String,
    pub points: Vec<DataPointDto>,
}

/// Data point DTO for GUI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataPointDto {
    pub timestamp: DateTime<Utc>,
    pub value: f64,
}

/// AI optimization status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiStatus {
    pub ml_router_servers: usize,
    pub cache_hit_rate: f64,
    pub recent_anomalies_count: usize,
    pub predictions_active: bool,
}

/// Region status for multi-region view
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionStatus {
    pub id: String,
    pub name: String,
    pub healthy: bool,
    pub servers: usize,
    pub load: u32,
    pub capacity: u32,
    pub latency_ms: f64,
}

/// Log entry for GUI log viewer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: LogLevel,
    pub target: String,
    pub message: String,
    pub fields: HashMap<String, String>,
}

/// Log level for GUI
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

/// Configuration update request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigUpdateRequest {
    pub field: String,
    pub value: serde_json::Value,
}

/// Server control command
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServerCommand {
    Start,
    Stop,
    Restart,
    Reload,
    Enable { server_id: String },
    Disable { server_id: String },
}

/// GUI event for real-time updates
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GuiEvent {
    ServerHealthChanged {
        server_id: String,
        health: HealthStatus,
    },
    MetricUpdated {
        metric: String,
        value: f64,
    },
    AlertTriggered {
        alert: AlertEvent,
    },
    AuditEvent {
        event: AuditEvent,
    },
    ConfigReloaded,
    ConnectionCountChanged {
        count: u32,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ServerConfig;
    use crate::types::TransportConfig;

    #[tokio::test]
    async fn test_gui_state_creation() {
        let config = Config::default();
        let analytics = Arc::new(AnalyticsEngine::new());
        let ai_engine = Arc::new(AIEngine::new());

        let state = GuiState::new(config, analytics, ai_engine, None);

        let overview = state.get_dashboard_overview().await;
        assert_eq!(overview.is_running, false);
        assert_eq!(overview.active_connections, 0);
    }

    #[tokio::test]
    async fn test_server_statuses() {
        let mut config = Config::default();
        config.servers.insert(
            "test-server".to_string(),
            ServerConfig {
                name: "Test Server".to_string(),
                transport: TransportConfig::Stdio {
                    command: "test".to_string(),
                    args: vec![],
                    env: HashMap::new(),
                },
                enabled: true,
                url: None,
                priority: 100,
                weight: 1,
                health_check: None,
            },
        );

        let analytics = Arc::new(AnalyticsEngine::new());
        let ai_engine = Arc::new(AIEngine::new());
        let state = GuiState::new(config, analytics, ai_engine, None);

        let statuses = state.get_server_statuses().await;
        assert_eq!(statuses.len(), 1);
        assert_eq!(statuses[0].id, "test-server");
        assert_eq!(statuses[0].name, "Test Server");
    }

    #[tokio::test]
    async fn test_active_connections_update() {
        let config = Config::default();
        let analytics = Arc::new(AnalyticsEngine::new());
        let ai_engine = Arc::new(AIEngine::new());
        let state = GuiState::new(config, analytics, ai_engine, None);

        state.set_active_connections(42).await;

        let overview = state.get_dashboard_overview().await;
        assert_eq!(overview.active_connections, 42);
    }

    #[tokio::test]
    async fn test_running_status() {
        let config = Config::default();
        let analytics = Arc::new(AnalyticsEngine::new());
        let ai_engine = Arc::new(AIEngine::new());
        let state = GuiState::new(config, analytics, ai_engine, None);

        state.set_running(true).await;

        let overview = state.get_dashboard_overview().await;
        assert_eq!(overview.is_running, true);
    }
}
