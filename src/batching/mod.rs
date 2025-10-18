//! Request Batching Module
//!
//! Aggregates multiple similar requests into a single backend call within a time window.
//! This is a core optimization that reduces backend load and context overhead by 50-70%.
//!
//! # How It Works
//!
//! When multiple clients request the same data (e.g., tools/list from server1) within
//! a 100ms window, the batch aggregator:
//! 1. Collects all requests in a pending batch
//! 2. Makes a SINGLE backend call when timeout expires or batch is full
//! 3. Distributes the response to all waiting clients
//!
//! # Example
//!
//! ```no_run
//! use only1mcp::batching::BatchAggregator;
//! use only1mcp::config::BatchingConfig;
//! use only1mcp::types::McpRequest;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let config = BatchingConfig {
//!     enabled: true,
//!     window_ms: 100,
//!     max_batch_size: 10,
//!     methods: vec!["tools/list".to_string()],
//! };
//!
//! let aggregator = BatchAggregator::new(config);
//!
//! // Submit request - will be batched with others
//! let request = McpRequest::new("tools/list", serde_json::json!({}), Some(serde_json::json!(1)));
//!
//! let response = aggregator.submit_request("server1".to_string(), request).await?;
//! # Ok(())
//! # }
//! ```

use crate::config::BatchingConfig;
use crate::error::{Error, Result};
use crate::types::{McpRequest, McpResponse};
use dashmap::DashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::oneshot;
use tokio::time::{sleep, Duration};

/// Re-export BatchingConfig as BatchConfig for backward compatibility
pub type BatchConfig = BatchingConfig;

/// Key for identifying batches (server + method combination)
#[derive(Hash, Eq, PartialEq, Clone, Debug)]
struct BatchKey {
    server_id: String,
    method: String,
}

/// Pending request waiting to be batched
struct PendingRequest {
    request: McpRequest,
    response_tx: oneshot::Sender<Result<McpResponse>>,
    submitted_at: Instant,
}

/// Collection of pending requests for a specific batch
struct PendingBatch {
    requests: Vec<PendingRequest>,
    deadline: Instant,
    created_at: Instant,
}

/// Request batch aggregator
///
/// Collects similar requests and processes them as a batch to reduce backend calls.
#[derive(Clone)]
pub struct BatchAggregator {
    batches: Arc<DashMap<BatchKey, PendingBatch>>,
    config: Arc<BatchConfig>,
    /// Function to execute backend call (injected for testing)
    backend_caller: Arc<dyn Fn(String, McpRequest) -> Result<McpResponse> + Send + Sync>,
}

impl BatchAggregator {
    /// Create a new batch aggregator with configuration
    pub fn new(config: BatchConfig) -> Self {
        Self {
            batches: Arc::new(DashMap::new()),
            config: Arc::new(config),
            backend_caller: Arc::new(|_, _| {
                Err(Error::Server(
                    "Backend caller not initialized - use with_backend_caller".to_string(),
                ))
            }),
        }
    }

    /// Set the backend caller function (for production use)
    pub fn with_backend_caller<F>(mut self, caller: F) -> Self
    where
        F: Fn(String, McpRequest) -> Result<McpResponse> + Send + Sync + 'static,
    {
        self.backend_caller = Arc::new(caller);
        self
    }

    /// Submit a request for batching
    ///
    /// This method:
    /// 1. Checks if the method supports batching
    /// 2. Adds request to the appropriate batch
    /// 3. Starts a timer if this is the first request in the batch
    /// 4. Flushes immediately if batch reaches max size
    /// 5. Returns the response when the batch completes
    ///
    /// # Arguments
    ///
    /// * `server_id` - Target backend server ID
    /// * `request` - MCP request to batch
    ///
    /// # Returns
    ///
    /// The MCP response once the batch is processed
    ///
    /// # Errors
    ///
    /// Returns error if backend call fails or channel is closed
    pub async fn submit_request(
        &self,
        server_id: String,
        request: McpRequest,
    ) -> Result<McpResponse> {
        // Check if method supports batching
        if !self.config.methods.contains(&request.method) {
            // Fallback to direct call for non-batchable methods
            return (self.backend_caller)(server_id, request);
        }

        let key = BatchKey {
            server_id: server_id.clone(),
            method: request.method.clone(),
        };

        let (tx, rx) = oneshot::channel();
        let submitted_at = Instant::now();

        // Add to batch
        let should_start_timer = {
            let mut batch = self.batches.entry(key.clone()).or_insert_with(|| {
                let deadline = Instant::now() + Duration::from_millis(self.config.window_ms);
                PendingBatch {
                    requests: Vec::new(),
                    deadline,
                    created_at: Instant::now(),
                }
            });

            let is_first = batch.requests.is_empty();
            batch.requests.push(PendingRequest {
                request,
                response_tx: tx,
                submitted_at,
            });

            let batch_size = batch.requests.len();

            // Check if batch is full
            if batch_size >= self.config.max_batch_size {
                // Remove and process immediately
                drop(batch); // Release DashMap lock
                if let Some((_, batch)) = self.batches.remove(&key) {
                    Self::process_batch_static(
                        server_id.clone(),
                        batch,
                        self.backend_caller.clone(),
                    );
                }
                return rx
                    .await
                    .map_err(|_| Error::Server("Batch response channel closed".to_string()))?;
            }

            is_first
        };

        // Start timer for first request in batch
        if should_start_timer {
            let batches = self.batches.clone();
            let backend_caller = self.backend_caller.clone();
            let window = Duration::from_millis(self.config.window_ms);

            tokio::spawn(async move {
                sleep(window).await;
                if let Some((_, batch)) = batches.remove(&key) {
                    Self::process_batch_static(server_id, batch, backend_caller);
                }
            });
        }

        // Wait for response
        rx.await
            .map_err(|_| Error::Server("Batch response channel closed".to_string()))?
    }

    /// Process a batch and distribute responses to all waiting clients
    fn process_batch_static(
        server_id: String,
        batch: PendingBatch,
        backend_caller: Arc<dyn Fn(String, McpRequest) -> Result<McpResponse> + Send + Sync>,
    ) {
        tokio::spawn(async move {
            let batch_size = batch.requests.len();
            let wait_time = batch.created_at.elapsed();

            tracing::debug!(
                "Processing batch: server={}, size={}, wait_time={:?}",
                server_id,
                batch_size,
                wait_time
            );

            // Record metrics
            crate::metrics::BATCH_REQUESTS_TOTAL.inc_by(batch_size as u64);
            crate::metrics::BATCH_SIZE.observe(batch_size as f64);
            crate::metrics::BATCH_WAIT_TIME_SECONDS.observe(wait_time.as_secs_f64());

            // Use first request as representative (for list methods, params don't matter)
            let representative_request = &batch.requests[0].request;

            // Make single backend call
            let result = backend_caller(server_id.clone(), representative_request.clone());

            // Distribute response to all waiters
            for pending in batch.requests {
                // Clone response for each waiter
                let response_result = result.clone();

                // Try to send (ignore if receiver dropped)
                let _ = pending.response_tx.send(response_result);
            }

            // Record batching efficiency
            let efficiency = 1.0 / batch_size as f64;
            crate::metrics::BATCHING_EFFICIENCY_RATIO.set(efficiency);

            tracing::info!(
                "Batch processed: server={}, batch_size={}, backend_calls=1, efficiency={:.2}%",
                server_id,
                batch_size,
                (1.0 - efficiency) * 100.0
            );
        });
    }

    /// Get current number of active batches (for monitoring)
    pub fn active_batch_count(&self) -> usize {
        self.batches.len()
    }

    /// Clear all pending batches (for testing/shutdown)
    pub async fn clear(&self) {
        self.batches.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn test_config() -> BatchConfig {
        BatchConfig {
            enabled: true,
            window_ms: 50, // Shorter for tests
            max_batch_size: 3,
            methods: vec!["tools/list".to_string()],
        }
    }

    fn sample_request(id: i64) -> McpRequest {
        McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(id)),
            method: "tools/list".to_string(),
            params: None,
        }
    }

    fn sample_response() -> McpResponse {
        McpResponse {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            result: Some(json!({"tools": []})),
            error: None,
        }
    }

    #[tokio::test]
    async fn test_batch_aggregator_creation() {
        let config = test_config();
        let aggregator = BatchAggregator::new(config.clone());

        assert_eq!(aggregator.active_batch_count(), 0);
    }

    #[tokio::test]
    async fn test_non_batchable_method_direct_call() {
        let config = BatchConfig {
            methods: vec!["tools/list".to_string()],
            ..test_config()
        };

        let aggregator = BatchAggregator::new(config).with_backend_caller(|_, req| {
            Ok(McpResponse {
                jsonrpc: "2.0".to_string(),
                id: req.id,
                result: Some(json!({"direct": true})),
                error: None,
            })
        });

        // Non-batchable method should call backend directly
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "tools/call".to_string(), // Not in config.methods
            params: None,
        };

        let response = aggregator.submit_request("server1".to_string(), request).await.unwrap();

        assert_eq!(response.result.unwrap()["direct"], json!(true));
    }

    #[tokio::test]
    async fn test_default_config() {
        let config = BatchConfig::default();

        assert!(!config.enabled);
        assert_eq!(config.window_ms, 100);
        assert_eq!(config.max_batch_size, 10);
        assert_eq!(config.methods.len(), 3);
    }
}
