//! AI-driven optimization for Only1MCP
//!
//! Provides intelligent request routing, predictive caching, anomaly detection,
//! and auto-scaling recommendations using machine learning techniques.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Request pattern for ML-based routing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestPattern {
    /// Request method/tool name
    pub method: String,

    /// Server ID that handled the request
    pub server_id: String,

    /// Response time in milliseconds
    pub response_time_ms: u64,

    /// Success status
    pub success: bool,

    /// Request timestamp
    pub timestamp: DateTime<Utc>,

    /// Request size in bytes
    pub request_size: usize,

    /// Response size in bytes
    pub response_size: usize,
}

/// ML-based request router using performance history
pub struct MLRouter {
    /// Historical request patterns
    patterns: Arc<RwLock<VecDeque<RequestPattern>>>,

    /// Maximum patterns to keep
    max_history: usize,

    /// Server performance scores
    server_scores: Arc<RwLock<HashMap<String, ServerScore>>>,
}

impl MLRouter {
    pub fn new(max_history: usize) -> Self {
        Self {
            patterns: Arc::new(RwLock::new(VecDeque::with_capacity(max_history))),
            max_history,
            server_scores: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Record a request pattern
    pub async fn record_pattern(&self, pattern: RequestPattern) {
        let mut patterns = self.patterns.write().await;

        if patterns.len() >= self.max_history {
            patterns.pop_front();
        }

        patterns.push_back(pattern.clone());
        drop(patterns);

        // Update server score
        self.update_server_score(&pattern).await;
    }

    /// Update server performance score
    async fn update_server_score(&self, pattern: &RequestPattern) {
        let mut scores = self.server_scores.write().await;

        let score = scores
            .entry(pattern.server_id.clone())
            .or_insert_with(|| ServerScore::new(pattern.server_id.clone()));

        // Update exponential moving average of response time
        let alpha = 0.3; // Weight for new observations
        score.avg_response_time = alpha * pattern.response_time_ms as f64
            + (1.0 - alpha) * score.avg_response_time;

        // Update success rate
        score.total_requests += 1;
        if pattern.success {
            score.successful_requests += 1;
        }
        score.success_rate =
            score.successful_requests as f64 / score.total_requests as f64;

        score.last_updated = Utc::now();
    }

    /// Get the best server for a given method based on historical performance
    pub async fn suggest_server(&self, method: &str, available_servers: &[String]) -> Option<String> {
        let patterns = self.patterns.read().await;
        let scores = self.server_scores.read().await;

        // Filter patterns for this method
        let method_patterns: Vec<&RequestPattern> = patterns
            .iter()
            .filter(|p| p.method == method)
            .collect();

        if method_patterns.is_empty() {
            // No historical data, use server with best overall score
            return self.best_overall_server(&scores, available_servers).await;
        }

        // Calculate performance score for each available server
        let mut server_performances: Vec<(String, f64)> = Vec::new();

        for server_id in available_servers {
            let server_patterns: Vec<&&RequestPattern> = method_patterns
                .iter()
                .filter(|p| &p.server_id == server_id)
                .collect();

            if server_patterns.is_empty() {
                continue;
            }

            // Calculate weighted score
            let avg_response_time: f64 = server_patterns
                .iter()
                .map(|p| p.response_time_ms as f64)
                .sum::<f64>()
                / server_patterns.len() as f64;

            let success_count = server_patterns.iter().filter(|p| p.success).count();
            let success_rate = success_count as f64 / server_patterns.len() as f64;

            // Score: higher success rate and lower response time is better
            // Normalize response time to 0-1 range (inverse, so lower is better)
            let response_time_score = 1.0 / (1.0 + avg_response_time / 1000.0);

            // Combined score (70% success rate, 30% response time)
            let score = 0.7 * success_rate + 0.3 * response_time_score;

            server_performances.push((server_id.clone(), score));
        }

        // Sort by score descending
        server_performances.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        server_performances.first().map(|(id, _)| id.clone())
    }

    /// Get best overall server when no method-specific data exists
    async fn best_overall_server(
        &self,
        scores: &HashMap<String, ServerScore>,
        available_servers: &[String],
    ) -> Option<String> {
        let mut best_server = None;
        let mut best_score = 0.0;

        for server_id in available_servers {
            if let Some(score) = scores.get(server_id) {
                let combined_score = score.success_rate * 0.7
                    + (1.0 / (1.0 + score.avg_response_time / 1000.0)) * 0.3;

                if combined_score > best_score {
                    best_score = combined_score;
                    best_server = Some(server_id.clone());
                }
            }
        }

        best_server.or_else(|| available_servers.first().cloned())
    }

    /// Get server performance statistics
    pub async fn get_server_stats(&self) -> HashMap<String, ServerScore> {
        let scores = self.server_scores.read().await;
        scores.clone()
    }
}

/// Server performance score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerScore {
    pub server_id: String,
    pub avg_response_time: f64,
    pub success_rate: f64,
    pub total_requests: u64,
    pub successful_requests: u64,
    pub last_updated: DateTime<Utc>,
}

impl ServerScore {
    fn new(server_id: String) -> Self {
        Self {
            server_id,
            avg_response_time: 0.0,
            success_rate: 1.0,
            total_requests: 0,
            successful_requests: 0,
            last_updated: Utc::now(),
        }
    }
}

/// Predictive cache manager
pub struct PredictiveCache {
    /// Access patterns for cache keys
    access_patterns: Arc<RwLock<HashMap<String, AccessPattern>>>,

    /// Predicted next accesses
    predictions: Arc<RwLock<Vec<CachePrediction>>>,
}

impl PredictiveCache {
    pub fn new() -> Self {
        Self {
            access_patterns: Arc::new(RwLock::new(HashMap::new())),
            predictions: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Record a cache access
    pub async fn record_access(&self, key: String, hit: bool) {
        let mut patterns = self.access_patterns.write().await;

        let pattern = patterns
            .entry(key.clone())
            .or_insert_with(|| AccessPattern::new(key.clone()));

        pattern.access_count += 1;
        pattern.last_access = Utc::now();

        if hit {
            pattern.hit_count += 1;
        }

        // Record access time for frequency analysis
        pattern.access_times.push(Utc::now());

        // Keep only recent access times (last 100)
        if pattern.access_times.len() > 100 {
            pattern.access_times.remove(0);
        }
    }

    /// Predict which cache keys should be preloaded
    pub async fn predict_preload(&self) -> Vec<String> {
        let patterns = self.access_patterns.read().await;
        let mut predictions: Vec<(String, f64)> = Vec::new();

        for (key, pattern) in patterns.iter() {
            let score = self.calculate_preload_score(pattern);
            if score > 0.5 {
                // Threshold for preloading
                predictions.push((key.clone(), score));
            }
        }

        // Sort by score descending
        predictions.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        // Return top 20 candidates
        predictions
            .into_iter()
            .take(20)
            .map(|(key, _)| key)
            .collect()
    }

    /// Calculate preload score for a cache key
    fn calculate_preload_score(&self, pattern: &AccessPattern) -> f64 {
        // Factors:
        // 1. Access frequency (higher is better)
        // 2. Hit rate (higher is better)
        // 3. Recency (more recent is better)
        // 4. Access pattern regularity

        let hit_rate = if pattern.access_count > 0 {
            pattern.hit_count as f64 / pattern.access_count as f64
        } else {
            0.0
        };

        // Recency score (decay over 24 hours)
        let hours_since_access = (Utc::now() - pattern.last_access).num_hours() as f64;
        let recency_score = 1.0 / (1.0 + hours_since_access / 24.0);

        // Frequency score (normalized by log scale)
        let frequency_score = (pattern.access_count as f64).ln() / 10.0;

        // Regularity score (based on variance of access intervals)
        let regularity_score = self.calculate_regularity(&pattern.access_times);

        // Weighted combination
        0.3 * hit_rate
            + 0.3 * recency_score
            + 0.2 * frequency_score.min(1.0)
            + 0.2 * regularity_score
    }

    /// Calculate regularity of access pattern
    fn calculate_regularity(&self, access_times: &[DateTime<Utc>]) -> f64 {
        if access_times.len() < 2 {
            return 0.0;
        }

        // Calculate intervals between accesses
        let mut intervals = Vec::new();
        for i in 1..access_times.len() {
            let interval = (access_times[i] - access_times[i - 1]).num_seconds() as f64;
            intervals.push(interval);
        }

        // Calculate coefficient of variation (lower is more regular)
        let mean = intervals.iter().sum::<f64>() / intervals.len() as f64;
        let variance = intervals
            .iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>()
            / intervals.len() as f64;
        let std_dev = variance.sqrt();

        let cv = if mean > 0.0 { std_dev / mean } else { 1.0 };

        // Convert to regularity score (inverse of CV, normalized)
        1.0 / (1.0 + cv)
    }

    /// Get cache statistics
    pub async fn get_stats(&self) -> CacheStats {
        let patterns = self.access_patterns.read().await;

        let total_keys = patterns.len();
        let total_accesses: u64 = patterns.values().map(|p| p.access_count).sum();
        let total_hits: u64 = patterns.values().map(|p| p.hit_count).sum();

        let hit_rate = if total_accesses > 0 {
            total_hits as f64 / total_accesses as f64
        } else {
            0.0
        };

        CacheStats {
            total_keys,
            total_accesses,
            total_hits,
            hit_rate,
        }
    }
}

impl Default for PredictiveCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Access pattern for a cache key
#[derive(Debug, Clone)]
struct AccessPattern {
    key: String,
    access_count: u64,
    hit_count: u64,
    last_access: DateTime<Utc>,
    access_times: Vec<DateTime<Utc>>,
}

impl AccessPattern {
    fn new(key: String) -> Self {
        Self {
            key,
            access_count: 0,
            hit_count: 0,
            last_access: Utc::now(),
            access_times: Vec::new(),
        }
    }
}

/// Cache prediction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachePrediction {
    pub key: String,
    pub confidence: f64,
    pub predicted_time: DateTime<Utc>,
}

/// Cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub total_keys: usize,
    pub total_accesses: u64,
    pub total_hits: u64,
    pub hit_rate: f64,
}

/// Anomaly detector for detecting unusual request patterns
pub struct AnomalyDetector {
    /// Historical metrics for baseline
    baseline: Arc<RwLock<MetricBaseline>>,

    /// Recent anomalies detected
    anomalies: Arc<RwLock<VecDeque<Anomaly>>>,

    /// Sensitivity threshold (higher = more sensitive)
    sensitivity: f64,
}

impl AnomalyDetector {
    pub fn new(sensitivity: f64) -> Self {
        Self {
            baseline: Arc::new(RwLock::new(MetricBaseline::new())),
            anomalies: Arc::new(RwLock::new(VecDeque::new())),
            sensitivity,
        }
    }

    /// Update baseline with new metric value
    pub async fn update_baseline(&self, metric_name: &str, value: f64) {
        let mut baseline = self.baseline.write().await;
        baseline.update(metric_name, value);
    }

    /// Check if a metric value is anomalous
    pub async fn check_anomaly(&self, metric_name: &str, value: f64) -> Option<Anomaly> {
        let baseline = self.baseline.read().await;

        if let Some(stats) = baseline.get_stats(metric_name) {
            // Use standard deviation for anomaly detection
            let z_score = (value - stats.mean).abs() / stats.std_dev;

            // Anomaly if z-score exceeds threshold
            if z_score > self.sensitivity {
                let severity = if z_score > self.sensitivity * 2.0 {
                    AnomalySeverity::Critical
                } else if z_score > self.sensitivity * 1.5 {
                    AnomalySeverity::High
                } else {
                    AnomalySeverity::Medium
                };

                let anomaly = Anomaly {
                    metric: metric_name.to_string(),
                    value,
                    expected: stats.mean,
                    deviation: z_score,
                    severity,
                    timestamp: Utc::now(),
                };

                // Record anomaly
                let mut anomalies = self.anomalies.write().await;
                anomalies.push_back(anomaly.clone());

                // Keep only recent anomalies (last 100)
                if anomalies.len() > 100 {
                    anomalies.pop_front();
                }

                warn!(
                    "Anomaly detected: {} = {} (expected {}, z-score: {:.2})",
                    metric_name, value, stats.mean, z_score
                );

                return Some(anomaly);
            }
        }

        None
    }

    /// Get recent anomalies
    pub async fn get_recent_anomalies(&self, limit: usize) -> Vec<Anomaly> {
        let anomalies = self.anomalies.read().await;
        anomalies.iter().rev().take(limit).cloned().collect()
    }
}

/// Metric baseline statistics
#[derive(Debug, Clone)]
struct MetricBaseline {
    metrics: HashMap<String, MetricStats>,
}

impl MetricBaseline {
    fn new() -> Self {
        Self {
            metrics: HashMap::new(),
        }
    }

    fn update(&mut self, metric_name: &str, value: f64) {
        let stats = self
            .metrics
            .entry(metric_name.to_string())
            .or_insert_with(MetricStats::new);

        stats.update(value);
    }

    fn get_stats(&self, metric_name: &str) -> Option<&MetricStats> {
        self.metrics.get(metric_name)
    }
}

/// Statistical metrics for anomaly detection
#[derive(Debug, Clone)]
struct MetricStats {
    mean: f64,
    variance: f64,
    std_dev: f64,
    count: u64,
    sum: f64,
    sum_squared: f64,
}

impl MetricStats {
    fn new() -> Self {
        Self {
            mean: 0.0,
            variance: 0.0,
            std_dev: 0.0,
            count: 0,
            sum: 0.0,
            sum_squared: 0.0,
        }
    }

    fn update(&mut self, value: f64) {
        self.count += 1;
        self.sum += value;
        self.sum_squared += value * value;

        self.mean = self.sum / self.count as f64;
        self.variance = (self.sum_squared / self.count as f64) - (self.mean * self.mean);
        self.std_dev = self.variance.sqrt();
    }
}

/// Detected anomaly
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Anomaly {
    pub metric: String,
    pub value: f64,
    pub expected: f64,
    pub deviation: f64,
    pub severity: AnomalySeverity,
    pub timestamp: DateTime<Utc>,
}

/// Anomaly severity levels
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AnomalySeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Auto-scaling recommender
pub struct ScalingRecommender {
    /// Load history
    load_history: Arc<RwLock<VecDeque<LoadMetric>>>,

    /// Maximum history to keep
    max_history: usize,
}

impl ScalingRecommender {
    pub fn new(max_history: usize) -> Self {
        Self {
            load_history: Arc::new(RwLock::new(VecDeque::with_capacity(max_history))),
            max_history,
        }
    }

    /// Record load metric
    pub async fn record_load(&self, metric: LoadMetric) {
        let mut history = self.load_history.write().await;

        if history.len() >= self.max_history {
            history.pop_front();
        }

        history.push_back(metric);
    }

    /// Get scaling recommendation
    pub async fn get_recommendation(&self) -> ScalingRecommendation {
        let history = self.load_history.read().await;

        if history.len() < 10 {
            return ScalingRecommendation {
                action: ScalingAction::NoChange,
                confidence: 0.0,
                reason: "Insufficient data".to_string(),
                suggested_instances: 0,
            };
        }

        // Analyze recent trends
        let recent_load: Vec<&LoadMetric> = history.iter().rev().take(20).collect();

        let avg_cpu = recent_load.iter().map(|m| m.cpu_percent).sum::<f64>()
            / recent_load.len() as f64;
        let avg_memory = recent_load.iter().map(|m| m.memory_percent).sum::<f64>()
            / recent_load.len() as f64;
        let avg_requests = recent_load.iter().map(|m| m.requests_per_second).sum::<f64>()
            / recent_load.len() as f64;

        // Calculate trend (simple linear regression slope)
        let cpu_trend = self.calculate_trend(&recent_load, |m| m.cpu_percent);
        let memory_trend = self.calculate_trend(&recent_load, |m| m.memory_percent);

        // Decide scaling action
        let (action, reason, confidence) = if avg_cpu > 80.0 || avg_memory > 80.0 {
            (
                ScalingAction::ScaleUp,
                format!(
                    "High resource usage: CPU {:.1}%, Memory {:.1}%",
                    avg_cpu, avg_memory
                ),
                0.9,
            )
        } else if cpu_trend > 5.0 || memory_trend > 5.0 {
            (
                ScalingAction::ScaleUp,
                format!(
                    "Rising resource trend: CPU trend {:.2}, Memory trend {:.2}",
                    cpu_trend, memory_trend
                ),
                0.7,
            )
        } else if avg_cpu < 20.0 && avg_memory < 20.0 && avg_requests < 100.0 {
            (
                ScalingAction::ScaleDown,
                format!(
                    "Low resource usage: CPU {:.1}%, Memory {:.1}%, {:.0} req/s",
                    avg_cpu, avg_memory, avg_requests
                ),
                0.8,
            )
        } else {
            (ScalingAction::NoChange, "Resource usage normal".to_string(), 1.0)
        };

        // Calculate suggested instance count
        let current_instances = recent_load.last().map(|m| m.instance_count).unwrap_or(1);
        let suggested_instances = match action {
            ScalingAction::ScaleUp => {
                // Scale up by 50% or add at least 1 instance
                ((current_instances as f64 * 1.5).ceil() as u32).max(current_instances + 1)
            }
            ScalingAction::ScaleDown => {
                // Scale down by 33% but keep at least 1 instance
                ((current_instances as f64 * 0.67).floor() as u32).max(1)
            }
            ScalingAction::NoChange => current_instances,
        };

        info!(
            "Scaling recommendation: {:?} (confidence: {:.2}) - {}",
            action, confidence, reason
        );

        ScalingRecommendation {
            action,
            confidence,
            reason,
            suggested_instances,
        }
    }

    /// Calculate trend using simple linear regression
    fn calculate_trend(&self, data: &[&LoadMetric], extract: fn(&LoadMetric) -> f64) -> f64 {
        if data.len() < 2 {
            return 0.0;
        }

        let n = data.len() as f64;
        let x_values: Vec<f64> = (0..data.len()).map(|i| i as f64).collect();
        let y_values: Vec<f64> = data.iter().map(|m| extract(m)).collect();

        let sum_x: f64 = x_values.iter().sum();
        let sum_y: f64 = y_values.iter().sum();
        let sum_xy: f64 = x_values.iter().zip(&y_values).map(|(x, y)| x * y).sum();
        let sum_x_squared: f64 = x_values.iter().map(|x| x * x).sum();

        // Slope of regression line
        (n * sum_xy - sum_x * sum_y) / (n * sum_x_squared - sum_x * sum_x)
    }
}

/// Load metric for scaling decisions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadMetric {
    pub timestamp: DateTime<Utc>,
    pub cpu_percent: f64,
    pub memory_percent: f64,
    pub requests_per_second: f64,
    pub instance_count: u32,
}

/// Scaling recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalingRecommendation {
    pub action: ScalingAction,
    pub confidence: f64,
    pub reason: String,
    pub suggested_instances: u32,
}

/// Scaling action types
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ScalingAction {
    ScaleUp,
    ScaleDown,
    NoChange,
}

/// AI optimization engine coordinating all AI features
pub struct AIEngine {
    ml_router: Arc<MLRouter>,
    predictive_cache: Arc<PredictiveCache>,
    anomaly_detector: Arc<AnomalyDetector>,
    scaling_recommender: Arc<ScalingRecommender>,
}

impl AIEngine {
    pub fn new() -> Self {
        Self {
            ml_router: Arc::new(MLRouter::new(10000)),
            predictive_cache: Arc::new(PredictiveCache::new()),
            anomaly_detector: Arc::new(AnomalyDetector::new(3.0)), // 3 standard deviations
            scaling_recommender: Arc::new(ScalingRecommender::new(1000)),
        }
    }

    pub fn ml_router(&self) -> Arc<MLRouter> {
        self.ml_router.clone()
    }

    pub fn predictive_cache(&self) -> Arc<PredictiveCache> {
        self.predictive_cache.clone()
    }

    pub fn anomaly_detector(&self) -> Arc<AnomalyDetector> {
        self.anomaly_detector.clone()
    }

    pub fn scaling_recommender(&self) -> Arc<ScalingRecommender> {
        self.scaling_recommender.clone()
    }

    /// Start background monitoring and optimization
    pub async fn start_background_tasks(&self) {
        // Start predictive cache preloading
        let cache = self.predictive_cache.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(300)); // Every 5 minutes

            loop {
                interval.tick().await;
                let preload_keys = cache.predict_preload().await;
                if !preload_keys.is_empty() {
                    debug!("Predicted {} cache keys for preloading", preload_keys.len());
                }
            }
        });

        // Start scaling recommendation monitoring
        let recommender = self.scaling_recommender.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60)); // Every minute

            loop {
                interval.tick().await;
                let recommendation = recommender.get_recommendation().await;
                if recommendation.action != ScalingAction::NoChange {
                    info!("Scaling recommendation: {:?}", recommendation);
                }
            }
        });
    }
}

impl Default for AIEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ml_router() {
        let router = MLRouter::new(100);

        // Record some patterns
        router
            .record_pattern(RequestPattern {
                method: "get_tools".to_string(),
                server_id: "server1".to_string(),
                response_time_ms: 100,
                success: true,
                timestamp: Utc::now(),
                request_size: 1024,
                response_size: 2048,
            })
            .await;

        router
            .record_pattern(RequestPattern {
                method: "get_tools".to_string(),
                server_id: "server2".to_string(),
                response_time_ms: 200,
                success: true,
                timestamp: Utc::now(),
                request_size: 1024,
                response_size: 2048,
            })
            .await;

        // Suggest should prefer server1 (lower response time)
        let suggestion = router
            .suggest_server("get_tools", &["server1".to_string(), "server2".to_string()])
            .await;

        assert_eq!(suggestion, Some("server1".to_string()));
    }

    #[tokio::test]
    async fn test_predictive_cache() {
        let cache = PredictiveCache::new();

        // Record accesses
        cache.record_access("key1".to_string(), true).await;
        cache.record_access("key1".to_string(), true).await;
        cache.record_access("key2".to_string(), false).await;

        let stats = cache.get_stats().await;
        assert_eq!(stats.total_keys, 2);
        assert_eq!(stats.total_accesses, 3);
        assert_eq!(stats.total_hits, 2);
    }

    #[tokio::test]
    async fn test_anomaly_detector() {
        let detector = AnomalyDetector::new(3.0);

        // Build baseline
        for i in 0..100 {
            detector.update_baseline("cpu", 50.0 + (i as f64 % 10.0)).await;
        }

        // Normal value should not trigger
        let anomaly = detector.check_anomaly("cpu", 55.0).await;
        assert!(anomaly.is_none());

        // Extreme value should trigger
        let anomaly = detector.check_anomaly("cpu", 200.0).await;
        assert!(anomaly.is_some());
    }

    #[tokio::test]
    async fn test_scaling_recommender() {
        let recommender = ScalingRecommender::new(100);

        // Record high load
        for _ in 0..20 {
            recommender
                .record_load(LoadMetric {
                    timestamp: Utc::now(),
                    cpu_percent: 85.0,
                    memory_percent: 75.0,
                    requests_per_second: 1000.0,
                    instance_count: 2,
                })
                .await;
        }

        let recommendation = recommender.get_recommendation().await;
        assert_eq!(recommendation.action, ScalingAction::ScaleUp);
        assert!(recommendation.suggested_instances > 2);
    }
}
