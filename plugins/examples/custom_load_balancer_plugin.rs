//! Custom Load Balancer Plugin Example
//!
//! Demonstrates how to create a custom load balancing strategy plugin
//! with advanced routing algorithms

use async_trait::async_trait;
use only1mcp::plugins::*;
use only1mcp::types::*;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Server health metrics
#[derive(Clone, Debug)]
pub struct ServerMetrics {
    pub server_id: String,
    pub active_connections: u32,
    pub avg_response_time_ms: f64,
    pub error_rate: f64,
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub last_updated: Instant,
}

/// Load balancing strategy
#[derive(Clone, Debug)]
pub enum LoadBalancingStrategy {
    /// Route to server with lowest response time
    LeastLatency,
    /// Route to server with lowest error rate
    LeastErrors,
    /// Composite score based on multiple factors
    Weighted {
        latency_weight: f64,
        error_weight: f64,
        load_weight: f64,
        resource_weight: f64,
    },
    /// Adaptive strategy that learns optimal routing
    Adaptive,
}

/// Custom load balancer plugin
pub struct CustomLoadBalancerPlugin {
    metadata: PluginMetadata,
    state: PluginState,
    strategy: LoadBalancingStrategy,
    /// Server metrics
    server_metrics: Arc<RwLock<HashMap<String, ServerMetrics>>>,
    /// Request history for adaptive learning
    request_history: Arc<RwLock<VecDeque<RequestOutcome>>>,
    /// Maximum history size
    max_history_size: usize,
}

/// Outcome of a request
#[derive(Clone, Debug)]
pub struct RequestOutcome {
    pub server_id: String,
    pub response_time_ms: f64,
    pub success: bool,
    pub timestamp: Instant,
}

impl CustomLoadBalancerPlugin {
    pub fn new(strategy: LoadBalancingStrategy) -> Self {
        Self {
            metadata: PluginMetadata {
                name: "custom-load-balancer".to_string(),
                version: "1.0.0".to_string(),
                author: "Only1MCP Team".to_string(),
                description: "Advanced custom load balancing with multiple strategies".to_string(),
                capabilities: vec![PluginCapability::RequestTransform],
                dependencies: vec![],
            },
            state: PluginState::Loaded,
            strategy,
            server_metrics: Arc::new(RwLock::new(HashMap::new())),
            request_history: Arc::new(RwLock::new(VecDeque::with_capacity(1000))),
            max_history_size: 1000,
        }
    }

    /// Update server metrics
    pub async fn update_metrics(&self, metrics: ServerMetrics) {
        let mut server_metrics = self.server_metrics.write().await;
        server_metrics.insert(metrics.server_id.clone(), metrics);
    }

    /// Record request outcome
    pub async fn record_outcome(&self, outcome: RequestOutcome) {
        let mut history = self.request_history.write().await;
        history.push_back(outcome);

        while history.len() > self.max_history_size {
            history.pop_front();
        }
    }

    /// Select best server based on current strategy
    pub async fn select_server(&self, available_servers: &[String]) -> Option<String> {
        if available_servers.is_empty() {
            return None;
        }

        let metrics = self.server_metrics.read().await;

        match &self.strategy {
            LoadBalancingStrategy::LeastLatency => {
                self.select_by_latency(available_servers, &metrics)
            }
            LoadBalancingStrategy::LeastErrors => {
                self.select_by_errors(available_servers, &metrics)
            }
            LoadBalancingStrategy::Weighted {
                latency_weight,
                error_weight,
                load_weight,
                resource_weight,
            } => self.select_by_weighted_score(
                available_servers,
                &metrics,
                *latency_weight,
                *error_weight,
                *load_weight,
                *resource_weight,
            ),
            LoadBalancingStrategy::Adaptive => {
                self.select_adaptive(available_servers, &metrics).await
            }
        }
    }

    /// Select server with lowest latency
    fn select_by_latency(
        &self,
        servers: &[String],
        metrics: &HashMap<String, ServerMetrics>,
    ) -> Option<String> {
        servers
            .iter()
            .filter_map(|id| metrics.get(id).map(|m| (id, m)))
            .min_by(|(_, a), (_, b)| {
                a.avg_response_time_ms
                    .partial_cmp(&b.avg_response_time_ms)
                    .unwrap()
            })
            .map(|(id, _)| id.clone())
    }

    /// Select server with lowest error rate
    fn select_by_errors(
        &self,
        servers: &[String],
        metrics: &HashMap<String, ServerMetrics>,
    ) -> Option<String> {
        servers
            .iter()
            .filter_map(|id| metrics.get(id).map(|m| (id, m)))
            .min_by(|(_, a), (_, b)| a.error_rate.partial_cmp(&b.error_rate).unwrap())
            .map(|(id, _)| id.clone())
    }

    /// Select server using weighted composite score
    fn select_by_weighted_score(
        &self,
        servers: &[String],
        metrics: &HashMap<String, ServerMetrics>,
        latency_weight: f64,
        error_weight: f64,
        load_weight: f64,
        resource_weight: f64,
    ) -> Option<String> {
        // Normalize metrics to 0-1 range
        let max_latency = servers
            .iter()
            .filter_map(|id| metrics.get(id))
            .map(|m| m.avg_response_time_ms)
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(1.0);

        let max_connections = servers
            .iter()
            .filter_map(|id| metrics.get(id))
            .map(|m| m.active_connections)
            .max()
            .unwrap_or(1) as f64;

        // Calculate composite scores (lower is better)
        servers
            .iter()
            .filter_map(|id| {
                metrics.get(id).map(|m| {
                    let latency_score = m.avg_response_time_ms / max_latency.max(1.0);
                    let error_score = m.error_rate;
                    let load_score = m.active_connections as f64 / max_connections.max(1.0);
                    let resource_score = (m.cpu_usage + m.memory_usage) / 200.0; // Average of CPU and memory

                    let composite_score = latency_score * latency_weight
                        + error_score * error_weight
                        + load_score * load_weight
                        + resource_score * resource_weight;

                    (id, composite_score)
                })
            })
            .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .map(|(id, _)| id.clone())
    }

    /// Adaptive selection using historical performance
    async fn select_adaptive(
        &self,
        servers: &[String],
        metrics: &HashMap<String, ServerMetrics>,
    ) -> Option<String> {
        let history = self.request_history.read().await;

        // Calculate success rate and average response time per server
        let mut server_stats: HashMap<String, (f64, f64, usize)> = HashMap::new();

        let cutoff = Instant::now() - Duration::from_secs(300); // Last 5 minutes

        for outcome in history.iter() {
            if outcome.timestamp > cutoff {
                let (total_time, successes, count) = server_stats
                    .entry(outcome.server_id.clone())
                    .or_insert((0.0, 0.0, 0));

                *total_time += outcome.response_time_ms;
                *successes += if outcome.success { 1.0 } else { 0.0 };
                *count += 1;
            }
        }

        // Score servers based on historical performance
        servers
            .iter()
            .map(|id| {
                let (total_time, successes, count) =
                    server_stats.get(id).unwrap_or(&(100.0, 0.8, 1));

                let avg_time = if *count > 0 {
                    total_time / *count as f64
                } else {
                    // Use current metrics if no history
                    metrics
                        .get(id)
                        .map(|m| m.avg_response_time_ms)
                        .unwrap_or(100.0)
                };

                let success_rate = if *count > 0 {
                    successes / *count as f64
                } else {
                    // Use current metrics if no history
                    metrics
                        .get(id)
                        .map(|m| 1.0 - m.error_rate)
                        .unwrap_or(0.95)
                };

                // Score = (latency * 0.6) + (1 - success_rate) * 100 * 0.4
                // Lower is better
                let score = (avg_time * 0.6) + ((1.0 - success_rate) * 100.0 * 0.4);

                (id, score)
            })
            .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .map(|(id, _)| id.clone())
    }

    /// Get current metrics for a server
    pub async fn get_metrics(&self, server_id: &str) -> Option<ServerMetrics> {
        let metrics = self.server_metrics.read().await;
        metrics.get(server_id).cloned()
    }

    /// Get statistics for all servers
    pub async fn get_all_stats(&self) -> HashMap<String, ServerMetrics> {
        let metrics = self.server_metrics.read().await;
        metrics.clone()
    }
}

impl Default for CustomLoadBalancerPlugin {
    fn default() -> Self {
        Self::new(LoadBalancingStrategy::Weighted {
            latency_weight: 0.4,
            error_weight: 0.3,
            load_weight: 0.2,
            resource_weight: 0.1,
        })
    }
}

#[async_trait]
impl Plugin for CustomLoadBalancerPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    async fn initialize(
        &mut self,
        config: HashMap<String, serde_json::Value>,
    ) -> Result<(), String> {
        // Read strategy configuration
        if let Some(strategy_config) = config.get("strategy") {
            if let Some(strategy_str) = strategy_config.as_str() {
                self.strategy = match strategy_str {
                    "least_latency" => LoadBalancingStrategy::LeastLatency,
                    "least_errors" => LoadBalancingStrategy::LeastErrors,
                    "adaptive" => LoadBalancingStrategy::Adaptive,
                    _ => LoadBalancingStrategy::Weighted {
                        latency_weight: 0.4,
                        error_weight: 0.3,
                        load_weight: 0.2,
                        resource_weight: 0.1,
                    },
                };
            }
        }

        self.state = PluginState::Ready;
        Ok(())
    }

    async fn start(&mut self) -> Result<(), String> {
        self.state = PluginState::Running;

        // Start background metrics collection
        let server_metrics = self.server_metrics.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(10));

            loop {
                interval.tick().await;

                // Clean up stale metrics (older than 5 minutes)
                let mut metrics = server_metrics.write().await;
                let cutoff = Instant::now() - Duration::from_secs(300);

                metrics.retain(|_, m| m.last_updated > cutoff);
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
        let metrics = self.server_metrics.read().await;
        Ok(PluginHealth {
            healthy: self.state == PluginState::Running,
            message: format!(
                "Tracking {} servers with {:?} strategy",
                metrics.len(),
                self.strategy
            ),
        })
    }
}

#[async_trait]
impl RequestTransformer for CustomLoadBalancerPlugin {
    async fn transform_request(
        &self,
        mut request: McpRequest,
        context: &PluginContext,
    ) -> Result<McpRequest, String> {
        // Get available servers from context
        let available_servers: Vec<String> = context
            .metadata
            .get("available_servers")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        // Select best server
        if let Some(selected_server) = self.select_server(&available_servers).await {
            request.headers.insert(
                "X-Target-Server".to_string(),
                selected_server,
            );
        }

        Ok(request)
    }
}

// Example usage
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_load_balancer_least_latency() {
        let plugin = CustomLoadBalancerPlugin::new(LoadBalancingStrategy::LeastLatency);

        // Add server metrics
        plugin
            .update_metrics(ServerMetrics {
                server_id: "server-1".to_string(),
                active_connections: 10,
                avg_response_time_ms: 50.0,
                error_rate: 0.01,
                cpu_usage: 60.0,
                memory_usage: 70.0,
                last_updated: Instant::now(),
            })
            .await;

        plugin
            .update_metrics(ServerMetrics {
                server_id: "server-2".to_string(),
                active_connections: 5,
                avg_response_time_ms: 30.0, // Faster
                error_rate: 0.02,
                cpu_usage: 50.0,
                memory_usage: 60.0,
                last_updated: Instant::now(),
            })
            .await;

        let selected = plugin
            .select_server(&["server-1".to_string(), "server-2".to_string()])
            .await
            .unwrap();

        assert_eq!(selected, "server-2"); // Should select server with lowest latency
    }

    #[tokio::test]
    async fn test_load_balancer_weighted() {
        let plugin = CustomLoadBalancerPlugin::new(LoadBalancingStrategy::Weighted {
            latency_weight: 0.5,
            error_weight: 0.3,
            load_weight: 0.1,
            resource_weight: 0.1,
        });

        plugin
            .update_metrics(ServerMetrics {
                server_id: "server-1".to_string(),
                active_connections: 20,
                avg_response_time_ms: 40.0,
                error_rate: 0.05,
                cpu_usage: 80.0,
                memory_usage: 75.0,
                last_updated: Instant::now(),
            })
            .await;

        plugin
            .update_metrics(ServerMetrics {
                server_id: "server-2".to_string(),
                active_connections: 10,
                avg_response_time_ms: 35.0,
                error_rate: 0.01, // Lower error rate
                cpu_usage: 50.0,
                memory_usage: 55.0,
                last_updated: Instant::now(),
            })
            .await;

        let selected = plugin
            .select_server(&["server-1".to_string(), "server-2".to_string()])
            .await
            .unwrap();

        assert_eq!(selected, "server-2");
    }

    #[tokio::test]
    async fn test_record_outcomes() {
        let plugin = CustomLoadBalancerPlugin::new(LoadBalancingStrategy::Adaptive);

        // Record some outcomes
        plugin
            .record_outcome(RequestOutcome {
                server_id: "server-1".to_string(),
                response_time_ms: 45.0,
                success: true,
                timestamp: Instant::now(),
            })
            .await;

        plugin
            .record_outcome(RequestOutcome {
                server_id: "server-1".to_string(),
                response_time_ms: 50.0,
                success: true,
                timestamp: Instant::now(),
            })
            .await;

        let history = plugin.request_history.read().await;
        assert_eq!(history.len(), 2);
    }
}
