//! Rate Limiter Plugin Example
//!
//! Demonstrates how to create a custom rate limiting plugin for Only1MCP

use async_trait::async_trait;
use only1mcp::plugins::*;
use only1mcp::types::*;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Custom rate limiter plugin
pub struct RateLimiterPlugin {
    metadata: PluginMetadata,
    state: PluginState,
    /// Track request counts per client
    client_requests: Arc<RwLock<HashMap<String, ClientRateLimit>>>,
    /// Requests per minute limit
    rpm_limit: u32,
}

#[derive(Clone)]
struct ClientRateLimit {
    count: u32,
    window_start: Instant,
}

impl RateLimiterPlugin {
    pub fn new(rpm_limit: u32) -> Self {
        Self {
            metadata: PluginMetadata {
                name: "rate-limiter".to_string(),
                version: "1.0.0".to_string(),
                author: "Only1MCP Team".to_string(),
                description: "Custom rate limiting plugin with per-client tracking".to_string(),
                capabilities: vec![PluginCapability::RequestTransform],
                dependencies: vec![],
            },
            state: PluginState::Loaded,
            client_requests: Arc::new(RwLock::new(HashMap::new())),
            rpm_limit,
        }
    }

    async fn check_rate_limit(&self, client_id: &str) -> Result<(), String> {
        let mut limits = self.client_requests.write().await;
        let now = Instant::now();

        let limit = limits.entry(client_id.to_string()).or_insert(ClientRateLimit {
            count: 0,
            window_start: now,
        });

        // Reset window if 1 minute has passed
        if now.duration_since(limit.window_start) >= Duration::from_secs(60) {
            limit.count = 0;
            limit.window_start = now;
        }

        // Check limit
        if limit.count >= self.rpm_limit {
            return Err(format!(
                "Rate limit exceeded: {} requests per minute",
                self.rpm_limit
            ));
        }

        // Increment counter
        limit.count += 1;
        Ok(())
    }
}

#[async_trait]
impl Plugin for RateLimiterPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    async fn initialize(
        &mut self,
        config: HashMap<String, serde_json::Value>,
    ) -> Result<(), String> {
        // Read custom config if provided
        if let Some(limit) = config.get("rpm_limit") {
            if let Some(limit_val) = limit.as_u64() {
                self.rpm_limit = limit_val as u32;
            }
        }

        self.state = PluginState::Ready;
        Ok(())
    }

    async fn start(&mut self) -> Result<(), String> {
        self.state = PluginState::Running;

        // Start background cleanup task
        let client_requests = self.client_requests.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(300)); // Every 5 minutes

            loop {
                interval.tick().await;

                // Clean up old entries
                let mut limits = client_requests.write().await;
                let now = Instant::now();
                limits.retain(|_, limit| {
                    now.duration_since(limit.window_start) < Duration::from_secs(120)
                });
            }
        });

        Ok(())
    }

    async fn stop(&mut self) -> Result<(), String> {
        self.state = PluginState::Stopped;
        Ok(())
    }

    fn state(&self) -> PluginState {
        self.state
    }

    async fn health_check(&self) -> Result<PluginHealth, String> {
        let requests = self.client_requests.read().await;
        Ok(PluginHealth {
            healthy: self.state == PluginState::Running,
            message: format!("Tracking {} clients", requests.len()),
        })
    }
}

#[async_trait]
impl RequestTransformer for RateLimiterPlugin {
    async fn transform_request(
        &self,
        mut request: McpRequest,
        context: &PluginContext,
    ) -> Result<McpRequest, String> {
        // Extract client ID from context
        let client_id = context
            .metadata
            .get("client_id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        // Check rate limit
        self.check_rate_limit(client_id).await?;

        // Add rate limit headers to request
        request.headers.insert(
            "X-RateLimit-Limit".to_string(),
            self.rpm_limit.to_string(),
        );

        Ok(request)
    }
}

// Example usage
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiter_plugin() {
        let mut plugin = RateLimiterPlugin::new(10);

        // Initialize
        let config = HashMap::new();
        plugin.initialize(config).await.unwrap();
        plugin.start().await.unwrap();

        // Create test request
        let request = McpRequest {
            method: "test".to_string(),
            params: serde_json::Value::Null,
            headers: HashMap::new(),
        };

        let context = PluginContext {
            metadata: {
                let mut map = HashMap::new();
                map.insert(
                    "client_id".to_string(),
                    serde_json::Value::String("test-client".to_string()),
                );
                map
            },
        };

        // Should succeed for first 10 requests
        for _ in 0..10 {
            assert!(plugin
                .transform_request(request.clone(), &context)
                .await
                .is_ok());
        }

        // 11th request should fail
        assert!(plugin
            .transform_request(request.clone(), &context)
            .await
            .is_err());
    }
}
