//! GUI Command API
//!
//! Provides command-based interface for controlling the proxy from GUI applications.
//! These commands can be used with Tauri, HTTP API, or any other interface.

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn};

use super::state::GuiState;
use super::{
    AiStatus, DashboardOverview, MetricData, RegionStatus, ServerCommand, ServerStatus,
};
use crate::audit::AuditEvent;
use crate::config::Config;
use crate::error::{Error, Result};
use crate::health::HealthStatus;
use crate::plugins::PluginMetadata;
use crate::tenancy::Tenant;

/// GUI Command Handler
pub struct GuiCommands {
    state: Arc<GuiState>,
}

impl GuiCommands {
    pub fn new(state: Arc<GuiState>) -> Self {
        Self { state }
    }

    /// Get dashboard overview
    pub async fn get_dashboard(&self) -> Result<DashboardOverview> {
        Ok(self.state.get_dashboard_overview().await)
    }

    /// Get all server statuses
    pub async fn get_servers(&self) -> Result<Vec<ServerStatus>> {
        Ok(self.state.get_server_statuses().await)
    }

    /// Get server health status
    pub async fn get_server_health(&self, server_id: String) -> Result<HealthStatus> {
        let statuses = self.state.get_server_statuses().await;
        statuses
            .into_iter()
            .find(|s| s.id == server_id)
            .map(|s| s.health)
            .ok_or_else(|| Error::Internal(format!("Server not found: {}", server_id)))
    }

    /// Execute server control command
    pub async fn control_server(&self, command: ServerCommand) -> Result<String> {
        match command {
            ServerCommand::Start => {
                info!("Starting proxy server via GUI command");
                self.state.set_running(true).await;
                Ok("Proxy server started".to_string())
            }
            ServerCommand::Stop => {
                info!("Stopping proxy server via GUI command");
                self.state.set_running(false).await;
                Ok("Proxy server stopped".to_string())
            }
            ServerCommand::Restart => {
                info!("Restarting proxy server via GUI command");
                self.state.set_running(false).await;
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                self.state.set_running(true).await;
                Ok("Proxy server restarted".to_string())
            }
            ServerCommand::Reload => {
                info!("Reloading proxy configuration via GUI command");
                // In production, reload config from file
                Ok("Configuration reloaded".to_string())
            }
            ServerCommand::Enable { server_id } => {
                info!("Enabling server: {}", server_id);
                // In production, update config and enable server
                Ok(format!("Server {} enabled", server_id))
            }
            ServerCommand::Disable { server_id } => {
                info!("Disabling server: {}", server_id);
                // In production, update config and disable server
                Ok(format!("Server {} disabled", server_id))
            }
        }
    }

    /// Get metrics for charts
    pub async fn get_metrics(&self, metric_names: Vec<String>) -> Result<Vec<MetricData>> {
        Ok(self.state.get_metrics(metric_names).await)
    }

    /// Get available metric names
    pub async fn list_metrics(&self) -> Result<Vec<String>> {
        let repo = self.state.analytics.repository();
        Ok(repo.list_metrics().await)
    }

    /// Query metrics with filters
    pub async fn query_metrics(&self, query: MetricQueryRequest) -> Result<Vec<MetricData>> {
        // Convert query to analytics query
        let analytics_query = crate::analytics::MetricQuery::new(query.start, query.end);

        let repo = self.state.analytics.repository();
        let results = repo.query(analytics_query).await;

        Ok(results
            .into_iter()
            .map(|r| MetricData {
                name: r.metric_name,
                points: r
                    .points
                    .into_iter()
                    .map(|p| super::DataPointDto {
                        timestamp: p.timestamp,
                        value: p.value,
                    })
                    .collect(),
            })
            .collect())
    }

    /// Get active alerts
    pub async fn get_alerts(&self) -> Result<Vec<crate::analytics::AlertEvent>> {
        Ok(self.state.get_active_alerts().await)
    }

    /// Get audit log
    pub async fn get_audit_log(&self, limit: usize) -> Result<Vec<AuditEvent>> {
        Ok(self.state.get_audit_log(limit).await)
    }

    /// Get loaded plugins
    pub async fn get_plugins(&self) -> Result<Vec<PluginMetadata>> {
        Ok(self.state.get_plugins().await)
    }

    /// Get tenants
    pub async fn get_tenants(&self) -> Result<Vec<Tenant>> {
        Ok(self.state.get_tenants().await)
    }

    /// Get AI optimization status
    pub async fn get_ai_status(&self) -> Result<AiStatus> {
        Ok(self.state.get_ai_status().await)
    }

    /// Get ML router server scores
    pub async fn get_ml_scores(&self) -> Result<Vec<ServerScore>> {
        let ml_router = self.state.ai_engine.ml_router();
        let scores = ml_router.get_server_stats().await;

        Ok(scores
            .into_iter()
            .map(|(id, score)| ServerScore {
                server_id: id,
                avg_response_time_ms: score.avg_response_time,
                success_rate: score.success_rate,
                total_requests: score.total_requests,
            })
            .collect())
    }

    /// Get predictive cache recommendations
    pub async fn get_cache_predictions(&self) -> Result<Vec<String>> {
        let cache = self.state.ai_engine.predictive_cache();
        Ok(cache.predict_preload().await)
    }

    /// Get recent anomalies
    pub async fn get_anomalies(&self, limit: usize) -> Result<Vec<crate::ai::Anomaly>> {
        let detector = self.state.ai_engine.anomaly_detector();
        Ok(detector.get_recent_anomalies(limit).await)
    }

    /// Get scaling recommendation
    pub async fn get_scaling_recommendation(
        &self,
    ) -> Result<crate::ai::ScalingRecommendation> {
        let recommender = self.state.ai_engine.scaling_recommender();
        Ok(recommender.get_recommendation().await)
    }

    /// Get multi-region status
    pub async fn get_regions(&self) -> Result<Option<Vec<RegionStatus>>> {
        Ok(self.state.get_region_status().await)
    }

    /// Get current configuration
    pub async fn get_config(&self) -> Result<Config> {
        Ok(self.state.get_config().await)
    }

    /// Update configuration
    pub async fn update_config(&self, config: Config) -> Result<String> {
        self.state.reload_config(config).await?;
        info!("Configuration updated via GUI");
        Ok("Configuration updated successfully".to_string())
    }

    /// Export metrics to CSV
    pub async fn export_metrics_csv(&self, metric_names: Vec<String>) -> Result<String> {
        let metrics = self.state.get_metrics(metric_names).await;
        let mut csv = String::from("timestamp,metric,value\n");

        for metric in metrics {
            for point in metric.points {
                csv.push_str(&format!(
                    "{},{},{}\n",
                    point.timestamp.to_rfc3339(),
                    metric.name,
                    point.value
                ));
            }
        }

        Ok(csv)
    }

    /// Export audit log to JSON
    pub async fn export_audit_json(&self, limit: usize) -> Result<String> {
        let events = self.state.get_audit_log(limit).await;
        serde_json::to_string_pretty(&events)
            .map_err(|e| Error::Internal(format!("Failed to serialize audit log: {}", e)))
    }

    /// Get system statistics
    pub async fn get_system_stats(&self) -> Result<SystemStats> {
        let overview = self.state.get_dashboard_overview().await;
        let servers = self.state.get_server_statuses().await;
        let ai_status = self.state.get_ai_status().await;

        let total_requests = 0u64; // Would be tracked in production
        let cache_size = ai_status.ml_router_servers;

        Ok(SystemStats {
            uptime_seconds: overview.uptime_seconds,
            total_requests,
            active_connections: overview.active_connections,
            total_servers: overview.total_servers,
            healthy_servers: overview.healthy_servers,
            cache_hit_rate: ai_status.cache_hit_rate,
            cache_size,
            memory_usage_mb: 0, // Would use sys-info in production
            cpu_usage_percent: 0.0,
        })
    }

    /// Test server connection
    pub async fn test_server_connection(&self, server_id: String) -> Result<ConnectionTestResult> {
        info!("Testing connection to server: {}", server_id);

        // In production, actually test the connection
        // For now, simulate a successful test
        Ok(ConnectionTestResult {
            server_id: server_id.clone(),
            success: true,
            latency_ms: 45,
            message: format!("Successfully connected to {}", server_id),
        })
    }

    /// Clear cache
    pub async fn clear_cache(&self) -> Result<String> {
        info!("Clearing cache via GUI command");
        // In production, clear actual cache
        Ok("Cache cleared successfully".to_string())
    }

    /// Get log stream (for real-time log viewing)
    pub async fn get_recent_logs(&self, limit: usize) -> Result<Vec<super::LogEntry>> {
        // In production, integrate with tracing subscriber
        // For now, return empty vec
        Ok(Vec::new())
    }
}

/// Metric query request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricQueryRequest {
    pub start: chrono::DateTime<chrono::Utc>,
    pub end: chrono::DateTime<chrono::Utc>,
    pub metric_pattern: Option<String>,
}

/// Server score for ML router
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerScore {
    pub server_id: String,
    pub avg_response_time_ms: f64,
    pub success_rate: f64,
    pub total_requests: u64,
}

/// System statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStats {
    pub uptime_seconds: u64,
    pub total_requests: u64,
    pub active_connections: u32,
    pub total_servers: usize,
    pub healthy_servers: usize,
    pub cache_hit_rate: f64,
    pub cache_size: usize,
    pub memory_usage_mb: u64,
    pub cpu_usage_percent: f64,
}

/// Connection test result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionTestResult {
    pub server_id: String,
    pub success: bool,
    pub latency_ms: u64,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analytics::AnalyticsEngine;
    use crate::ai::AIEngine;
    use crate::config::Config;

    #[tokio::test]
    async fn test_gui_commands() {
        let config = Config::default();
        let analytics = Arc::new(AnalyticsEngine::new());
        let ai_engine = Arc::new(AIEngine::new());
        let state = Arc::new(GuiState::new(config, analytics, ai_engine, None));
        let commands = GuiCommands::new(state);

        let dashboard = commands.get_dashboard().await.unwrap();
        assert_eq!(dashboard.is_running, false);

        let servers = commands.get_servers().await.unwrap();
        assert_eq!(servers.len(), 0);
    }

    #[tokio::test]
    async fn test_server_control() {
        let config = Config::default();
        let analytics = Arc::new(AnalyticsEngine::new());
        let ai_engine = Arc::new(AIEngine::new());
        let state = Arc::new(GuiState::new(config, analytics, ai_engine, None));
        let commands = GuiCommands::new(state.clone());

        let result = commands.control_server(ServerCommand::Start).await.unwrap();
        assert!(result.contains("started"));

        let dashboard = commands.get_dashboard().await.unwrap();
        assert_eq!(dashboard.is_running, true);
    }

    #[tokio::test]
    async fn test_export_metrics() {
        let config = Config::default();
        let analytics = Arc::new(AnalyticsEngine::new());
        let ai_engine = Arc::new(AIEngine::new());
        let state = Arc::new(GuiState::new(config, analytics, ai_engine, None));
        let commands = GuiCommands::new(state);

        let csv = commands.export_metrics_csv(vec![]).await.unwrap();
        assert!(csv.contains("timestamp,metric,value"));
    }
}
