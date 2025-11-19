//! Custom Metrics Collector Plugin Example
//!
//! Demonstrates how to create a custom metrics collection plugin for Only1MCP
//! that tracks request patterns, performance, and custom business metrics

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use only1mcp::plugins::*;
use only1mcp::types::*;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Metric data point
#[derive(Clone, Debug)]
pub struct MetricPoint {
    pub timestamp: DateTime<Utc>,
    pub value: f64,
    pub tags: HashMap<String, String>,
}

/// Request metrics
#[derive(Clone, Debug)]
pub struct RequestMetrics {
    pub method: String,
    pub status: String,
    pub duration_ms: f64,
    pub request_size: usize,
    pub response_size: usize,
    pub timestamp: DateTime<Utc>,
}

/// Aggregated statistics
#[derive(Clone, Debug, Default)]
pub struct MetricStats {
    pub count: u64,
    pub sum: f64,
    pub min: f64,
    pub max: f64,
    pub mean: f64,
    pub p50: f64,
    pub p95: f64,
    pub p99: f64,
}

/// Custom metrics collector plugin
pub struct MetricsCollectorPlugin {
    metadata: PluginMetadata,
    state: PluginState,
    /// Store recent request metrics
    request_history: Arc<RwLock<VecDeque<RequestMetrics>>>,
    /// Store custom metric time series
    custom_metrics: Arc<RwLock<HashMap<String, VecDeque<MetricPoint>>>>,
    /// Maximum history size
    max_history_size: usize,
    /// Metric retention duration
    retention_duration: Duration,
}

impl MetricsCollectorPlugin {
    pub fn new(max_history_size: usize, retention_hours: u64) -> Self {
        Self {
            metadata: PluginMetadata {
                name: "metrics-collector".to_string(),
                version: "1.0.0".to_string(),
                author: "Only1MCP Team".to_string(),
                description: "Custom metrics collection and aggregation plugin".to_string(),
                capabilities: vec![
                    PluginCapability::RequestTransform,
                    PluginCapability::ResponseTransform,
                    PluginCapability::MetricsCollector,
                ],
                dependencies: vec![],
            },
            state: PluginState::Loaded,
            request_history: Arc::new(RwLock::new(VecDeque::with_capacity(max_history_size))),
            custom_metrics: Arc::new(RwLock::new(HashMap::new())),
            max_history_size,
            retention_duration: Duration::from_secs(retention_hours * 3600),
        }
    }

    /// Record a custom metric
    pub async fn record_metric(&self, name: String, value: f64, tags: HashMap<String, String>) {
        let point = MetricPoint {
            timestamp: Utc::now(),
            value,
            tags,
        };

        let mut metrics = self.custom_metrics.write().await;
        let series = metrics.entry(name).or_insert_with(|| VecDeque::new());

        series.push_back(point);

        // Enforce max size
        while series.len() > self.max_history_size {
            series.pop_front();
        }
    }

    /// Get metric statistics
    pub async fn get_stats(&self, metric_name: &str, window: Duration) -> Option<MetricStats> {
        let metrics = self.custom_metrics.read().await;
        let series = metrics.get(metric_name)?;

        let cutoff = Utc::now() - chrono::Duration::from_std(window).ok()?;
        let values: Vec<f64> = series
            .iter()
            .filter(|p| p.timestamp > cutoff)
            .map(|p| p.value)
            .collect();

        if values.is_empty() {
            return None;
        }

        Some(Self::calculate_stats(&values))
    }

    /// Get request statistics
    pub async fn get_request_stats(&self, window: Duration) -> MetricStats {
        let history = self.request_history.read().await;
        let cutoff = Utc::now() - chrono::Duration::from_std(window).unwrap();

        let durations: Vec<f64> = history
            .iter()
            .filter(|r| r.timestamp > cutoff)
            .map(|r| r.duration_ms)
            .collect();

        if durations.is_empty() {
            return MetricStats::default();
        }

        Self::calculate_stats(&durations)
    }

    /// Get request rate (requests per second)
    pub async fn get_request_rate(&self, window: Duration) -> f64 {
        let history = self.request_history.read().await;
        let cutoff = Utc::now() - chrono::Duration::from_std(window).unwrap();

        let count = history.iter().filter(|r| r.timestamp > cutoff).count();

        count as f64 / window.as_secs_f64()
    }

    /// Get error rate
    pub async fn get_error_rate(&self, window: Duration) -> f64 {
        let history = self.request_history.read().await;
        let cutoff = Utc::now() - chrono::Duration::from_std(window).unwrap();

        let recent: Vec<&RequestMetrics> = history
            .iter()
            .filter(|r| r.timestamp > cutoff)
            .collect();

        if recent.is_empty() {
            return 0.0;
        }

        let errors = recent
            .iter()
            .filter(|r| r.status.starts_with("error"))
            .count();

        errors as f64 / recent.len() as f64
    }

    /// Get metrics by method
    pub async fn get_metrics_by_method(&self, window: Duration) -> HashMap<String, MetricStats> {
        let history = self.request_history.read().await;
        let cutoff = Utc::now() - chrono::Duration::from_std(window).unwrap();

        let mut by_method: HashMap<String, Vec<f64>> = HashMap::new();

        for req in history.iter().filter(|r| r.timestamp > cutoff) {
            by_method
                .entry(req.method.clone())
                .or_insert_with(Vec::new)
                .push(req.duration_ms);
        }

        by_method
            .into_iter()
            .map(|(method, durations)| (method, Self::calculate_stats(&durations)))
            .collect()
    }

    /// Calculate statistics from values
    fn calculate_stats(values: &[f64]) -> MetricStats {
        if values.is_empty() {
            return MetricStats::default();
        }

        let mut sorted = values.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let count = sorted.len() as u64;
        let sum: f64 = sorted.iter().sum();
        let mean = sum / count as f64;

        MetricStats {
            count,
            sum,
            min: sorted[0],
            max: sorted[sorted.len() - 1],
            mean,
            p50: Self::percentile(&sorted, 0.50),
            p95: Self::percentile(&sorted, 0.95),
            p99: Self::percentile(&sorted, 0.99),
        }
    }

    /// Calculate percentile
    fn percentile(sorted_values: &[f64], p: f64) -> f64 {
        let index = (sorted_values.len() as f64 * p) as usize;
        let index = index.min(sorted_values.len() - 1);
        sorted_values[index]
    }

    /// Clean up old metrics
    async fn cleanup_old_metrics(&self) {
        let cutoff = Utc::now() - chrono::Duration::from_std(self.retention_duration).unwrap();

        // Clean request history
        let mut history = self.request_history.write().await;
        while let Some(req) = history.front() {
            if req.timestamp > cutoff {
                break;
            }
            history.pop_front();
        }

        // Clean custom metrics
        let mut metrics = self.custom_metrics.write().await;
        for series in metrics.values_mut() {
            while let Some(point) = series.front() {
                if point.timestamp > cutoff {
                    break;
                }
                series.pop_front();
            }
        }
    }
}

impl Default for MetricsCollectorPlugin {
    fn default() -> Self {
        Self::new(10000, 24) // 10k entries, 24 hour retention
    }
}

#[async_trait]
impl Plugin for MetricsCollectorPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    async fn initialize(
        &mut self,
        config: HashMap<String, serde_json::Value>,
    ) -> Result<(), String> {
        // Read custom config if provided
        if let Some(max_size) = config.get("max_history_size") {
            if let Some(size) = max_size.as_u64() {
                self.max_history_size = size as usize;
            }
        }

        if let Some(retention) = config.get("retention_hours") {
            if let Some(hours) = retention.as_u64() {
                self.retention_duration = Duration::from_secs(hours * 3600);
            }
        }

        self.state = PluginState::Ready;
        Ok(())
    }

    async fn start(&mut self) -> Result<(), String> {
        self.state = PluginState::Running;

        // Start background cleanup task
        let plugin = Arc::new(RwLock::new(self.clone()));
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(3600)); // Every hour

            loop {
                interval.tick().await;
                let p = plugin.read().await;
                p.cleanup_old_metrics().await;
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
        let history = self.request_history.read().await;
        let metrics = self.custom_metrics.read().await;

        Ok(PluginHealth {
            healthy: self.state == PluginState::Running,
            message: format!(
                "Tracking {} requests, {} custom metrics",
                history.len(),
                metrics.len()
            ),
        })
    }
}

#[async_trait]
impl RequestTransformer for MetricsCollectorPlugin {
    async fn transform_request(
        &self,
        mut request: McpRequest,
        context: &PluginContext,
    ) -> Result<McpRequest, String> {
        // Store request start time in context
        let start_time = Instant::now();
        request
            .headers
            .insert("X-Request-Start".to_string(), format!("{:?}", start_time));

        Ok(request)
    }
}

#[async_trait]
impl ResponseTransformer for MetricsCollectorPlugin {
    async fn transform_response(
        &self,
        response: McpResponse,
        context: &PluginContext,
    ) -> Result<McpResponse, String> {
        // Calculate request duration
        if let Some(start_time_str) = context.metadata.get("request_start_time") {
            // Record metrics
            let metrics = RequestMetrics {
                method: context
                    .metadata
                    .get("method")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string(),
                status: if response.error.is_some() {
                    "error".to_string()
                } else {
                    "success".to_string()
                },
                duration_ms: 0.0, // Would calculate from start_time
                request_size: 0,  // Would extract from context
                response_size: 0, // Would extract from response
                timestamp: Utc::now(),
            };

            let mut history = self.request_history.write().await;
            history.push_back(metrics);

            // Enforce max size
            while history.len() > self.max_history_size {
                history.pop_front();
            }
        }

        Ok(response)
    }
}

// Example usage
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_metrics_collector() {
        let plugin = MetricsCollectorPlugin::new(100, 24);

        // Record some custom metrics
        let mut tags = HashMap::new();
        tags.insert("server".to_string(), "server-1".to_string());

        plugin
            .record_metric("cpu_usage".to_string(), 75.5, tags.clone())
            .await;
        plugin
            .record_metric("cpu_usage".to_string(), 80.2, tags.clone())
            .await;
        plugin
            .record_metric("cpu_usage".to_string(), 72.1, tags)
            .await;

        // Get stats
        let stats = plugin
            .get_stats("cpu_usage", Duration::from_secs(3600))
            .await
            .unwrap();

        assert_eq!(stats.count, 3);
        assert!(stats.mean > 75.0 && stats.mean < 77.0);
        assert_eq!(stats.min, 72.1);
        assert_eq!(stats.max, 80.2);
    }

    #[test]
    fn test_percentile_calculation() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        assert_eq!(MetricsCollectorPlugin::percentile(&values, 0.50), 5.0);
        assert_eq!(MetricsCollectorPlugin::percentile(&values, 0.95), 10.0);
    }

    #[test]
    fn test_stats_calculation() {
        let values = vec![10.0, 20.0, 30.0, 40.0, 50.0];
        let stats = MetricsCollectorPlugin::calculate_stats(&values);

        assert_eq!(stats.count, 5);
        assert_eq!(stats.sum, 150.0);
        assert_eq!(stats.mean, 30.0);
        assert_eq!(stats.min, 10.0);
        assert_eq!(stats.max, 50.0);
    }
}
