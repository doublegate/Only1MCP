use crate::error::{Error, Result};
use crate::types::{HealthStatus, ServerStatus, SystemInfo, ToolInfo};
use reqwest::Client;
use std::time::Duration;

/// HTTP client for communicating with Only1MCP daemon via Admin API
pub struct TuiClient {
    client: Client,
    base_url: String,
}

impl TuiClient {
    /// Create new TUI client for given host and port
    pub fn new(host: &str, port: u16) -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(5))
                .build()
                .expect("Failed to build HTTP client"),
            base_url: format!("http://{}:{}", host, port),
        }
    }

    /// Check if daemon is running and responding
    pub async fn is_running(&self) -> bool {
        self.get_health().await.is_ok()
    }

    /// GET /api/v1/admin/servers
    pub async fn get_servers(&self) -> Result<Vec<ServerStatus>> {
        let url = format!("{}/api/v1/admin/servers", self.base_url);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| Error::Transport(format!("Failed to fetch servers: {}", e)))?;

        if !response.status().is_success() {
            return Err(Error::Transport(format!(
                "HTTP {}: {}",
                response.status(),
                url
            )));
        }

        response
            .json()
            .await
            .map_err(|e| Error::Transport(format!("Failed to parse servers: {}", e)))
    }

    /// GET /api/v1/admin/tools
    pub async fn get_tools(&self) -> Result<Vec<ToolInfo>> {
        let url = format!("{}/api/v1/admin/tools", self.base_url);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| Error::Transport(format!("Failed to fetch tools: {}", e)))?;

        if !response.status().is_success() {
            return Err(Error::Transport(format!(
                "HTTP {}: {}",
                response.status(),
                url
            )));
        }

        response
            .json()
            .await
            .map_err(|e| Error::Transport(format!("Failed to parse tools: {}", e)))
    }

    /// GET /api/v1/admin/health
    pub async fn get_health(&self) -> Result<HealthStatus> {
        let url = format!("{}/api/v1/admin/health", self.base_url);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| Error::Transport(format!("Failed to fetch health: {}", e)))?;

        if !response.status().is_success() {
            return Err(Error::Transport(format!(
                "HTTP {}: {}",
                response.status(),
                url
            )));
        }

        response
            .json()
            .await
            .map_err(|e| Error::Transport(format!("Failed to parse health: {}", e)))
    }

    /// GET /api/v1/admin/system
    pub async fn get_system_info(&self) -> Result<SystemInfo> {
        let url = format!("{}/api/v1/admin/system", self.base_url);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| Error::Transport(format!("Failed to fetch system info: {}", e)))?;

        if !response.status().is_success() {
            return Err(Error::Transport(format!(
                "HTTP {}: {}",
                response.status(),
                url
            )));
        }

        response
            .json()
            .await
            .map_err(|e| Error::Transport(format!("Failed to parse system info: {}", e)))
    }
}
