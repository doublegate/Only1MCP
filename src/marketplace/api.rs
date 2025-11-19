/// Plugin Marketplace REST API

use crate::error::{Error, Result};
use crate::marketplace::{db, storage, types::*, validator::PluginValidator};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use std::sync::Arc;

/// Plugin Registry API state
pub struct PluginRegistryApi {
    // Future: Add authentication, rate limiting, etc.
}

impl PluginRegistryApi {
    pub fn new() -> Self {
        Self {}
    }

    /// Create API router
    pub fn router() -> Router {
        Router::new()
            .route("/api/plugins", get(list_plugins).post(publish_plugin))
            .route("/api/plugins/:name", get(get_plugin))
            .route("/api/plugins/:name/:version", get(get_plugin_version))
            .route("/api/plugins/:name/:version/download", get(download_plugin))
            .route("/api/plugins/:name/rate", post(rate_plugin))
            .with_state(Arc::new(PluginRegistryApi::new()))
    }
}

/// List all plugins (with search/filter)
async fn list_plugins(
    Query(query): Query<PluginSearchQuery>,
) -> Result<Json<PluginListResponse>, ApiError> {
    let result = db::search_plugins(&query).await?;
    Ok(Json(result))
}

/// Get plugin details
async fn get_plugin(Path(name): Path<String>) -> Result<Json<Plugin>, ApiError> {
    let plugin = db::get_plugin_by_name(&name)
        .await?
        .ok_or_else(|| Error::Config(format!("Plugin '{}' not found", name)))?;

    Ok(Json(plugin))
}

/// Get specific plugin version
async fn get_plugin_version(
    Path((name, version)): Path<(String, String)>,
) -> Result<Json<PluginVersion>, ApiError> {
    // TODO: Implement get_plugin_version in db module
    Err(ApiError(Error::Config("Not yet implemented".to_string())))
}

/// Download plugin tarball
async fn download_plugin(
    Path((name, version)): Path<(String, String)>,
) -> Result<Vec<u8>, ApiError> {
    let tarball = storage::download_plugin(&name, &version).await?;

    // Increment download counter
    if let Some(plugin) = db::get_plugin_by_name(&name).await? {
        let _ = db::increment_downloads(plugin.id).await;
    }

    Ok(tarball)
}

/// Publish a new plugin
async fn publish_plugin(
    Json(request): Json<PublishRequest>,
) -> Result<Json<PublishResponse>, ApiError> {
    // 1. Validate manifest
    PluginValidator::validate_manifest(&request.manifest)?;

    // 2. Decode and validate tarball
    let tarball_data = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        &request.tarball_base64,
    )
    .map_err(|e| Error::Config(format!("Invalid base64 tarball: {}", e)))?;

    // 3. Security scan
    PluginValidator::security_scan(&tarball_data).await?;

    // 4. Upload to storage
    let (tarball_url, checksum) =
        storage::upload_plugin(&request.name, &request.version, &request.tarball_base64).await?;

    // 5. Insert into database
    let plugin_id = match db::get_plugin_by_name(&request.name).await? {
        Some(existing) => {
            // Update existing plugin
            existing.id
        }
        None => {
            // Create new plugin
            db::insert_plugin(&request.manifest).await?
        }
    };

    // 6. Insert version
    let version_id = db::insert_plugin_version(
        plugin_id,
        &request.version,
        &tarball_url,
        &checksum,
        &request.manifest,
    )
    .await?;

    tracing::info!(
        "Published plugin {} v{} (plugin_id={}, version_id={})",
        request.name,
        request.version,
        plugin_id,
        version_id
    );

    Ok(Json(PublishResponse {
        success: true,
        plugin_id,
        version_id,
        tarball_url,
        checksum,
    }))
}

/// Rate a plugin
async fn rate_plugin(
    Path(name): Path<String>,
    Json(rating): Json<PluginRating>,
) -> Result<StatusCode, ApiError> {
    // Validate rating
    if !(1..=5).contains(&rating.rating) {
        return Err(ApiError(Error::Config(
            "Rating must be between 1 and 5".to_string(),
        )));
    }

    // Get plugin
    let plugin = db::get_plugin_by_name(&name)
        .await?
        .ok_or_else(|| Error::Config(format!("Plugin '{}' not found", name)))?;

    // TODO: Get user_id from authentication
    let user_id = "anonymous";

    // Upsert rating
    db::upsert_rating(
        plugin.id,
        user_id,
        rating.rating,
        rating.review.as_deref(),
    )
    .await?;

    Ok(StatusCode::OK)
}

/// Publish response
#[derive(serde::Serialize)]
struct PublishResponse {
    success: bool,
    plugin_id: i64,
    version_id: i64,
    tarball_url: String,
    checksum: String,
}

/// API error wrapper for proper HTTP responses
struct ApiError(Error);

impl From<Error> for ApiError {
    fn from(err: Error) -> Self {
        ApiError(err)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self.0 {
            Error::Config(msg) => (StatusCode::BAD_REQUEST, msg),
            Error::Transport(msg) => (StatusCode::SERVICE_UNAVAILABLE, msg),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string()),
        };

        let body = serde_json::json!({
            "error": error_message
        });

        (status, Json(body)).into_response()
    }
}
