//! Multi-tenancy support for Only1MCP
//!
//! Provides tenant isolation, per-tenant configuration, and access control.
//! Each tenant gets isolated resources, rate limits, and metrics.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Tenant identifier
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct TenantId(pub String);

impl TenantId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for TenantId {
    fn from(s: String) -> Self {
        TenantId(s)
    }
}

impl From<&str> for TenantId {
    fn from(s: &str) -> Self {
        TenantId(s.to_string())
    }
}

/// Tenant configuration and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tenant {
    /// Unique tenant identifier
    pub id: TenantId,

    /// Tenant display name
    pub name: String,

    /// Tenant tier (free, pro, enterprise)
    pub tier: TenantTier,

    /// Enabled status
    pub enabled: bool,

    /// Rate limit overrides for this tenant
    pub rate_limit: Option<TenantRateLimit>,

    /// Allowed MCP servers for this tenant
    pub allowed_servers: Vec<String>,

    /// Custom configuration
    pub config: HashMap<String, serde_json::Value>,

    /// Metadata tags
    pub tags: HashMap<String, String>,
}

/// Tenant tier with different resource limits
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TenantTier {
    /// Free tier with basic limits
    Free,
    /// Pro tier with higher limits
    Pro,
    /// Enterprise tier with custom limits
    Enterprise,
}

impl TenantTier {
    /// Get default rate limit for this tier
    pub fn default_rate_limit(&self) -> TenantRateLimit {
        match self {
            TenantTier::Free => TenantRateLimit {
                requests_per_minute: 60,
                requests_per_hour: 1000,
                requests_per_day: 10000,
                burst_size: 10,
            },
            TenantTier::Pro => TenantRateLimit {
                requests_per_minute: 300,
                requests_per_hour: 10000,
                requests_per_day: 100000,
                burst_size: 50,
            },
            TenantTier::Enterprise => TenantRateLimit {
                requests_per_minute: 1000,
                requests_per_hour: 50000,
                requests_per_day: 1000000,
                burst_size: 200,
            },
        }
    }
}

/// Per-tenant rate limits
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TenantRateLimit {
    pub requests_per_minute: u32,
    pub requests_per_hour: u32,
    pub requests_per_day: u32,
    pub burst_size: u32,
}

/// Tenant context attached to requests
#[derive(Debug, Clone)]
pub struct TenantContext {
    pub tenant_id: TenantId,
    pub tier: TenantTier,
    pub metadata: HashMap<String, String>,
}

impl TenantContext {
    pub fn new(tenant_id: TenantId, tier: TenantTier) -> Self {
        Self {
            tenant_id,
            tier,
            metadata: HashMap::new(),
        }
    }

    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

/// Tenant registry for managing tenants
pub struct TenantRegistry {
    tenants: Arc<RwLock<HashMap<TenantId, Tenant>>>,
}

impl TenantRegistry {
    pub fn new() -> Self {
        Self {
            tenants: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a new tenant
    pub async fn register(&self, tenant: Tenant) -> Result<(), TenancyError> {
        let mut tenants = self.tenants.write().await;

        if tenants.contains_key(&tenant.id) {
            return Err(TenancyError::TenantAlreadyExists(tenant.id.clone()));
        }

        tenants.insert(tenant.id.clone(), tenant);
        Ok(())
    }

    /// Get tenant by ID
    pub async fn get(&self, id: &TenantId) -> Result<Tenant, TenancyError> {
        let tenants = self.tenants.read().await;
        tenants
            .get(id)
            .cloned()
            .ok_or_else(|| TenancyError::TenantNotFound(id.clone()))
    }

    /// Update tenant
    pub async fn update(&self, tenant: Tenant) -> Result<(), TenancyError> {
        let mut tenants = self.tenants.write().await;

        if !tenants.contains_key(&tenant.id) {
            return Err(TenancyError::TenantNotFound(tenant.id.clone()));
        }

        tenants.insert(tenant.id.clone(), tenant);
        Ok(())
    }

    /// Delete tenant
    pub async fn delete(&self, id: &TenantId) -> Result<(), TenancyError> {
        let mut tenants = self.tenants.write().await;
        tenants
            .remove(id)
            .ok_or_else(|| TenancyError::TenantNotFound(id.clone()))?;
        Ok(())
    }

    /// List all tenants
    pub async fn list(&self) -> Vec<Tenant> {
        let tenants = self.tenants.read().await;
        tenants.values().cloned().collect()
    }

    /// Check if tenant has access to server
    pub async fn can_access_server(
        &self,
        tenant_id: &TenantId,
        server_id: &str,
    ) -> Result<bool, TenancyError> {
        let tenant = self.get(tenant_id).await?;

        if !tenant.enabled {
            return Ok(false);
        }

        // Empty allowed_servers means access to all
        if tenant.allowed_servers.is_empty() {
            return Ok(true);
        }

        Ok(tenant.allowed_servers.contains(&server_id.to_string()))
    }

    /// Get effective rate limit for tenant
    pub async fn get_rate_limit(&self, tenant_id: &TenantId) -> Result<TenantRateLimit, TenancyError> {
        let tenant = self.get(tenant_id).await?;
        Ok(tenant.rate_limit.unwrap_or_else(|| tenant.tier.default_rate_limit()))
    }
}

impl Default for TenantRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Tenancy errors
#[derive(Debug, thiserror::Error)]
pub enum TenancyError {
    #[error("Tenant not found: {0:?}")]
    TenantNotFound(TenantId),

    #[error("Tenant already exists: {0:?}")]
    TenantAlreadyExists(TenantId),

    #[error("Tenant disabled: {0:?}")]
    TenantDisabled(TenantId),

    #[error("Access denied for tenant {0:?} to server {1}")]
    AccessDenied(TenantId, String),

    #[error("Rate limit exceeded for tenant {0:?}")]
    RateLimitExceeded(TenantId),
}

/// Extract tenant ID from request headers
///
/// Supports multiple header formats:
/// - X-Tenant-ID: tenant_id
/// - Authorization: Bearer <token> (decode to get tenant)
/// - API-Key: key (lookup to get tenant)
pub fn extract_tenant_id_from_headers(
    headers: &axum::http::HeaderMap,
) -> Option<TenantId> {
    // Try X-Tenant-ID header first
    if let Some(tenant_header) = headers.get("x-tenant-id") {
        if let Ok(tenant_str) = tenant_header.to_str() {
            return Some(TenantId::new(tenant_str));
        }
    }

    // Could extract from JWT claims if Authorization header present
    // For now, return None if no explicit tenant ID

    None
}

/// Tenant middleware for extracting and validating tenant context
pub async fn tenant_middleware(
    tenant_registry: Arc<TenantRegistry>,
    mut request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Result<axum::response::Response, (axum::http::StatusCode, String)> {
    use axum::http::StatusCode;

    // Extract tenant ID from headers
    let tenant_id = extract_tenant_id_from_headers(request.headers())
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                "Missing X-Tenant-ID header".to_string(),
            )
        })?;

    // Validate tenant exists and is enabled
    let tenant = tenant_registry
        .get(&tenant_id)
        .await
        .map_err(|e| (StatusCode::UNAUTHORIZED, e.to_string()))?;

    if !tenant.enabled {
        return Err((
            StatusCode::FORBIDDEN,
            format!("Tenant {} is disabled", tenant_id.as_str()),
        ));
    }

    // Create tenant context and attach to request
    let context = TenantContext::new(tenant_id, tenant.tier);
    request.extensions_mut().insert(context);

    // Continue to next middleware
    Ok(next.run(request).await)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tenant_registry() {
        let registry = TenantRegistry::new();

        let tenant = Tenant {
            id: TenantId::new("test-tenant"),
            name: "Test Tenant".to_string(),
            tier: TenantTier::Pro,
            enabled: true,
            rate_limit: None,
            allowed_servers: vec!["server1".to_string(), "server2".to_string()],
            config: HashMap::new(),
            tags: HashMap::new(),
        };

        // Register tenant
        registry.register(tenant.clone()).await.unwrap();

        // Get tenant
        let retrieved = registry.get(&TenantId::new("test-tenant")).await.unwrap();
        assert_eq!(retrieved.name, "Test Tenant");
        assert_eq!(retrieved.tier, TenantTier::Pro);

        // Check server access
        assert!(registry
            .can_access_server(&TenantId::new("test-tenant"), "server1")
            .await
            .unwrap());
        assert!(!registry
            .can_access_server(&TenantId::new("test-tenant"), "server3")
            .await
            .unwrap());
    }

    #[test]
    fn test_tenant_tier_rate_limits() {
        let free_limit = TenantTier::Free.default_rate_limit();
        assert_eq!(free_limit.requests_per_minute, 60);

        let pro_limit = TenantTier::Pro.default_rate_limit();
        assert_eq!(pro_limit.requests_per_minute, 300);

        let enterprise_limit = TenantTier::Enterprise.default_rate_limit();
        assert_eq!(enterprise_limit.requests_per_minute, 1000);
    }
}
