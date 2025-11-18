//! Advanced analytics and monitoring for Only1MCP
//!
//! Provides time-series metrics storage, custom dashboards, query language,
//! and alerting capabilities for comprehensive system observability.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, warn};


/// Time-series data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataPoint {
    /// Timestamp of the data point
    pub timestamp: DateTime<Utc>,

    /// Metric value
    pub value: f64,

    /// Optional tags/labels for this data point
    pub tags: HashMap<String, String>,
}

impl DataPoint {
    pub fn new(value: f64) -> Self {
        Self {
            timestamp: Utc::now(),
            value,
            tags: HashMap::new(),
        }
    }

    pub fn with_tags(mut self, tags: HashMap<String, String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn with_tag(mut self, key: String, value: String) -> Self {
        self.tags.insert(key, value);
        self
    }
}

/// Time-series metric storage
#[derive(Debug, Clone)]
pub struct TimeSeries {
    /// Metric name
    pub name: String,

    /// Data points ordered by timestamp
    pub points: BTreeMap<DateTime<Utc>, DataPoint>,

    /// Retention period for data points
    pub retention: Duration,

    /// Aggregation type for downsampling
    pub aggregation: AggregationType,
}

impl TimeSeries {
    pub fn new(name: String, retention: Duration, aggregation: AggregationType) -> Self {
        Self {
            name,
            points: BTreeMap::new(),
            retention,
            aggregation,
        }
    }

    /// Add a data point to the time series
    pub fn add_point(&mut self, point: DataPoint) {
        self.points.insert(point.timestamp, point);
        self.cleanup_old_points();
    }

    /// Remove points older than retention period
    fn cleanup_old_points(&mut self) {
        let cutoff = Utc::now() - self.retention;
        self.points.retain(|timestamp, _| *timestamp >= cutoff);
    }

    /// Query data points in a time range
    pub fn query_range(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Vec<&DataPoint> {
        self.points
            .range(start..=end)
            .map(|(_, point)| point)
            .collect()
    }

    /// Aggregate values over a time window
    pub fn aggregate_window(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Option<f64> {
        let points = self.query_range(start, end);
        if points.is_empty() {
            return None;
        }

        match self.aggregation {
            AggregationType::Sum => Some(points.iter().map(|p| p.value).sum()),
            AggregationType::Average => {
                let sum: f64 = points.iter().map(|p| p.value).sum();
                Some(sum / points.len() as f64)
            }
            AggregationType::Min => points.iter().map(|p| p.value).min_by(|a, b| a.partial_cmp(b).unwrap()),
            AggregationType::Max => points.iter().map(|p| p.value).max_by(|a, b| a.partial_cmp(b).unwrap()),
            AggregationType::Count => Some(points.len() as f64),
            AggregationType::Last => points.last().map(|p| p.value),
        }
    }

    /// Downsample to a specific interval
    pub fn downsample(&self, interval: Duration) -> Vec<DataPoint> {
        let mut result = Vec::new();

        if self.points.is_empty() {
            return result;
        }

        let first_timestamp = self.points.keys().next().unwrap();
        let mut current_window_start = *first_timestamp;

        while current_window_start <= Utc::now() {
            let window_end = current_window_start + interval;

            if let Some(value) = self.aggregate_window(current_window_start, window_end) {
                result.push(DataPoint {
                    timestamp: current_window_start,
                    value,
                    tags: HashMap::new(),
                });
            }

            current_window_start = window_end;
        }

        result
    }
}

/// Aggregation types for time-series data
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AggregationType {
    Sum,
    Average,
    Min,
    Max,
    Count,
    Last,
}

/// Metrics repository storing all time-series data
pub struct MetricsRepository {
    metrics: Arc<RwLock<HashMap<String, TimeSeries>>>,
    default_retention: Duration,
}

impl MetricsRepository {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(HashMap::new())),
            default_retention: Duration::hours(24), // 24 hours default
        }
    }

    pub fn with_retention(mut self, retention: Duration) -> Self {
        self.default_retention = retention;
        self
    }

    /// Record a metric value
    pub async fn record(&self, metric_name: &str, value: f64, tags: HashMap<String, String>) {
        let mut metrics = self.metrics.write().await;

        let series = metrics.entry(metric_name.to_string()).or_insert_with(|| {
            TimeSeries::new(
                metric_name.to_string(),
                self.default_retention,
                AggregationType::Average,
            )
        });

        series.add_point(DataPoint::new(value).with_tags(tags));
    }

    /// Get time series by name
    pub async fn get(&self, metric_name: &str) -> Option<TimeSeries> {
        let metrics = self.metrics.read().await;
        metrics.get(metric_name).cloned()
    }

    /// Query multiple metrics with filters
    pub async fn query(&self, query: MetricQuery) -> Vec<QueryResult> {
        let metrics = self.metrics.read().await;
        let mut results = Vec::new();

        for (name, series) in metrics.iter() {
            // Apply metric name filter
            if let Some(ref pattern) = query.metric_pattern {
                if !name.contains(pattern) {
                    continue;
                }
            }

            // Query time range
            let points = series.query_range(query.start, query.end);

            // Apply tag filters
            let filtered_points: Vec<&DataPoint> = points
                .into_iter()
                .filter(|point| {
                    query.tags.iter().all(|(key, value)| {
                        point.tags.get(key).map_or(false, |v| v == value)
                    })
                })
                .collect();

            if !filtered_points.is_empty() {
                results.push(QueryResult {
                    metric_name: name.clone(),
                    points: filtered_points.iter().map(|&p| p.clone()).collect(),
                    aggregated_value: series.aggregate_window(query.start, query.end),
                });
            }
        }

        results
    }

    /// List all metric names
    pub async fn list_metrics(&self) -> Vec<String> {
        let metrics = self.metrics.read().await;
        metrics.keys().cloned().collect()
    }

    /// Delete a metric
    pub async fn delete(&self, metric_name: &str) -> bool {
        let mut metrics = self.metrics.write().await;
        metrics.remove(metric_name).is_some()
    }

    /// Clean up old data points across all metrics
    pub async fn cleanup(&self) {
        let mut metrics = self.metrics.write().await;
        for series in metrics.values_mut() {
            series.cleanup_old_points();
        }
    }
}

impl Default for MetricsRepository {
    fn default() -> Self {
        Self::new()
    }
}

/// Query for retrieving metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricQuery {
    /// Pattern to match metric names (substring match)
    pub metric_pattern: Option<String>,

    /// Start time for query range
    pub start: DateTime<Utc>,

    /// End time for query range
    pub end: DateTime<Utc>,

    /// Tag filters (all must match)
    pub tags: HashMap<String, String>,
}

impl MetricQuery {
    pub fn new(start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        Self {
            metric_pattern: None,
            start,
            end,
            tags: HashMap::new(),
        }
    }

    pub fn with_pattern(mut self, pattern: String) -> Self {
        self.metric_pattern = Some(pattern);
        self
    }

    pub fn with_tag(mut self, key: String, value: String) -> Self {
        self.tags.insert(key, value);
        self
    }

    /// Query for last N minutes
    pub fn last_minutes(minutes: i64) -> Self {
        let end = Utc::now();
        let start = end - Duration::minutes(minutes);
        Self::new(start, end)
    }

    /// Query for last N hours
    pub fn last_hours(hours: i64) -> Self {
        let end = Utc::now();
        let start = end - Duration::hours(hours);
        Self::new(start, end)
    }
}

/// Result of a metric query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub metric_name: String,
    pub points: Vec<DataPoint>,
    pub aggregated_value: Option<f64>,
}

/// Dashboard configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dashboard {
    /// Dashboard ID
    pub id: String,

    /// Dashboard name
    pub name: String,

    /// Dashboard description
    pub description: String,

    /// Widgets in this dashboard
    pub widgets: Vec<DashboardWidget>,

    /// Refresh interval in seconds
    pub refresh_interval: u32,

    /// Dashboard tags
    pub tags: Vec<String>,
}

/// Dashboard widget displaying metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardWidget {
    /// Widget ID
    pub id: String,

    /// Widget title
    pub title: String,

    /// Widget type
    pub widget_type: WidgetType,

    /// Metrics to display
    pub metrics: Vec<String>,

    /// Query configuration
    pub query: MetricQuery,

    /// Position and size
    pub layout: WidgetLayout,
}

/// Widget visualization types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WidgetType {
    LineChart,
    BarChart,
    Gauge,
    SingleStat,
    Table,
    Heatmap,
}

/// Widget layout configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetLayout {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// Alert rule configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    /// Rule ID
    pub id: String,

    /// Rule name
    pub name: String,

    /// Metric to monitor
    pub metric: String,

    /// Alert condition
    pub condition: AlertCondition,

    /// Evaluation interval
    pub interval: Duration,

    /// How long condition must be true before alerting
    pub for_duration: Duration,

    /// Alert severity
    pub severity: AlertSeverity,

    /// Notification channels
    pub channels: Vec<String>,

    /// Whether the rule is enabled
    pub enabled: bool,
}

/// Alert condition types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AlertCondition {
    Threshold {
        operator: ComparisonOperator,
        value: f64,
    },
    RateOfChange {
        operator: ComparisonOperator,
        value: f64,
        window: Duration,
    },
    Anomaly {
        sensitivity: f64,
    },
}

/// Comparison operators for alerts
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonOperator {
    GreaterThan,
    LessThan,
    Equal,
    NotEqual,
    GreaterThanOrEqual,
    LessThanOrEqual,
}

impl ComparisonOperator {
    pub fn evaluate(&self, a: f64, b: f64) -> bool {
        match self {
            ComparisonOperator::GreaterThan => a > b,
            ComparisonOperator::LessThan => a < b,
            ComparisonOperator::Equal => (a - b).abs() < f64::EPSILON,
            ComparisonOperator::NotEqual => (a - b).abs() >= f64::EPSILON,
            ComparisonOperator::GreaterThanOrEqual => a >= b,
            ComparisonOperator::LessThanOrEqual => a <= b,
        }
    }
}

/// Alert severity levels
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

/// Alert manager for evaluating rules and triggering notifications
pub struct AlertManager {
    rules: Arc<RwLock<HashMap<String, AlertRule>>>,
    active_alerts: Arc<RwLock<HashMap<String, ActiveAlert>>>,
    repository: Arc<MetricsRepository>,
}

impl AlertManager {
    pub fn new(repository: Arc<MetricsRepository>) -> Self {
        Self {
            rules: Arc::new(RwLock::new(HashMap::new())),
            active_alerts: Arc::new(RwLock::new(HashMap::new())),
            repository,
        }
    }

    /// Add an alert rule
    pub async fn add_rule(&self, rule: AlertRule) {
        let mut rules = self.rules.write().await;
        rules.insert(rule.id.clone(), rule);
    }

    /// Remove an alert rule
    pub async fn remove_rule(&self, rule_id: &str) -> bool {
        let mut rules = self.rules.write().await;
        rules.remove(rule_id).is_some()
    }

    /// Evaluate all alert rules
    pub async fn evaluate_all(&self) -> Vec<AlertEvent> {
        let rules = self.rules.read().await;
        let mut events = Vec::new();

        for rule in rules.values() {
            if !rule.enabled {
                continue;
            }

            if let Some(event) = self.evaluate_rule(rule).await {
                events.push(event);
            }
        }

        events
    }

    /// Evaluate a single alert rule
    async fn evaluate_rule(&self, rule: &AlertRule) -> Option<AlertEvent> {
        // Get the metric time series
        let series = self.repository.get(&rule.metric).await?;

        // Calculate evaluation window
        let end = Utc::now();
        let start = end - rule.for_duration;

        // Get aggregated value
        let value = series.aggregate_window(start, end)?;

        // Evaluate condition
        let triggered = match &rule.condition {
            AlertCondition::Threshold { operator, value: threshold } => {
                operator.evaluate(value, *threshold)
            }
            AlertCondition::RateOfChange { operator, value: threshold, window } => {
                let previous_start = start - *window;
                let previous_value = series.aggregate_window(previous_start, start)?;
                let rate = (value - previous_value) / window.num_seconds() as f64;
                operator.evaluate(rate, *threshold)
            }
            AlertCondition::Anomaly { sensitivity: _ } => {
                // Simple anomaly detection: check if value is far from recent average
                // In production, use proper statistical methods
                let history_start = start - Duration::hours(24);
                let avg = series.aggregate_window(history_start, start)?;
                let deviation = (value - avg).abs();
                deviation > avg * 0.3 // 30% deviation threshold
            }
        };

        if triggered {
            debug!("Alert rule '{}' triggered with value {}", rule.name, value);

            let mut active_alerts = self.active_alerts.write().await;
            let alert = active_alerts.entry(rule.id.clone()).or_insert_with(|| {
                ActiveAlert {
                    rule_id: rule.id.clone(),
                    started_at: Utc::now(),
                    last_notified: None,
                }
            });

            // Check if we need to send notification
            if alert.should_notify() {
                alert.last_notified = Some(Utc::now());

                return Some(AlertEvent {
                    rule_id: rule.id.clone(),
                    rule_name: rule.name.clone(),
                    severity: rule.severity,
                    metric: rule.metric.clone(),
                    value,
                    message: format!(
                        "Alert '{}' triggered: {} = {}",
                        rule.name, rule.metric, value
                    ),
                    timestamp: Utc::now(),
                });
            }
        } else {
            // Alert resolved
            let mut active_alerts = self.active_alerts.write().await;
            if active_alerts.remove(&rule.id).is_some() {
                debug!("Alert rule '{}' resolved", rule.name);
            }
        }

        None
    }

    /// Get all active alerts
    pub async fn get_active_alerts(&self) -> Vec<String> {
        let active_alerts = self.active_alerts.read().await;
        active_alerts.keys().cloned().collect()
    }
}

/// Active alert state
#[derive(Debug, Clone)]
struct ActiveAlert {
    rule_id: String,
    started_at: DateTime<Utc>,
    last_notified: Option<DateTime<Utc>>,
}

impl ActiveAlert {
    fn should_notify(&self) -> bool {
        // Notify immediately if first time, otherwise wait 5 minutes
        match self.last_notified {
            None => true,
            Some(last) => Utc::now() - last > Duration::minutes(5),
        }
    }
}

/// Alert event that gets sent to notification channels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertEvent {
    pub rule_id: String,
    pub rule_name: String,
    pub severity: AlertSeverity,
    pub metric: String,
    pub value: f64,
    pub message: String,
    pub timestamp: DateTime<Utc>,
}

/// Analytics engine coordinating all analytics functionality
pub struct AnalyticsEngine {
    repository: Arc<MetricsRepository>,
    alert_manager: Arc<AlertManager>,
    dashboards: Arc<RwLock<HashMap<String, Dashboard>>>,
}

impl AnalyticsEngine {
    pub fn new() -> Self {
        let repository = Arc::new(MetricsRepository::new());
        let alert_manager = Arc::new(AlertManager::new(repository.clone()));

        Self {
            repository,
            alert_manager,
            dashboards: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn repository(&self) -> Arc<MetricsRepository> {
        self.repository.clone()
    }

    pub fn alert_manager(&self) -> Arc<AlertManager> {
        self.alert_manager.clone()
    }

    /// Add a dashboard
    pub async fn add_dashboard(&self, dashboard: Dashboard) {
        let mut dashboards = self.dashboards.write().await;
        dashboards.insert(dashboard.id.clone(), dashboard);
    }

    /// Get a dashboard
    pub async fn get_dashboard(&self, id: &str) -> Option<Dashboard> {
        let dashboards = self.dashboards.read().await;
        dashboards.get(id).cloned()
    }

    /// List all dashboards
    pub async fn list_dashboards(&self) -> Vec<Dashboard> {
        let dashboards = self.dashboards.read().await;
        dashboards.values().cloned().collect()
    }

    /// Start background evaluation of alerts
    pub async fn start_alert_evaluation(&self) {
        let alert_manager = self.alert_manager.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));

            loop {
                interval.tick().await;

                let events = alert_manager.evaluate_all().await;
                for event in events {
                    warn!(
                        "Alert triggered: {} - {} (severity: {:?})",
                        event.rule_name, event.message, event.severity
                    );
                    // In production, send to notification channels
                }
            }
        });
    }

    /// Start background cleanup of old metrics
    pub async fn start_cleanup(&self) {
        let repository = self.repository.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(3600)); // Every hour

            loop {
                interval.tick().await;
                repository.cleanup().await;
                debug!("Cleaned up old metric data points");
            }
        });
    }
}

impl Default for AnalyticsEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_point_creation() {
        let point = DataPoint::new(42.5)
            .with_tag("server".to_string(), "server1".to_string())
            .with_tag("region".to_string(), "us-east".to_string());

        assert_eq!(point.value, 42.5);
        assert_eq!(point.tags.len(), 2);
        assert_eq!(point.tags.get("server").unwrap(), "server1");
    }

    #[test]
    fn test_time_series_aggregation() {
        let mut series = TimeSeries::new(
            "test_metric".to_string(),
            Duration::hours(1),
            AggregationType::Average,
        );

        let now = Utc::now();
        series.add_point(DataPoint { timestamp: now, value: 10.0, tags: HashMap::new() });
        series.add_point(DataPoint { timestamp: now + Duration::seconds(30), value: 20.0, tags: HashMap::new() });
        series.add_point(DataPoint { timestamp: now + Duration::seconds(60), value: 30.0, tags: HashMap::new() });

        let avg = series.aggregate_window(now, now + Duration::minutes(2));
        assert_eq!(avg, Some(20.0));
    }

    #[test]
    fn test_comparison_operators() {
        assert!(ComparisonOperator::GreaterThan.evaluate(10.0, 5.0));
        assert!(ComparisonOperator::LessThan.evaluate(5.0, 10.0));
        assert!(ComparisonOperator::Equal.evaluate(5.0, 5.0));
        assert!(ComparisonOperator::GreaterThanOrEqual.evaluate(10.0, 10.0));
    }

    #[tokio::test]
    async fn test_metrics_repository() {
        let repo = MetricsRepository::new();

        let mut tags = HashMap::new();
        tags.insert("server".to_string(), "server1".to_string());

        repo.record("cpu_usage", 75.5, tags.clone()).await;
        repo.record("cpu_usage", 80.0, tags).await;

        let series = repo.get("cpu_usage").await.unwrap();
        assert_eq!(series.points.len(), 2);
    }

    #[tokio::test]
    async fn test_metric_query() {
        let repo = MetricsRepository::new();

        let mut tags1 = HashMap::new();
        tags1.insert("server".to_string(), "server1".to_string());

        let mut tags2 = HashMap::new();
        tags2.insert("server".to_string(), "server2".to_string());

        repo.record("requests", 100.0, tags1).await;
        repo.record("requests", 200.0, tags2.clone()).await;

        let query = MetricQuery::last_minutes(5)
            .with_tag("server".to_string(), "server2".to_string());

        let results = repo.query(query).await;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].points.len(), 1);
        assert_eq!(results[0].points[0].value, 200.0);
    }
}
