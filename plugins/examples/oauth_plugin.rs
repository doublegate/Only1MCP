//! OAuth Authentication Plugin Example
//!
//! Demonstrates how to create a custom OAuth2 authentication plugin for Only1MCP

use async_trait::async_trait;
use only1mcp::plugins::*;
use only1mcp::types::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

/// OAuth configuration
#[derive(Clone, Debug, Deserialize)]
pub struct OAuthConfig {
    pub client_id: String,
    pub client_secret: String,
    pub authorization_url: String,
    pub token_url: String,
    pub redirect_url: String,
    pub scopes: Vec<String>,
}

/// OAuth token
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OAuthToken {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub token_type: String,
    pub expires_in: u64,
    pub scope: Option<String>,
    #[serde(skip)]
    pub created_at: u64,
}

impl OAuthToken {
    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        now >= self.created_at + self.expires_in
    }

    pub fn time_until_expiry(&self) -> Duration {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let expiry = self.created_at + self.expires_in;
        if expiry > now {
            Duration::from_secs(expiry - now)
        } else {
            Duration::from_secs(0)
        }
    }
}

/// OAuth authentication plugin
pub struct OAuthPlugin {
    metadata: PluginMetadata,
    state: PluginState,
    config: Option<OAuthConfig>,
    /// Store tokens per user
    tokens: Arc<RwLock<HashMap<String, OAuthToken>>>,
    /// HTTP client for OAuth requests
    http_client: reqwest::Client,
}

impl OAuthPlugin {
    pub fn new() -> Self {
        Self {
            metadata: PluginMetadata {
                name: "oauth-authenticator".to_string(),
                version: "1.0.0".to_string(),
                author: "Only1MCP Team".to_string(),
                description: "OAuth2 authentication provider plugin".to_string(),
                capabilities: vec![
                    PluginCapability::RequestTransform,
                    PluginCapability::ResponseTransform,
                ],
                dependencies: vec![],
            },
            state: PluginState::Loaded,
            config: None,
            tokens: Arc::new(RwLock::new(HashMap::new())),
            http_client: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap(),
        }
    }

    /// Generate authorization URL
    pub fn get_authorization_url(&self, state: &str) -> Result<String, String> {
        let config = self
            .config
            .as_ref()
            .ok_or_else(|| "OAuth not configured".to_string())?;

        let mut url = url::Url::parse(&config.authorization_url)
            .map_err(|e| format!("Invalid authorization URL: {}", e))?;

        url.query_pairs_mut()
            .append_pair("client_id", &config.client_id)
            .append_pair("redirect_uri", &config.redirect_url)
            .append_pair("response_type", "code")
            .append_pair("scope", &config.scopes.join(" "))
            .append_pair("state", state);

        Ok(url.to_string())
    }

    /// Exchange authorization code for access token
    pub async fn exchange_code(&self, code: &str) -> Result<OAuthToken, String> {
        let config = self
            .config
            .as_ref()
            .ok_or_else(|| "OAuth not configured".to_string())?;

        let params = [
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", &config.redirect_url),
            ("client_id", &config.client_id),
            ("client_secret", &config.client_secret),
        ];

        let response = self
            .http_client
            .post(&config.token_url)
            .form(&params)
            .send()
            .await
            .map_err(|e| format!("Failed to exchange code: {}", e))?;

        if !response.status().is_success() {
            return Err(format!(
                "Token exchange failed: {}",
                response.status()
            ));
        }

        let mut token: OAuthToken = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse token response: {}", e))?;

        token.created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Ok(token)
    }

    /// Refresh an expired access token
    pub async fn refresh_token(&self, refresh_token: &str) -> Result<OAuthToken, String> {
        let config = self
            .config
            .as_ref()
            .ok_or_else(|| "OAuth not configured".to_string())?;

        let params = [
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", &config.client_id),
            ("client_secret", &config.client_secret),
        ];

        let response = self
            .http_client
            .post(&config.token_url)
            .form(&params)
            .send()
            .await
            .map_err(|e| format!("Failed to refresh token: {}", e))?;

        if !response.status().is_success() {
            return Err(format!(
                "Token refresh failed: {}",
                response.status()
            ));
        }

        let mut token: OAuthToken = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse token response: {}", e))?;

        token.created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Ok(token)
    }

    /// Validate and possibly refresh token
    async fn ensure_valid_token(&self, user_id: &str) -> Result<OAuthToken, String> {
        let tokens = self.tokens.read().await;
        let token = tokens
            .get(user_id)
            .ok_or_else(|| "No token found for user".to_string())?;

        if !token.is_expired() {
            return Ok(token.clone());
        }

        // Token expired, try to refresh
        drop(tokens); // Release read lock

        let refresh_token = token
            .refresh_token
            .as_ref()
            .ok_or_else(|| "No refresh token available".to_string())?;

        let new_token = self.refresh_token(refresh_token).await?;

        // Store new token
        let mut tokens = self.tokens.write().await;
        tokens.insert(user_id.to_string(), new_token.clone());

        Ok(new_token)
    }
}

impl Default for OAuthPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Plugin for OAuthPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    async fn initialize(
        &mut self,
        config: HashMap<String, serde_json::Value>,
    ) -> Result<(), String> {
        // Parse OAuth configuration
        let oauth_config: OAuthConfig = serde_json::from_value(
            config
                .get("oauth")
                .ok_or_else(|| "Missing 'oauth' configuration".to_string())?
                .clone(),
        )
        .map_err(|e| format!("Invalid OAuth config: {}", e))?;

        self.config = Some(oauth_config);
        self.state = PluginState::Ready;
        Ok(())
    }

    async fn start(&mut self) -> Result<(), String> {
        self.state = PluginState::Running;

        // Start background token refresh task
        let tokens = self.tokens.clone();
        let http_client = self.http_client.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(300)); // Every 5 minutes

            loop {
                interval.tick().await;

                let mut tokens_guard = tokens.write().await;
                let mut to_refresh = Vec::new();

                // Find tokens that will expire soon (within 10 minutes)
                for (user_id, token) in tokens_guard.iter() {
                    if token.time_until_expiry() < Duration::from_secs(600) {
                        if let Some(refresh_token) = &token.refresh_token {
                            to_refresh.push((user_id.clone(), refresh_token.clone()));
                        }
                    }
                }

                drop(tokens_guard);

                // Refresh tokens
                for (user_id, refresh_token) in to_refresh {
                    if let Some(cfg) = &config {
                        let params = [
                            ("grant_type", "refresh_token"),
                            ("refresh_token", refresh_token.as_str()),
                            ("client_id", cfg.client_id.as_str()),
                            ("client_secret", cfg.client_secret.as_str()),
                        ];

                        if let Ok(response) = http_client.post(&cfg.token_url).form(&params).send().await {
                            if let Ok(mut new_token) = response.json::<OAuthToken>().await {
                                new_token.created_at = SystemTime::now()
                                    .duration_since(UNIX_EPOCH)
                                    .unwrap()
                                    .as_secs();

                                let mut tokens_guard = tokens.write().await;
                                tokens_guard.insert(user_id, new_token);
                            }
                        }
                    }
                }
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
        let tokens = self.tokens.read().await;
        Ok(PluginHealth {
            healthy: self.state == PluginState::Running,
            message: format!(
                "Managing {} active tokens",
                tokens.len()
            ),
        })
    }
}

#[async_trait]
impl RequestTransformer for OAuthPlugin {
    async fn transform_request(
        &self,
        mut request: McpRequest,
        context: &PluginContext,
    ) -> Result<McpRequest, String> {
        // Extract user ID from context
        let user_id = context
            .metadata
            .get("user_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing user_id in context".to_string())?;

        // Get valid token
        let token = self.ensure_valid_token(user_id).await?;

        // Add OAuth token to request headers
        request.headers.insert(
            "Authorization".to_string(),
            format!("{} {}", token.token_type, token.access_token),
        );

        Ok(request)
    }
}

#[async_trait]
impl ResponseTransformer for OAuthPlugin {
    async fn transform_response(
        &self,
        response: McpResponse,
        _context: &PluginContext,
    ) -> Result<McpResponse, String> {
        // Could add response processing here if needed
        Ok(response)
    }
}

// Example usage
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_oauth_plugin() {
        let mut plugin = OAuthPlugin::new();

        // Initialize with config
        let mut config = HashMap::new();
        config.insert(
            "oauth".to_string(),
            serde_json::json!({
                "client_id": "test_client",
                "client_secret": "test_secret",
                "authorization_url": "https://provider.com/oauth/authorize",
                "token_url": "https://provider.com/oauth/token",
                "redirect_url": "http://localhost:8080/callback",
                "scopes": ["read", "write"]
            }),
        );

        plugin.initialize(config).await.unwrap();
        assert_eq!(plugin.state(), PluginState::Ready);

        // Test authorization URL generation
        let auth_url = plugin.get_authorization_url("random_state").unwrap();
        assert!(auth_url.contains("client_id=test_client"));
        assert!(auth_url.contains("scope=read%20write"));
    }

    #[test]
    fn test_token_expiry() {
        let token = OAuthToken {
            access_token: "test".to_string(),
            refresh_token: None,
            token_type: "Bearer".to_string(),
            expires_in: 3600,
            scope: None,
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                - 3700, // Created 3700 seconds ago
        };

        assert!(token.is_expired());
    }
}
