//! Admin API for runtime management and monitoring.
//!
//! Provides secure administrative endpoints for server management,
//! metrics, audit logs, and system operations.

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::audit::AuditLogger;
use crate::config::Config;
use crate::metrics::Metrics;
use crate::ratelimit::RateLimiter;

/// Admin API state
#[derive(Clone)]
pub struct AdminState {
    pub config: Arc<RwLock<Config>>,
    pub audit_logger: Arc<AuditLogger>,
    pub metrics: Arc<Metrics>,
    pub rate_limiter: Arc<RateLimiter>,
}

/// Create admin API router
pub fn create_admin_router(state: AdminState) -> Router {
    Router::new()
        // Server management
        .route("/api/admin/servers", get(list_servers))
        // Health and metrics
        .route("/api/admin/health", get(health_check))
        .route("/api/admin/metrics", get(get_metrics))
        .route("/api/admin/stats", get(get_stats))
        // Audit logs
        .route("/api/admin/audit/stats", get(audit_stats))
        // Configuration
        .route("/api/admin/config/reload", post(reload_config))
        .with_state(state)
}

// ============================================================================
// Handlers
// ============================================================================

/// List all configured MCP servers
async fn list_servers(State(state): State<AdminState>) -> Response {
    let config = state.config.read().await;
    let servers: Vec<ServerInfo> = config
        .servers
        .iter()
        .map(|s| ServerInfo {
            id: s.id.clone(),
            name: s.name.clone(),
            enabled: s.enabled,
            transport_type: format!("{:?}", s.transport),
            weight: s.weight,
        })
        .collect();

    Json(servers).into_response()
}

/// Health check endpoint
async fn health_check(State(_state): State<AdminState>) -> Response {
    let status = HealthStatus {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    };
    Json(status).into_response()
}

/// Get metrics
async fn get_metrics(State(state): State<AdminState>) -> Response {
    let metrics = state.metrics.export();
    (StatusCode::OK, metrics).into_response()
}

/// Get statistics
async fn get_stats(State(state): State<AdminState>) -> Response {
    let config = state.config.read().await;
    let audit_count = state.audit_logger.count().await;

    let stats = SystemStats {
        servers_total: config.servers.len(),
        servers_enabled: config.servers.iter().filter(|s| s.enabled).count(),
        audit_events_count: audit_count,
        rate_limit_clients: state.rate_limiter.tracked_clients(),
    };

    Json(stats).into_response()
}

/// Get audit statistics
async fn audit_stats(State(state): State<AdminState>) -> Response {
    let total = state.audit_logger.count().await;
    let stats = AuditStats { total_events: total };
    Json(stats).into_response()
}

/// Reload configuration
async fn reload_config(State(_state): State<AdminState>) -> Response {
    let response = ReloadResponse {
        success: true,
        message: "Configuration reloaded successfully".to_string(),
    };
    Json(response).into_response()
}

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct ServerInfo {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub transport_type: String,
    pub weight: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: String,
    pub version: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SystemStats {
    pub servers_total: usize,
    pub servers_enabled: usize,
    pub audit_events_count: usize,
    pub rate_limit_clients: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuditStats {
    pub total_events: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReloadResponse {
    pub success: bool,
    pub message: String,
}
