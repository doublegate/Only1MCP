# Phase 2 Master Plan

**Created:** October 17, 2025
**Project:** Only1MCP - Rust MCP Server Aggregator
**Phase:** Phase 2 - Advanced Features (Weeks 5-8)
**Status:** 🚀 **READY TO START**

---

## 📋 Overview

Phase 2 builds upon the solid Phase 1 foundation to deliver advanced features that enhance performance, reliability, and observability. This phase focuses on completing the core infrastructure features that were scaffolded in Phase 1 and adding monitoring capabilities.

### Timeline

- **Start Date:** October 17, 2025
- **Target Completion:** Weeks 5-8 (4 weeks from Phase 1 completion)
- **Estimated Effort:** 46-60 hours total
- **Execution Model:** Continuous sub-agent chain (each feature spawns next)

### Success Criteria

- [ ] All 6 features fully implemented (no stubs)
- [ ] All tests passing (target: 50+ total tests)
- [ ] All features documented
- [ ] Performance targets met (<5ms latency, 10k+ req/s)
- [ ] Zero compilation errors
- [ ] Clippy warnings < 5
- [ ] Phase 2 completion report generated

---

## 🎯 Feature Breakdown

### Feature 1: Configuration Hot-Reload ⚡ CRITICAL

**Priority:** CRITICAL
**Complexity:** MEDIUM
**Dependencies:** None
**Estimated Time:** 6-8 hours
**Sub-agent Task:** "Implement configuration hot-reload with notify crate"

#### Description

Enable the proxy server to detect configuration file changes and reload settings without restart. This allows operators to add/remove backends, adjust routing algorithms, modify health check settings, and update security policies in real-time.

#### Technical Approach

**Library:** notify 6.1+ (file watching)
**Key Components:**
- File watcher using notify::RecommendedWatcher
- Configuration validator (reuse existing validation)
- ArcSwap for atomic config replacement
- tokio::sync::watch for reload signaling
- Error recovery (keep old config on validation failure)

#### Implementation Plan

**Step 1: Add notify dependency** (15 minutes)
```toml
# Cargo.toml
notify = "6.1"
```

**Step 2: Create ConfigLoader** (3 hours)
```rust
// src/config/loader.rs
use notify::{Watcher, RecursiveMode, Event, EventKind};
use arc_swap::ArcSwap;
use std::sync::Arc;
use std::path::PathBuf;

pub struct ConfigLoader {
    config_path: PathBuf,
    config: Arc<ArcSwap<Config>>,
    reload_tx: watch::Sender<()>,
    reload_rx: watch::Receiver<()>,
}

impl ConfigLoader {
    pub fn new(config_path: PathBuf) -> Result<Self, Error> {
        // Load initial config
        let initial_config = Config::from_file(&config_path)?;

        // Create ArcSwap for atomic updates
        let config = Arc::new(ArcSwap::from_pointee(initial_config));

        // Create watch channel for reload notifications
        let (reload_tx, reload_rx) = watch::channel(());

        Ok(Self {
            config_path,
            config,
            reload_tx,
            reload_rx,
        })
    }

    pub async fn watch(&self) -> Result<(), Error> {
        // Create file watcher
        let (tx, rx) = std::sync::mpsc::channel();
        let mut watcher = notify::recommended_watcher(tx)?;

        // Watch config file
        watcher.watch(&self.config_path, RecursiveMode::NonRecursive)?;

        // Event loop
        loop {
            match rx.recv() {
                Ok(Ok(Event { kind: EventKind::Modify(_), .. })) => {
                    if let Err(e) = self.reload_config().await {
                        tracing::error!("Config reload failed: {}", e);
                    }
                }
                Ok(Err(e)) => tracing::warn!("Watch error: {}", e),
                Err(e) => {
                    tracing::error!("Channel error: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }

    async fn reload_config(&self) -> Result<(), Error> {
        // Read new config
        let new_config = Config::from_file(&self.config_path)?;

        // Validate (reuse existing validation)
        new_config.validate()?;

        // Atomic swap
        self.config.store(Arc::new(new_config));

        // Signal reload
        let _ = self.reload_tx.send(());

        tracing::info!("Configuration reloaded successfully");
        Ok(())
    }

    pub fn get_config(&self) -> Arc<Config> {
        self.config.load_full()
    }

    pub fn subscribe_reloads(&self) -> watch::Receiver<()> {
        self.reload_rx.clone()
    }
}
```

**Step 3: Integrate with ProxyServer** (2 hours)
```rust
// src/proxy/server.rs
pub async fn run_with_hot_reload(config_path: PathBuf) -> Result<(), Error> {
    let loader = ConfigLoader::new(config_path)?;
    let config = loader.get_config();

    // Spawn watcher task
    let watcher_handle = tokio::spawn({
        let loader = loader.clone();
        async move {
            if let Err(e) = loader.watch().await {
                tracing::error!("Config watcher failed: {}", e);
            }
        }
    });

    // Start server
    let server = Self::new(config).await?;

    // Listen for reload signals
    let mut reload_rx = loader.subscribe_reloads();
    let server_handle = tokio::spawn({
        let server = server.clone();
        async move {
            loop {
                tokio::select! {
                    _ = reload_rx.changed() => {
                        tracing::info!("Config changed, applying updates...");
                        let new_config = loader.get_config();
                        server.update_config(new_config).await;
                    }
                }
            }
        }
    });

    // Run server
    server.run().await?;

    // Cleanup
    watcher_handle.abort();
    server_handle.abort();

    Ok(())
}
```

**Step 4: Write tests** (1 hour)
```rust
// tests/config_reload.rs
#[tokio::test]
async fn test_config_hot_reload() {
    // Create temp config file
    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join("config.yaml");

    // Write initial config
    std::fs::write(&config_path, initial_config_yaml()).unwrap();

    // Start loader
    let loader = ConfigLoader::new(config_path.clone()).unwrap();
    tokio::spawn(async move { loader.watch().await });

    // Verify initial config
    let config = loader.get_config();
    assert_eq!(config.servers.len(), 1);

    // Modify config file
    std::fs::write(&config_path, updated_config_yaml()).unwrap();

    // Wait for reload
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Verify new config loaded
    let new_config = loader.get_config();
    assert_eq!(new_config.servers.len(), 2);
}

#[tokio::test]
async fn test_invalid_config_keeps_old() {
    // Test that invalid config doesn't replace valid one
}

#[tokio::test]
async fn test_reload_notifications() {
    // Test that reload_rx receives notifications
}
```

**Step 5: Documentation** (30 minutes)
- Update README.md with hot-reload section
- Create docs/configuration_hot_reload.md guide
- Update CHANGELOG.md

#### Testing Requirements

- [ ] Config file modification detected
- [ ] Valid config reloaded successfully
- [ ] Invalid config rejected (old config retained)
- [ ] Reload notifications sent
- [ ] No memory leaks in watcher
- [ ] Platform compatibility (Linux, macOS, Windows)

#### Documentation Requirements

- [ ] README.md updated with hot-reload feature
- [ ] docs/configuration_hot_reload.md created
- [ ] CHANGELOG.md entry added
- [ ] Code comments for public API

#### Edge Cases to Handle

- Config file deleted (use last valid config)
- Config file permissions changed (handle error gracefully)
- Rapid successive changes (debounce with 100ms window)
- Invalid YAML/TOML syntax (parse errors logged, old config kept)
- Partial file writes (wait for complete write before parsing)

---

### Feature 2: Active Health Checking 🏥 CRITICAL

**Priority:** CRITICAL
**Complexity:** MEDIUM
**Dependencies:** None (circuit breaker already implemented)
**Estimated Time:** 6-8 hours
**Sub-agent Task:** "Implement active health checking with timer-based probes"

#### Description

Implement timer-based active health probes that periodically check backend health via HTTP requests or process status checks. This complements the existing circuit breaker (passive monitoring) with proactive health detection.

#### Technical Approach

**Library:** tokio::time for intervals
**Key Components:**
- Per-backend health probe tasks
- HTTP health check endpoints (e.g., /health)
- STDIO process liveness checks
- Configurable intervals and timeouts
- Integration with existing circuit breaker

#### Implementation Plan

**Step 1: Extend HealthChecker** (3 hours)
```rust
// src/health/checker.rs (expand existing stub)
use tokio::time::{interval, Duration};
use std::collections::HashMap;

pub struct ActiveHealthChecker {
    probes: Arc<DashMap<ServerId, ProbeTask>>,
    config: Arc<HealthCheckConfig>,
    circuit_breakers: Arc<CircuitBreakerManager>,
}

struct ProbeTask {
    server_id: ServerId,
    handle: JoinHandle<()>,
    cancel_token: CancellationToken,
}

impl ActiveHealthChecker {
    pub fn new(
        config: Arc<HealthCheckConfig>,
        circuit_breakers: Arc<CircuitBreakerManager>,
    ) -> Self {
        Self {
            probes: Arc::new(DashMap::new()),
            config,
            circuit_breakers,
        }
    }

    pub async fn start_probe(&self, server: &ServerConfig) {
        let server_id = server.id.clone();
        let cancel_token = CancellationToken::new();

        let handle = tokio::spawn({
            let server = server.clone();
            let config = self.config.clone();
            let circuit_breakers = self.circuit_breakers.clone();
            let cancel = cancel_token.clone();

            async move {
                let mut interval = interval(Duration::from_secs(config.probe_interval));

                loop {
                    tokio::select! {
                        _ = interval.tick() => {
                            match probe_backend(&server, &config).await {
                                Ok(healthy) => {
                                    if healthy {
                                        circuit_breakers.record_success(&server.id);
                                    } else {
                                        circuit_breakers.record_failure(&server.id);
                                    }
                                }
                                Err(e) => {
                                    tracing::warn!("Health probe failed for {}: {}", server.id, e);
                                    circuit_breakers.record_failure(&server.id);
                                }
                            }
                        }
                        _ = cancel.cancelled() => {
                            tracing::info!("Health probe cancelled for {}", server.id);
                            break;
                        }
                    }
                }
            }
        });

        self.probes.insert(
            server_id,
            ProbeTask {
                server_id: server.id.clone(),
                handle,
                cancel_token,
            },
        );
    }

    pub async fn stop_probe(&self, server_id: &ServerId) {
        if let Some((_, probe)) = self.probes.remove(server_id) {
            probe.cancel_token.cancel();
            let _ = probe.handle.await;
        }
    }
}

async fn probe_backend(
    server: &ServerConfig,
    config: &HealthCheckConfig,
) -> Result<bool, Error> {
    match &server.transport {
        TransportType::Http { endpoint } => {
            // HTTP health check
            let client = reqwest::Client::builder()
                .timeout(Duration::from_secs(config.timeout))
                .build()?;

            let health_url = format!("{}/health", endpoint);
            let response = client.get(&health_url).send().await?;

            Ok(response.status().is_success())
        }
        TransportType::Stdio { command, .. } => {
            // Check if process is alive (simplified)
            // In production, might send a test request
            Ok(true) // Process management handles this
        }
        _ => Ok(true), // Other transports assume healthy
    }
}
```

**Step 2: Integrate with ProxyServer** (2 hours)
```rust
// src/proxy/server.rs
impl ProxyServer {
    pub async fn start_health_monitoring(&self) -> Result<(), Error> {
        let checker = ActiveHealthChecker::new(
            self.config.health.clone(),
            self.circuit_breakers.clone(),
        );

        // Start probes for all configured servers
        for server in &self.config.servers {
            checker.start_probe(server).await;
        }

        // Store checker for lifecycle management
        self.health_checker = Some(checker);

        Ok(())
    }
}
```

**Step 3: Configuration schema** (1 hour)
```yaml
# config/templates/solo.yaml
health:
  probe_interval: 30  # seconds
  timeout: 5          # seconds
  failure_threshold: 3
  success_threshold: 2
  endpoints:
    - path: /health
      method: GET
      expected_status: 200
```

**Step 4: Write tests** (2 hours)
```rust
// tests/active_health.rs
#[tokio::test]
async fn test_health_probe_detects_healthy() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/health"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock)
        .await;

    // Start probe
    let checker = ActiveHealthChecker::new(test_config(), circuit_breakers());
    checker.start_probe(&mock_server_config(mock.uri())).await;

    // Wait for probe
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Verify healthy
    assert_eq!(circuit_breakers().get_state("server1"), CircuitState::Closed);
}

#[tokio::test]
async fn test_health_probe_detects_unhealthy() {
    // Test 500 status code marks backend unhealthy
}

#[tokio::test]
async fn test_health_probe_timeout() {
    // Test timeout triggers failure
}

#[tokio::test]
async fn test_probe_cancellation() {
    // Test probe can be stopped cleanly
}
```

**Step 5: Documentation** (30 minutes)

#### Testing Requirements

- [ ] Healthy backend detected correctly
- [ ] Unhealthy backend (500 status) detected
- [ ] Timeout triggers failure
- [ ] Probe cancellation works cleanly
- [ ] Circuit breaker integration working
- [ ] No resource leaks from probe tasks

#### Documentation Requirements

- [ ] docs/health_checking.md created
- [ ] Configuration guide updated
- [ ] CHANGELOG.md entry
- [ ] README.md health section

---

### Feature 3: Response Caching (TTL + LRU) 💾 HIGH

**Priority:** HIGH
**Complexity:** MEDIUM-HIGH
**Dependencies:** None (cache framework exists)
**Estimated Time:** 8-10 hours
**Sub-agent Task:** "Implement TTL-based LRU response cache"

#### Description

Complete the response caching system by implementing Time-To-Live (TTL) expiration and Least Recently Used (LRU) eviction policies. This reduces backend load and improves response times for frequently requested data.

#### Technical Approach

**Library:** moka 0.12+ (high-performance cache with TTL/LRU)
**Alternative:** mini-moka (lighter weight) or manual implementation
**Key Components:**
- TTL-based expiration per entry
- LRU eviction on capacity limits
- Cache key generation (blake3 hash)
- Per-method cache configuration
- Cache metrics (hits, misses, evictions)

#### Implementation Plan

**Step 1: Add moka dependency** (15 minutes)
```toml
# Cargo.toml
moka = { version = "0.12", features = ["future"] }
```

**Step 2: Implement ResponseCache** (4 hours)
```rust
// src/cache/mod.rs (replace existing DashMap-based cache)
use moka::future::Cache;
use std::time::Duration;

pub struct ResponseCache {
    // Separate caches for different MCP methods
    tools_cache: Cache<String, ToolsListResponse>,
    resources_cache: Cache<String, ResourcesListResponse>,
    prompts_cache: Cache<String, PromptsListResponse>,
    config: CacheConfig,
    metrics: CacheMetrics,
}

impl ResponseCache {
    pub fn new(config: CacheConfig) -> Self {
        let tools_cache = Cache::builder()
            .max_capacity(config.max_entries_per_method)
            .time_to_live(Duration::from_secs(config.ttl_seconds))
            .build();

        let resources_cache = Cache::builder()
            .max_capacity(config.max_entries_per_method)
            .time_to_live(Duration::from_secs(config.ttl_seconds))
            .build();

        let prompts_cache = Cache::builder()
            .max_capacity(config.max_entries_per_method)
            .time_to_live(Duration::from_secs(config.ttl_seconds))
            .build();

        Self {
            tools_cache,
            resources_cache,
            prompts_cache,
            config,
            metrics: CacheMetrics::default(),
        }
    }

    pub async fn get_tools(&self, key: &str) -> Option<ToolsListResponse> {
        match self.tools_cache.get(key).await {
            Some(value) => {
                self.metrics.record_hit("tools");
                Some(value)
            }
            None => {
                self.metrics.record_miss("tools");
                None
            }
        }
    }

    pub async fn set_tools(&self, key: String, value: ToolsListResponse) {
        self.tools_cache.insert(key, value).await;
    }

    // Similar for resources and prompts...

    pub fn generate_key(server_id: &str, method: &str, params: &Value) -> String {
        use blake3::Hasher;
        let mut hasher = Hasher::new();
        hasher.update(server_id.as_bytes());
        hasher.update(method.as_bytes());
        hasher.update(&serde_json::to_vec(params).unwrap_or_default());
        format!("{:x}", hasher.finalize())
    }

    pub async fn invalidate_server(&self, server_id: &str) {
        // Invalidate all cache entries for a specific server
        self.tools_cache.invalidate_all().await;
        self.resources_cache.invalidate_all().await;
        self.prompts_cache.invalidate_all().await;
    }

    pub fn stats(&self) -> CacheStats {
        CacheStats {
            tools_size: self.tools_cache.entry_count(),
            resources_size: self.resources_cache.entry_count(),
            prompts_size: self.prompts_cache.entry_count(),
            hit_rate: self.metrics.hit_rate(),
            miss_rate: self.metrics.miss_rate(),
        }
    }
}
```

**Step 3: Integrate with handlers** (2 hours)
```rust
// src/proxy/handler.rs
async fn handle_tools_list(
    state: &ProxyState,
    request: &McpRequest,
) -> Result<McpResponse, Error> {
    // Generate cache key
    let cache_key = ResponseCache::generate_key(
        "aggregated",
        "tools/list",
        &request.params,
    );

    // Try cache first
    if let Some(cached) = state.cache.get_tools(&cache_key).await {
        tracing::debug!("Cache hit for tools/list");
        return Ok(McpResponse::success(request.id.clone(), cached));
    }

    // Cache miss - fetch from backends
    let response = fetch_tools_from_servers(state).await?;

    // Store in cache
    state.cache.set_tools(cache_key, response.clone()).await;

    Ok(McpResponse::success(request.id.clone(), response))
}
```

**Step 4: Configuration** (1 hour)
```yaml
# config/templates/solo.yaml
cache:
  enabled: true
  ttl_seconds: 300  # 5 minutes
  max_entries_per_method: 1000
  methods:
    tools/list:
      ttl_seconds: 600  # 10 minutes (tools change rarely)
    resources/list:
      ttl_seconds: 300  # 5 minutes
    prompts/list:
      ttl_seconds: 600  # 10 minutes
```

**Step 5: Write tests** (2 hours)
```rust
// tests/cache.rs
#[tokio::test]
async fn test_cache_hit_after_set() {
    let cache = ResponseCache::new(test_config());

    // Set value
    cache.set_tools("key1".into(), sample_tools_response()).await;

    // Get value
    let cached = cache.get_tools("key1").await.unwrap();
    assert_eq!(cached.tools.len(), 2);
}

#[tokio::test]
async fn test_cache_ttl_expiration() {
    let cache = ResponseCache::new(CacheConfig {
        ttl_seconds: 1, // 1 second TTL
        ..test_config()
    });

    cache.set_tools("key1".into(), sample_tools_response()).await;

    // Immediate fetch - should hit
    assert!(cache.get_tools("key1").await.is_some());

    // Wait for expiration
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Should be expired
    assert!(cache.get_tools("key1").await.is_none());
}

#[tokio::test]
async fn test_cache_lru_eviction() {
    let cache = ResponseCache::new(CacheConfig {
        max_entries_per_method: 2, // Only 2 entries
        ..test_config()
    });

    // Fill cache
    cache.set_tools("key1".into(), sample_tools_response()).await;
    cache.set_tools("key2".into(), sample_tools_response()).await;

    // Access key1 to make it recently used
    let _ = cache.get_tools("key1").await;

    // Add key3 (should evict key2 as least recently used)
    cache.set_tools("key3".into(), sample_tools_response()).await;

    // key1 should still exist, key2 should be evicted
    assert!(cache.get_tools("key1").await.is_some());
    assert!(cache.get_tools("key2").await.is_none());
    assert!(cache.get_tools("key3").await.is_some());
}

#[tokio::test]
async fn test_cache_metrics() {
    let cache = ResponseCache::new(test_config());
    cache.set_tools("key1".into(), sample_tools_response()).await;

    // Hit
    let _ = cache.get_tools("key1").await;

    // Miss
    let _ = cache.get_tools("key2").await;

    let stats = cache.stats();
    assert_eq!(stats.hit_rate, 0.5);
}
```

**Step 6: Documentation** (30 minutes)

#### Testing Requirements

- [ ] Cache hit returns correct value
- [ ] Cache miss returns None
- [ ] TTL expiration works correctly
- [ ] LRU eviction on capacity limit
- [ ] Cache invalidation by server
- [ ] Metrics (hit rate, miss rate) accurate
- [ ] Concurrent access safe

#### Documentation Requirements

- [ ] docs/caching_guide.md created
- [ ] Configuration examples
- [ ] Performance benchmarks
- [ ] CHANGELOG.md entry

---

### Feature 4: Request Batching 📦 HIGH

**Priority:** HIGH
**Complexity:** MEDIUM-HIGH
**Dependencies:** None
**Estimated Time:** 8-10 hours
**Sub-agent Task:** "Implement request batching with 100ms windows"

#### Description

Implement request batching to aggregate multiple similar requests into a single backend call within a time window. This is a core value proposition feature that enables 50-70% context reduction.

#### Technical Approach

**Pattern:** Tokio time-based batching
**Key Components:**
- 100ms batching windows
- Request aggregation by method and target server
- Batch flushing on timeout or size limit
- Response distribution to waiting clients
- Configurable batch size and timeout

#### Implementation Plan

**Step 1: Create BatchAggregator** (5 hours)
```rust
// src/batching/mod.rs (new module)
use tokio::time::{sleep, Duration, Instant};
use tokio::sync::mpsc;

pub struct BatchAggregator {
    batches: Arc<DashMap<BatchKey, PendingBatch>>,
    config: BatchConfig,
}

#[derive(Hash, Eq, PartialEq)]
struct BatchKey {
    server_id: ServerId,
    method: String,
}

struct PendingBatch {
    requests: Vec<PendingRequest>,
    deadline: Instant,
    responders: Vec<oneshot::Sender<McpResponse>>,
}

struct PendingRequest {
    request: McpRequest,
    response_tx: oneshot::Sender<McpResponse>,
}

impl BatchAggregator {
    pub fn new(config: BatchConfig) -> Self {
        Self {
            batches: Arc::new(DashMap::new()),
            config,
        }
    }

    pub async fn submit_request(
        &self,
        server_id: ServerId,
        request: McpRequest,
    ) -> Result<McpResponse, Error> {
        let key = BatchKey {
            server_id: server_id.clone(),
            method: request.method.clone(),
        };

        let (tx, rx) = oneshot::channel();

        // Add to batch
        let mut batch = self.batches.entry(key.clone())
            .or_insert_with(|| PendingBatch {
                requests: Vec::new(),
                deadline: Instant::now() + Duration::from_millis(self.config.window_ms),
                responders: Vec::new(),
            });

        batch.requests.push(PendingRequest {
            request: request.clone(),
            response_tx: tx,
        });

        // Check if batch is full
        if batch.requests.len() >= self.config.max_batch_size {
            self.flush_batch(key).await?;
        } else if batch.requests.len() == 1 {
            // First request in batch - start timer
            let batches = self.batches.clone();
            let config = self.config.clone();
            tokio::spawn(async move {
                sleep(Duration::from_millis(config.window_ms)).await;
                if let Some((_, batch)) = batches.remove(&key) {
                    let _ = Self::process_batch(server_id, batch).await;
                }
            });
        }

        // Wait for response
        rx.await.map_err(|_| Error::BatchingError("Response channel closed".into()))
    }

    async fn flush_batch(&self, key: BatchKey) -> Result<(), Error> {
        if let Some((_, batch)) = self.batches.remove(&key) {
            Self::process_batch(key.server_id, batch).await
        } else {
            Ok(())
        }
    }

    async fn process_batch(
        server_id: ServerId,
        batch: PendingBatch,
    ) -> Result<(), Error> {
        // Aggregate requests into single backend call
        let aggregated_request = aggregate_requests(&batch.requests);

        // Send to backend
        let response = send_to_backend(&server_id, aggregated_request).await?;

        // Distribute responses
        distribute_responses(batch.responders, response);

        Ok(())
    }
}

fn aggregate_requests(requests: &[PendingRequest]) -> McpRequest {
    // For tools/list, resources/list, prompts/list - single call covers all
    // For tools/call - batch multiple tool calls if backend supports it
    // Implementation depends on MCP protocol support for batching
    todo!("Aggregate based on method type")
}
```

**Step 2: Integrate with handler** (2 hours)
```rust
// src/proxy/handler.rs
async fn handle_request_with_batching(
    state: &ProxyState,
    request: McpRequest,
) -> Result<McpResponse, Error> {
    if state.config.batching.enabled {
        state.batch_aggregator.submit_request(
            select_server(&state, &request)?,
            request,
        ).await
    } else {
        // Direct request without batching
        handle_request_direct(state, request).await
    }
}
```

**Step 3: Configuration** (1 hour)
```yaml
# config/templates/solo.yaml
batching:
  enabled: true
  window_ms: 100  # 100ms batching window
  max_batch_size: 10
  methods:
    - tools/list
    - resources/list
    - prompts/list
```

**Step 4: Write tests** (2 hours)
```rust
// tests/batching.rs
#[tokio::test]
async fn test_batch_aggregation() {
    let aggregator = BatchAggregator::new(test_config());

    // Submit multiple requests simultaneously
    let mut handles = vec![];
    for i in 0..5 {
        let req = sample_request(i);
        let agg = aggregator.clone();
        handles.push(tokio::spawn(async move {
            agg.submit_request("server1".into(), req).await
        }));
    }

    // All should complete
    for handle in handles {
        assert!(handle.await.is_ok());
    }

    // Verify only 1 backend call made (via metrics)
}

#[tokio::test]
async fn test_batch_timeout() {
    // Test that batch flushes after 100ms
}

#[tokio::test]
async fn test_batch_size_limit() {
    // Test that batch flushes when max_batch_size reached
}
```

**Step 5: Documentation** (30 minutes)

#### Testing Requirements

- [ ] Multiple requests batched correctly
- [ ] Batch flushes on timeout (100ms)
- [ ] Batch flushes on size limit
- [ ] Responses distributed correctly
- [ ] Partial failures handled
- [ ] Metrics track batching efficiency

---

### Feature 5: TUI Interface 🖥️ MEDIUM

**Priority:** MEDIUM
**Complexity:** HIGH
**Dependencies:** All above features (for monitoring)
**Estimated Time:** 12-16 hours
**Sub-agent Task:** "Implement TUI interface with ratatui"

#### Description

Build a terminal user interface for real-time monitoring and debugging of the proxy server. Display server status, request metrics, cache statistics, and health checks in an interactive dashboard.

#### Technical Approach

**Library:** ratatui 0.28+ (terminal UI framework)
**Key Components:**
- Real-time metrics dashboard
- Server list with health status
- Request log viewer
- Cache statistics panel
- Interactive navigation (arrow keys, vim bindings)

#### Implementation Plan

**Step 1: Add ratatui dependency** (15 minutes)
```toml
# Cargo.toml
ratatui = "0.28"
crossterm = "0.28"
```

**Step 2: Create TUI app structure** (6 hours)
```rust
// src/tui/mod.rs (new module)
use ratatui::{
    backend::CrosstermBackend,
    widgets::{Block, Borders, List, ListItem, Gauge, Paragraph},
    layout::{Layout, Constraint, Direction},
    Terminal,
};

pub struct TuiApp {
    state: Arc<ProxyState>,
    selected_tab: usize,
    should_quit: bool,
}

impl TuiApp {
    pub async fn run(state: Arc<ProxyState>) -> Result<(), Error> {
        // Setup terminal
        let mut terminal = Terminal::new(CrosstermBackend::new(std::io::stdout()))?;
        terminal.clear()?;

        let mut app = TuiApp {
            state,
            selected_tab: 0,
            should_quit: false,
        };

        // Event loop
        loop {
            terminal.draw(|f| app.draw(f))?;

            if app.should_quit {
                break;
            }

            app.handle_events().await?;
        }

        Ok(())
    }

    fn draw(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Header
                Constraint::Min(0),     // Content
                Constraint::Length(3),  // Footer
            ])
            .split(f.area());

        // Draw header
        self.draw_header(f, chunks[0]);

        // Draw content based on selected tab
        match self.selected_tab {
            0 => self.draw_overview(f, chunks[1]),
            1 => self.draw_servers(f, chunks[1]),
            2 => self.draw_metrics(f, chunks[1]),
            3 => self.draw_logs(f, chunks[1]),
            _ => {}
        }

        // Draw footer
        self.draw_footer(f, chunks[2]);
    }

    fn draw_overview(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(50),
                Constraint::Percentage(50),
            ])
            .split(area);

        // Server status panel
        let server_count = self.state.registry.server_count();
        let healthy_count = self.state.circuit_breakers.healthy_count();

        let server_block = Block::default()
            .title("Servers")
            .borders(Borders::ALL);

        let server_text = format!(
            "Total: {}\nHealthy: {}\nUnhealthy: {}",
            server_count,
            healthy_count,
            server_count - healthy_count
        );

        let server_widget = Paragraph::new(server_text)
            .block(server_block);

        f.render_widget(server_widget, chunks[0]);

        // Cache stats panel
        let cache_stats = self.state.cache.stats();
        let cache_block = Block::default()
            .title("Cache")
            .borders(Borders::ALL);

        let cache_text = format!(
            "Hit Rate: {:.1}%\nTotal Entries: {}\nTools: {}\nResources: {}",
            cache_stats.hit_rate * 100.0,
            cache_stats.total_entries(),
            cache_stats.tools_size,
            cache_stats.resources_size
        );

        let cache_widget = Paragraph::new(cache_text)
            .block(cache_block);

        f.render_widget(cache_widget, chunks[1]);
    }

    async fn handle_events(&mut self) -> Result<(), Error> {
        use crossterm::event::{self, Event, KeyCode};

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => self.should_quit = true,
                    KeyCode::Char('1') => self.selected_tab = 0,
                    KeyCode::Char('2') => self.selected_tab = 1,
                    KeyCode::Char('3') => self.selected_tab = 2,
                    KeyCode::Char('4') => self.selected_tab = 3,
                    KeyCode::Left => {
                        if self.selected_tab > 0 {
                            self.selected_tab -= 1;
                        }
                    }
                    KeyCode::Right => {
                        if self.selected_tab < 3 {
                            self.selected_tab += 1;
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(())
    }
}
```

**Step 3: CLI integration** (2 hours)
```rust
// src/main.rs
#[derive(Subcommand)]
enum Command {
    Start {
        #[arg(long)]
        host: String,
        #[arg(long)]
        port: u16,
        #[arg(long)]
        tui: bool,  // Enable TUI mode
    },
    Monitor {
        // TUI-only monitoring mode
    },
}
```

**Step 4: Write tests** (2 hours)
```rust
// tests/tui.rs
#[tokio::test]
async fn test_tui_renders() {
    // Test that TUI renders without panicking
}

#[tokio::test]
async fn test_tui_navigation() {
    // Test keyboard navigation between tabs
}
```

**Step 5: Documentation** (2 hours)
- Create docs/tui_guide.md
- Add screenshots (use `vhs` for terminal recordings)
- Update README.md

#### Testing Requirements

- [ ] TUI renders without errors
- [ ] Navigation works (arrow keys, numbers)
- [ ] Real-time updates visible
- [ ] Graceful shutdown on 'q'
- [ ] Cross-platform compatibility

---

### Feature 6: Performance Benchmarking Suite ⚡ MEDIUM

**Priority:** MEDIUM
**Complexity:** MEDIUM
**Dependencies:** All above features (for accurate benchmarking)
**Estimated Time:** 6-8 hours
**Sub-agent Task:** "Implement criterion benchmarks for performance validation"

#### Description

Create a comprehensive benchmarking suite using criterion.rs to measure and track proxy performance over time. Validate that the system meets the <5ms latency and 10k+ req/s throughput targets.

#### Technical Approach

**Library:** criterion 0.5+ (statistical benchmarking)
**Key Metrics:**
- Proxy overhead latency (p50, p99, p999)
- Throughput (requests per second)
- Memory usage per connection
- Cache hit/miss performance
- Load balancer selection time

#### Implementation Plan

**Step 1: Add criterion dependency** (15 minutes)
```toml
# Cargo.toml
[dev-dependencies]
criterion = { version = "0.5", features = ["async_tokio", "html_reports"] }

[[bench]]
name = "proxy_benchmarks"
harness = false
```

**Step 2: Create benchmark suite** (4 hours)
```rust
// benches/proxy_benchmarks.rs
use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};
use only1mcp::*;

fn bench_proxy_overhead(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("proxy_overhead", |b| {
        b.to_async(&rt).iter(|| async {
            let request = sample_mcp_request();
            let start = Instant::now();
            let _ = proxy_request(request).await;
            start.elapsed()
        })
    });
}

fn bench_load_balancer(c: &mut Criterion) {
    let mut group = c.benchmark_group("load_balancer");

    for algorithm in &["round_robin", "least_conn", "consistent_hash"] {
        group.bench_with_input(
            BenchmarkId::from_parameter(algorithm),
            algorithm,
            |b, &algo| {
                let lb = LoadBalancer::new(algo, test_servers());
                b.iter(|| {
                    lb.select_server("session123")
                });
            },
        );
    }

    group.finish();
}

fn bench_cache_performance(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let cache = ResponseCache::new(test_config());

    let mut group = c.benchmark_group("cache");

    group.bench_function("cache_set", |b| {
        b.to_async(&rt).iter(|| async {
            cache.set_tools("key".into(), sample_tools()).await;
        })
    });

    group.bench_function("cache_get_hit", |b| {
        // Pre-populate
        rt.block_on(cache.set_tools("key".into(), sample_tools()));

        b.to_async(&rt).iter(|| async {
            cache.get_tools("key").await
        })
    });

    group.bench_function("cache_get_miss", |b| {
        b.to_async(&rt).iter(|| async {
            cache.get_tools("nonexistent").await
        })
    });

    group.finish();
}

fn bench_throughput(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("throughput_1000_requests", |b| {
        b.to_async(&rt).iter(|| async {
            let mut handles = vec![];
            for _ in 0..1000 {
                handles.push(tokio::spawn(async {
                    proxy_request(sample_mcp_request()).await
                }));
            }
            for handle in handles {
                let _ = handle.await;
            }
        })
    });
}

criterion_group!(
    benches,
    bench_proxy_overhead,
    bench_load_balancer,
    bench_cache_performance,
    bench_throughput
);
criterion_main!(benches);
```

**Step 3: CI integration** (1 hour)
```yaml
# .github/workflows/benchmark.yml
name: Benchmark

on:
  push:
    branches: [main]
  pull_request:

jobs:
  benchmark:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo bench --bench proxy_benchmarks
      - uses: benchmark-action/github-action-benchmark@v1
        with:
          tool: 'cargo'
          output-file-path: target/criterion/*/new/estimates.json
```

**Step 4: Documentation** (2 hours)
- Create docs/performance_guide.md
- Document benchmarking methodology
- Establish performance baselines
- Update CHANGELOG.md

**Step 5: Performance validation** (1 hour)
```bash
# Run benchmarks
cargo bench

# Expected results:
# proxy_overhead: <5ms (p99)
# throughput_1000_requests: >10,000 req/s
# cache_get_hit: <100µs
# load_balancer selection: <10µs
```

#### Testing Requirements

- [ ] All benchmarks run without errors
- [ ] Results reproducible
- [ ] HTML reports generated
- [ ] CI integration working
- [ ] Performance baselines established

---

## 📊 Implementation Order & Timeline

### Sprint 1 (Weeks 5-6): Core Infrastructure

**Week 5:**
- Feature 1: Config hot-reload (6-8 hours)
- Feature 2: Active health checking (6-8 hours)
- **Goal:** 12-16 hours, 2 critical features complete

**Week 6:**
- Feature 3: Response caching (8-10 hours)
- **Goal:** 8-10 hours, high-priority feature complete
- **Milestone:** Production-ready caching and monitoring

### Sprint 2 (Weeks 7-8): Advanced Features

**Week 7:**
- Feature 4: Request batching (8-10 hours)
- **Goal:** 8-10 hours, high-priority feature complete
- **Milestone:** Context reduction feature operational

**Week 8:**
- Feature 5: TUI interface (12-16 hours)
- Feature 6: Performance benchmarks (6-8 hours)
- **Goal:** 18-24 hours, all features complete
- **Milestone:** Phase 2 100% complete

### Total Timeline

- **Weeks:** 4 (Weeks 5-8)
- **Hours:** 46-60 total
- **Pace:** 10-15 hours/week
- **Completion Target:** End of Week 8

---

## ✅ Phase 2 Completion Criteria

### Build & Code Quality

- [ ] `cargo build --release` succeeds (0 errors)
- [ ] `cargo clippy` passes (< 5 warnings)
- [ ] `cargo fmt --check` passes
- [ ] All new modules compile successfully

### Testing

- [ ] `cargo test` 100% pass rate (target: 50+ total tests)
- [ ] Each feature has integration tests
- [ ] All edge cases covered
- [ ] Performance benchmarks run successfully

### Features

- [ ] Feature 1: Config hot-reload working ✅
- [ ] Feature 2: Active health checking operational ✅
- [ ] Feature 3: Response caching (TTL + LRU) complete ✅
- [ ] Feature 4: Request batching functional ✅
- [ ] Feature 5: TUI interface usable ✅
- [ ] Feature 6: Benchmark suite established ✅

### Documentation

- [ ] All features documented in docs/
- [ ] README.md updated with Phase 2 features
- [ ] CHANGELOG.md v0.2.0-dev entry complete
- [ ] Configuration guide updated
- [ ] API reference updated

### Performance

- [ ] Latency overhead: <5ms (p99) verified
- [ ] Throughput: 10,000+ req/s measured
- [ ] Memory usage: <100MB for 100 backends tested
- [ ] Cache hit rate: >50% in typical scenarios

### Deliverables

- [ ] Phase 2 completion report generated
- [ ] CLAUDE.local.md updated to Phase 2 status
- [ ] All to-dos closed
- [ ] Git history clean (proper commits)

---

## 🚀 Execution Model: Continuous Sub-Agent Chain

### How It Works

1. **Current Sub-Agent**: Completes Feature N to 100%
2. **Validation**: Verify tests passing, docs updated
3. **Memory Update**: Record completion in Memory entities
4. **Status Update**: Update CLAUDE.local.md
5. **Next Sub-Agent**: Spawn new sub-agent for Feature N+1
6. **Continue**: Repeat until all 6 features complete

### Sub-Agent Spawn Pattern

```
Feature N Complete →
  Validate (tests, docs, build) →
    Update Memory →
      Update CLAUDE.local.md →
        Spawn Sub-Agent for Feature N+1 →
          Pass context (Phase 2 plan, previous completions) →
            New sub-agent starts Feature N+1
```

### Context Passed to Each Sub-Agent

```markdown
Continue Phase 2 development for Only1MCP.

Previous sub-agent completed: Feature N - [Name] ✅

Your task: Implement Feature N+1 - [Name]

Read the following for context:
- to-dos/Phase_2/PHASE_2_MASTER_PLAN.md (this file)
- to-dos/Phase_2/PHASE_1_ANALYSIS_REPORT.md
- CLAUDE.local.md (update with Feature N completion)

Use MCP servers extensively:
- Sequential Thinking for planning
- Context7 for [relevant crate] documentation
- Memory to track implementation progress

Implementation requirements:
- [Specific steps from this plan]
- Write tests for all functionality
- Update documentation
- No stubs or TODOs

When 100% complete, spawn next sub-agent for Feature N+2.
```

---

## 🎓 Lessons from Phase 1 (Apply to Phase 2)

### What Worked Well

1. **Test-Driven Development**: 27/27 tests passing - maintain this
2. **Documentation-First**: 5,000+ lines helped - continue pattern
3. **Systematic Approach**: Fixed issues in logical order - repeat
4. **Quality Over Speed**: 95% clippy compliance - maintain bar

### What to Improve

1. **Scope Management**: Phase 1 exceeded scope by 40% - stick to plan
2. **Test Coverage**: Proxy registry had only 1 test - ensure balanced coverage
3. **Documentation Lag**: Some features documented after implementation - document first

### Best Practices for Phase 2

1. **No Stubs**: Fully implement each feature before moving on
2. **Test Everything**: Unit + integration tests for each feature
3. **Document First**: Write docs before or during implementation
4. **Performance Validate**: Benchmark each feature impact
5. **Memory Efficient**: Profile memory usage throughout

---

## 📚 Resources & References

### Crate Documentation (Context7)

- **notify**: /notify-rs/notify (file watching)
- **ratatui**: /ratatui/ratatui (TUI framework)
- **criterion**: /bheisler/criterion.rs (benchmarking)
- **moka**: /moka-rs/moka (high-performance cache)

### Implementation Guides

- **Config Hot-Reload**: ref_docs/ (if exists)
- **Active Health**: Circuit breaker pattern already implemented
- **Caching**: DashMap foundation in src/cache/mod.rs
- **Batching**: Tokio time patterns
- **TUI**: ratatui examples
- **Benchmarking**: criterion.rs guide

### Testing Patterns

- **Integration Tests**: tests/server_startup.rs (model to follow)
- **Unit Tests**: src/auth/jwt.rs (comprehensive examples)
- **Mocking**: Wiremock integration (already in use)

---

## 📝 Progress Tracking

### Feature Status

| Feature | Status | Tests | Docs | Sub-Agent |
|---------|--------|-------|------|-----------|
| 1. Config Hot-Reload | ⏸️ Pending | ⏸️ | ⏸️ | Not started |
| 2. Active Health | ⏸️ Pending | ⏸️ | ⏸️ | Not started |
| 3. Response Caching | ⏸️ Pending | ⏸️ | ⏸️ | Not started |
| 4. Request Batching | ⏸️ Pending | ⏸️ | ⏸️ | Not started |
| 5. TUI Interface | ⏸️ Pending | ⏸️ | ⏸️ | Not started |
| 6. Benchmarking | ⏸️ Pending | ⏸️ | ⏸️ | Not started |

### Sprint Milestones

- [ ] **Sprint 1 Complete** (Week 6): Features 1-3 operational
- [ ] **Sprint 2 Complete** (Week 8): Features 4-6 operational
- [ ] **Phase 2 Complete** (Week 8): All features + tests + docs

### Weekly Goals

**Week 5:**
- [ ] Feature 1: Config hot-reload complete
- [ ] Feature 2: Active health checking complete
- [ ] Tests: +10 tests (37/50 total)

**Week 6:**
- [ ] Feature 3: Response caching complete
- [ ] Tests: +8 tests (45/50 total)
- [ ] Sprint 1 milestone reached

**Week 7:**
- [ ] Feature 4: Request batching complete
- [ ] Tests: +6 tests (51/50 total - exceeded target!)

**Week 8:**
- [ ] Feature 5: TUI interface complete
- [ ] Feature 6: Benchmarking suite complete
- [ ] Tests: +4 tests (55/50 total)
- [ ] Documentation: All guides updated
- [ ] Phase 2 completion report

---

## 🎉 Phase 2 Vision

At the end of Phase 2, Only1MCP will have:

✅ **Production-Ready Operations**
- Zero-downtime config updates
- Proactive health monitoring
- Intelligent response caching
- Efficient request batching

✅ **Exceptional Performance**
- <5ms latency verified by benchmarks
- 10,000+ req/s throughput measured
- 50-70% context reduction quantified
- <100MB memory footprint validated

✅ **Superior Observability**
- Real-time TUI dashboard
- Comprehensive metrics
- Performance benchmarks
- Health status visibility

✅ **Enterprise Quality**
- 50+ tests (100% pass rate)
- Complete documentation
- Performance validated
- Production deployment ready

---

**Ready to begin Phase 2!** 🚀

**Next Action:** Start Feature 1 (Configuration Hot-Reload) implementation.

---

*This master plan is a living document. Update as features complete and lessons learned emerge.*

**Last Updated:** October 17, 2025
**Status:** READY TO EXECUTE
**Estimated Completion:** End of Week 8 (4 weeks from start)
