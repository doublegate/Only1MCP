//! HTTP API for GUI
//!
//! Provides RESTful HTTP endpoints for the GUI backend.
//! Can be used by web dashboards, Tauri apps, or any HTTP client.

use axum::{
    extract::{Path, Query, State, WebSocketUpgrade},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, error};

use super::commands::{GuiCommands, MetricQueryRequest};
use super::websocket::{GuiEventBroadcaster, WsConnection, WsMessage};
use super::ServerCommand;
use crate::error::Result;

/// GUI API Router
pub fn gui_router(commands: Arc<GuiCommands>, broadcaster: GuiEventBroadcaster) -> Router {
    Router::new()
        // Dashboard
        .route("/api/gui/dashboard", get(get_dashboard))
        .route("/api/gui/servers", get(get_servers))
        .route("/api/gui/server/:id/health", get(get_server_health))
        .route("/api/gui/control", post(control_server))
        // Metrics
        .route("/api/gui/metrics", post(get_metrics))
        .route("/api/gui/metrics/list", get(list_metrics))
        .route("/api/gui/metrics/query", post(query_metrics))
        .route("/api/gui/metrics/export/csv", post(export_metrics_csv))
        // Alerts
        .route("/api/gui/alerts", get(get_alerts))
        // Audit
        .route("/api/gui/audit", get(get_audit))
        .route("/api/gui/audit/export", get(export_audit))
        // Plugins
        .route("/api/gui/plugins", get(get_plugins))
        // Tenants
        .route("/api/gui/tenants", get(get_tenants))
        // AI
        .route("/api/gui/ai/status", get(get_ai_status))
        .route("/api/gui/ai/ml-scores", get(get_ml_scores))
        .route("/api/gui/ai/cache-predictions", get(get_cache_predictions))
        .route("/api/gui/ai/anomalies", get(get_anomalies))
        .route("/api/gui/ai/scaling", get(get_scaling))
        // Multi-region
        .route("/api/gui/regions", get(get_regions))
        // Configuration
        .route("/api/gui/config", get(get_config))
        .route("/api/gui/config", post(update_config))
        // System
        .route("/api/gui/system/stats", get(get_system_stats))
        .route("/api/gui/system/test/:id", post(test_connection))
        .route("/api/gui/system/clear-cache", post(clear_cache))
        .route("/api/gui/logs", get(get_logs))
        // WebSocket
        .route("/api/gui/ws", get(websocket_handler))
        .with_state(GuiApiState { commands, broadcaster })
}

/// GUI API state
#[derive(Clone)]
struct GuiApiState {
    commands: Arc<GuiCommands>,
    broadcaster: GuiEventBroadcaster,
}

/// Dashboard overview handler
async fn get_dashboard(State(state): State<GuiApiState>) -> Result<Json<serde_json::Value>, ApiError> {
    let dashboard = state.commands.get_dashboard().await?;
    Ok(Json(serde_json::to_value(dashboard)?))
}

/// Get servers handler
async fn get_servers(State(state): State<GuiApiState>) -> Result<Json<serde_json::Value>, ApiError> {
    let servers = state.commands.get_servers().await?;
    Ok(Json(serde_json::to_value(servers)?))
}

/// Get server health handler
async fn get_server_health(
    State(state): State<GuiApiState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let health = state.commands.get_server_health(id).await?;
    Ok(Json(serde_json::to_value(health)?))
}

/// Server control handler
async fn control_server(
    State(state): State<GuiApiState>,
    Json(command): Json<ServerCommand>,
) -> Result<Json<ApiResponse>, ApiError> {
    let message = state.commands.control_server(command).await?;
    Ok(Json(ApiResponse::success(message)))
}

/// Get metrics handler
async fn get_metrics(
    State(state): State<GuiApiState>,
    Json(req): Json<MetricNamesRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let metrics = state.commands.get_metrics(req.names).await?;
    Ok(Json(serde_json::to_value(metrics)?))
}

/// List metrics handler
async fn list_metrics(State(state): State<GuiApiState>) -> Result<Json<Vec<String>>, ApiError> {
    let names = state.commands.list_metrics().await?;
    Ok(Json(names))
}

/// Query metrics handler
async fn query_metrics(
    State(state): State<GuiApiState>,
    Json(query): Json<MetricQueryRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let results = state.commands.query_metrics(query).await?;
    Ok(Json(serde_json::to_value(results)?))
}

/// Export metrics CSV handler
async fn export_metrics_csv(
    State(state): State<GuiApiState>,
    Json(req): Json<MetricNamesRequest>,
) -> Result<String, ApiError> {
    let csv = state.commands.export_metrics_csv(req.names).await?;
    Ok(csv)
}

/// Get alerts handler
async fn get_alerts(State(state): State<GuiApiState>) -> Result<Json<serde_json::Value>, ApiError> {
    let alerts = state.commands.get_alerts().await?;
    Ok(Json(serde_json::to_value(alerts)?))
}

/// Get audit log handler
#[derive(Deserialize)]
struct AuditQueryParams {
    #[serde(default = "default_limit")]
    limit: usize,
}

fn default_limit() -> usize {
    100
}

async fn get_audit(
    State(state): State<GuiApiState>,
    Query(params): Query<AuditQueryParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let events = state.commands.get_audit_log(params.limit).await?;
    Ok(Json(serde_json::to_value(events)?))
}

/// Export audit log handler
async fn export_audit(
    State(state): State<GuiApiState>,
    Query(params): Query<AuditQueryParams>,
) -> Result<String, ApiError> {
    let json = state.commands.export_audit_json(params.limit).await?;
    Ok(json)
}

/// Get plugins handler
async fn get_plugins(State(state): State<GuiApiState>) -> Result<Json<serde_json::Value>, ApiError> {
    let plugins = state.commands.get_plugins().await?;
    Ok(Json(serde_json::to_value(plugins)?))
}

/// Get tenants handler
async fn get_tenants(State(state): State<GuiApiState>) -> Result<Json<serde_json::Value>, ApiError> {
    let tenants = state.commands.get_tenants().await?;
    Ok(Json(serde_json::to_value(tenants)?))
}

/// Get AI status handler
async fn get_ai_status(State(state): State<GuiApiState>) -> Result<Json<serde_json::Value>, ApiError> {
    let status = state.commands.get_ai_status().await?;
    Ok(Json(serde_json::to_value(status)?))
}

/// Get ML scores handler
async fn get_ml_scores(State(state): State<GuiApiState>) -> Result<Json<serde_json::Value>, ApiError> {
    let scores = state.commands.get_ml_scores().await?;
    Ok(Json(serde_json::to_value(scores)?))
}

/// Get cache predictions handler
async fn get_cache_predictions(State(state): State<GuiApiState>) -> Result<Json<Vec<String>>, ApiError> {
    let predictions = state.commands.get_cache_predictions().await?;
    Ok(Json(predictions))
}

/// Get anomalies handler
async fn get_anomalies(
    State(state): State<GuiApiState>,
    Query(params): Query<AuditQueryParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let anomalies = state.commands.get_anomalies(params.limit).await?;
    Ok(Json(serde_json::to_value(anomalies)?))
}

/// Get scaling recommendation handler
async fn get_scaling(State(state): State<GuiApiState>) -> Result<Json<serde_json::Value>, ApiError> {
    let recommendation = state.commands.get_scaling_recommendation().await?;
    Ok(Json(serde_json::to_value(recommendation)?))
}

/// Get regions handler
async fn get_regions(State(state): State<GuiApiState>) -> Result<Json<serde_json::Value>, ApiError> {
    let regions = state.commands.get_regions().await?;
    Ok(Json(serde_json::to_value(regions)?))
}

/// Get config handler
async fn get_config(State(state): State<GuiApiState>) -> Result<Json<serde_json::Value>, ApiError> {
    let config = state.commands.get_config().await?;
    Ok(Json(serde_json::to_value(config)?))
}

/// Update config handler
async fn update_config(
    State(state): State<GuiApiState>,
    Json(config): Json<crate::config::Config>,
) -> Result<Json<ApiResponse>, ApiError> {
    let message = state.commands.update_config(config).await?;
    Ok(Json(ApiResponse::success(message)))
}

/// Get system stats handler
async fn get_system_stats(State(state): State<GuiApiState>) -> Result<Json<serde_json::Value>, ApiError> {
    let stats = state.commands.get_system_stats().await?;
    Ok(Json(serde_json::to_value(stats)?))
}

/// Test connection handler
async fn test_connection(
    State(state): State<GuiApiState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let result = state.commands.test_server_connection(id).await?;
    Ok(Json(serde_json::to_value(result)?))
}

/// Clear cache handler
async fn clear_cache(State(state): State<GuiApiState>) -> Result<Json<ApiResponse>, ApiError> {
    let message = state.commands.clear_cache().await?;
    Ok(Json(ApiResponse::success(message)))
}

/// Get logs handler
async fn get_logs(
    State(state): State<GuiApiState>,
    Query(params): Query<AuditQueryParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let logs = state.commands.get_recent_logs(params.limit).await?;
    Ok(Json(serde_json::to_value(logs)?))
}

/// WebSocket upgrade handler
async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<GuiApiState>,
) -> Response {
    ws.on_upgrade(move |socket| handle_websocket(socket, state))
}

/// WebSocket connection handler
async fn handle_websocket(
    socket: axum::extract::ws::WebSocket,
    state: GuiApiState,
) {
    use futures_util::{SinkExt, StreamExt};

    let conn_id = uuid::Uuid::new_v4().to_string();
    debug!("WebSocket connection established: {}", conn_id);

    let (mut sender, mut receiver) = socket.split();
    let rx = state.broadcaster.subscribe();
    let mut ws_conn = WsConnection::new(conn_id.clone(), rx);

    // Add subscription
    state.broadcaster.add_subscription(conn_id.clone()).await;

    // Spawn sender task
    let send_task = tokio::spawn(async move {
        while let Some(event) = ws_conn.recv().await {
            let message = WsMessage::Event { event };
            if let Ok(json) = serde_json::to_string(&message) {
                if sender
                    .send(axum::extract::ws::Message::Text(json))
                    .await
                    .is_err()
                {
                    break;
                }
            }
        }
    });

    // Handle incoming messages
    while let Some(Ok(msg)) = receiver.next().await {
        if let axum::extract::ws::Message::Text(text) = msg {
            // Handle client messages (subscribe, ping, etc.)
            debug!("Received WebSocket message: {}", text);
        }
    }

    // Cleanup
    send_task.abort();
    state.broadcaster.remove_subscription(&conn_id).await;
    debug!("WebSocket connection closed: {}", conn_id);
}

/// API request types
#[derive(Debug, Deserialize)]
struct MetricNamesRequest {
    names: Vec<String>,
}

/// API response wrapper
#[derive(Debug, Serialize)]
struct ApiResponse {
    success: bool,
    message: String,
}

impl ApiResponse {
    fn success(message: String) -> Self {
        Self {
            success: true,
            message,
        }
    }
}

/// API error wrapper
struct ApiError(crate::error::Error);

impl From<crate::error::Error> for ApiError {
    fn from(err: crate::error::Error) -> Self {
        ApiError(err)
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(err: serde_json::Error) -> Self {
        ApiError(crate::error::Error::Internal(err.to_string()))
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match &self.0 {
            crate::error::Error::NotFound(_) => StatusCode::NOT_FOUND,
            crate::error::Error::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            crate::error::Error::InvalidRequest(_) => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let body = Json(serde_json::json!({
            "error": self.0.to_string(),
        }));

        (status, body).into_response()
    }
}
