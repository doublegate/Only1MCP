# Next Steps - Immediate Action Plan
## Only1MCP Development Priorities

**Generated:** October 14, 2025
**Based On:** Architecture Alignment Audit Results
**Priority:** High → Medium → Low
**Estimated Total Time:** 15-20 hours to Phase 1 MVP

---

## 🔴 CRITICAL - Do These First (6-7 hours)

### 1. Extract MCP Type Definitions ⏱️ 2 hours
**Why:** Multiple modules need these types, currently duplicated in transport/http.rs

**What to do:**
```bash
# Create/edit src/types/mod.rs
```

**Code to add:**
```rust
// src/types/mod.rs

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// JSON-RPC 2.0 request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpRequest {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    pub params: Option<Value>,
}

/// JSON-RPC 2.0 response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResponse {
    pub jsonrpc: String,
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<McpError>,
}

/// JSON-RPC 2.0 error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// MCP Tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub inputSchema: Value,
}

/// MCP Resource definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    pub uri: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mimeType: Option<String>,
}

/// MCP Prompt definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prompt {
    pub name: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Vec<PromptArgument>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptArgument {
    pub name: String,
    pub description: String,
    pub required: bool,
}
```

**Then update:**
```rust
// src/transport/http.rs
use crate::types::{McpRequest, McpResponse, McpError};

// Remove duplicate definitions from http.rs
```

**Files to update:**
- src/types/mod.rs (create types)
- src/transport/http.rs (remove duplicates, add imports)
- src/proxy/handler.rs (add imports)

**Validation:**
```bash
cargo check --lib
# Should compile without type definition errors
```

---

### 2. Add Metrics Declarations ⏱️ 1 hour
**Why:** Metrics endpoint won't work without these declarations

**What to do:**
```bash
# Edit src/metrics/mod.rs
```

**Code to add at the top of the file:**
```rust
use lazy_static::lazy_static;
use prometheus::{
    Counter, CounterVec, Gauge, GaugeVec, Histogram, HistogramVec,
    Registry, Opts, register_counter, register_histogram,
    register_gauge, register_counter_vec, register_gauge_vec,
};

lazy_static! {
    /// Total MCP requests received
    static ref MCP_REQUESTS_TOTAL: CounterVec = register_counter_vec!(
        "only1mcp_requests_total",
        "Total number of MCP requests",
        &["method", "status"]
    ).unwrap();

    /// MCP request duration in seconds
    static ref MCP_REQUEST_DURATION_SECONDS: HistogramVec = register_histogram_vec!(
        "only1mcp_request_duration_seconds",
        "MCP request duration in seconds",
        &["method"],
        vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0]
    ).unwrap();

    /// Context tokens saved
    static ref CONTEXT_TOKENS_SAVED: Counter = register_counter!(
        "only1mcp_context_tokens_saved_total",
        "Total context tokens saved through optimization"
    ).unwrap();

    /// Cache hit ratio
    static ref CONTEXT_CACHE_HIT_RATIO: Gauge = register_gauge!(
        "only1mcp_cache_hit_ratio",
        "Current cache hit ratio (0.0 to 1.0)"
    ).unwrap();

    /// Backend health status
    static ref BACKEND_HEALTH_STATUS: GaugeVec = register_gauge_vec!(
        "only1mcp_backend_health_status",
        "Backend server health (1=healthy, 0=unhealthy)",
        &["server_id"]
    ).unwrap();

    /// Backend latency in seconds
    static ref BACKEND_LATENCY_SECONDS: HistogramVec = register_histogram_vec!(
        "only1mcp_backend_latency_seconds",
        "Backend server latency in seconds",
        &["server_id"],
        vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0]
    ).unwrap();

    /// Connection pool size
    static ref CONNECTION_POOL_SIZE: GaugeVec = register_gauge_vec!(
        "only1mcp_connection_pool_size",
        "Connection pool size by state",
        &["server_id", "state"]  // state: active, idle, pending
    ).unwrap();

    /// Circuit breaker state
    static ref CIRCUIT_BREAKER_STATE: GaugeVec = register_gauge_vec!(
        "only1mcp_circuit_breaker_state",
        "Circuit breaker state (0=closed, 1=open, 2=half-open)",
        &["server_id"]
    ).unwrap();

    /// Rate limit exceeded count
    static ref RATE_LIMIT_EXCEEDED: CounterVec = register_counter_vec!(
        "only1mcp_rate_limit_exceeded_total",
        "Number of requests that exceeded rate limit",
        &["client_id"]
    ).unwrap();
}
```

**Then update the existing functions to use these:**
```rust
// In record_request()
MCP_REQUESTS_TOTAL
    .with_label_values(&[method, status])
    .inc();

MCP_REQUEST_DURATION_SECONDS
    .with_label_values(&[method])
    .observe(duration);

// etc.
```

**Validation:**
```bash
cargo check --lib
# Should compile without "cannot find value" errors
```

---

### 3. Implement Configuration Loading ⏱️ 3-4 hours
**Why:** Phase 1 requirement, server can't start without config

**What to do:**
```bash
# Edit src/config/mod.rs
```

**Key functions to implement:**
```rust
use figment::{Figment, providers::{Format, Yaml, Toml, Env}};
use serde::{Deserialize, Serialize};
use std::path::Path;

impl Config {
    /// Load from specific file
    pub fn from_file(path: &Path) -> Result<Self> {
        let config: Config = Figment::new()
            .merge(Yaml::file(path))
            .merge(Env::prefixed("ONLY1MCP_"))
            .extract()?;

        config.validate()?;
        Ok(config)
    }

    /// Discover and load config from standard locations
    pub fn discover_and_load() -> Result<Self> {
        // Try in order:
        // 1. ./only1mcp.yaml
        // 2. ./only1mcp.toml
        // 3. ~/.only1mcp/config.yaml
        // 4. /etc/only1mcp/config.yaml

        let paths = vec![
            "./only1mcp.yaml",
            "./only1mcp.toml",
            "~/.only1mcp/config.yaml",
            "/etc/only1mcp/config.yaml",
        ];

        for path_str in paths {
            let path = Path::new(path_str);
            if path.exists() {
                return Self::from_file(path);
            }
        }

        Err(Error::Config("No configuration file found".to_string()))
    }

    /// Validate configuration
    pub fn validate_file(path: &Path) -> Result<()> {
        let config = Self::from_file(path)?;
        config.validate()?;
        Ok(())
    }

    fn validate(&self) -> Result<()> {
        // Validate servers exist
        if self.servers.is_empty() {
            return Err(Error::Config("No servers configured".to_string()));
        }

        // Validate ports
        if self.server.port == 0 {
            return Err(Error::Config("Invalid port: 0".to_string()));
        }

        // More validation...
        Ok(())
    }
}
```

**Define the Config struct:**
```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub server: ServerConfig,
    pub servers: Vec<BackendServer>,
    #[serde(default)]
    pub routing: RoutingConfig,
    #[serde(default)]
    pub cache: CacheConfig,
    #[serde(default)]
    pub auth: Option<AuthConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    8080
}

// ... more config structures
```

**Validation:**
```bash
cargo test config::tests
only1mcp validate config/templates/solo.yaml
```

---

## 🟡 HIGH PRIORITY - Do These Next (8-12 hours)

### 4. Implement Request Handlers ⏱️ 6-8 hours
**Why:** Core functionality, server can't process requests without these

**Files to edit:**
- src/proxy/handler.rs (main handler logic)
- src/proxy/registry.rs (server management)
- src/proxy/router.rs (request routing integration)

**handler.rs - Key function:**
```rust
pub async fn handle_jsonrpc_request(
    State(state): State<AppState>,
    Json(request): Json<McpRequest>,
) -> Result<Json<McpResponse>, (StatusCode, String)> {
    // 1. Parse JSON-RPC request
    let method = &request.method;
    let params = request.params.clone();

    // 2. Select backend server via load balancer
    let server_id = state.registry
        .read()
        .await
        .select_server_for_method(method)
        .map_err(|e| (StatusCode::SERVICE_UNAVAILABLE, e.to_string()))?;

    // 3. Get transport for selected server
    let transport = state.registry
        .read()
        .await
        .get_transport(&server_id)
        .ok_or((StatusCode::NOT_FOUND, "Server not found".to_string()))?;

    // 4. Forward request to backend
    let response = transport
        .send_request(request)
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?;

    // 5. Record metrics
    state.metrics.record_request(method, duration);

    // 6. Return response
    Ok(Json(response))
}
```

**registry.rs - Key struct:**
```rust
pub struct ServerRegistry {
    servers: HashMap<ServerId, ServerInfo>,
    load_balancer: Arc<LoadBalancer>,
}

pub struct ServerInfo {
    id: ServerId,
    name: String,
    transport: Arc<dyn Transport>,
    status: ServerStatus,
}

impl ServerRegistry {
    pub async fn from_config(config: &Config) -> Result<Self> {
        let mut servers = HashMap::new();

        for server_config in &config.servers {
            let transport = match &server_config.transport {
                TransportType::Stdio => {
                    Arc::new(StdioTransport::new(...)?)
                }
                TransportType::Http => {
                    Arc::new(HttpTransport::new(...)?)
                }
                // ... other transports
            };

            servers.insert(
                server_config.id.clone(),
                ServerInfo { ... }
            );
        }

        Ok(Self {
            servers,
            load_balancer: Arc::new(LoadBalancer::new(config.routing.clone())),
        })
    }

    pub fn select_server_for_method(&self, method: &str) -> Result<ServerId> {
        let eligible = self.servers.keys().cloned().collect();
        self.load_balancer.select_server(method, &eligible, None).await
    }
}
```

**Validation:**
```bash
cargo test proxy::tests
```

---

### 5. Implement Active Health Checking ⏱️ 3-4 hours
**Why:** Reliability feature, automatic failover needs health state

**File to edit:**
- src/health/checker.rs

**Key implementation:**
```rust
pub struct HealthChecker {
    checks: Arc<DashMap<ServerId, HealthCheckConfig>>,
    results: Arc<DashMap<ServerId, HealthStatus>>,
}

impl HealthChecker {
    pub fn new() -> Self {
        Self {
            checks: Arc::new(DashMap::new()),
            results: Arc::new(DashMap::new()),
        }
    }

    /// Start background health checking
    pub fn start_checking(&self, registry: Arc<RwLock<ServerRegistry>>) {
        let checks = self.checks.clone();
        let results = self.results.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(10));

            loop {
                interval.tick().await;

                // Check each server
                for entry in checks.iter() {
                    let server_id = entry.key();
                    let config = entry.value();

                    // Perform health check
                    let healthy = Self::check_server(server_id, config).await;

                    // Update results
                    results.insert(
                        server_id.clone(),
                        HealthStatus { healthy, last_check: Utc::now() }
                    );

                    // Update load balancer
                    registry.read().await
                        .update_server_health(server_id, healthy);
                }
            }
        });
    }

    async fn check_server(id: &ServerId, config: &HealthCheckConfig) -> bool {
        // Send health check request
        // Return true if successful, false otherwise
        todo!()
    }
}
```

**Validation:**
```bash
cargo test health::tests::test_health_checker
```

---

## 🟢 MEDIUM PRIORITY - Do After Above (4-6 hours)

### 6. Write Integration Tests ⏱️ 4-6 hours
**Why:** Validate end-to-end functionality

**Create file:**
```bash
# tests/proxy_integration.rs
```

**Test scenarios:**
```rust
#[tokio::test]
async fn test_basic_request_routing() {
    // 1. Start mock MCP server
    // 2. Start proxy with config pointing to mock
    // 3. Send request to proxy
    // 4. Verify request reaches mock server
    // 5. Verify response returns to client
}

#[tokio::test]
async fn test_failover() {
    // 1. Start 2 mock servers
    // 2. Kill one server
    // 3. Verify requests route to healthy server
    // 4. Restart killed server
    // 5. Verify both servers receive traffic
}

#[tokio::test]
async fn test_load_balancing() {
    // 1. Start 3 mock servers
    // 2. Send 100 requests
    // 3. Verify requests distributed evenly
}

#[tokio::test]
async fn test_config_hot_reload() {
    // 1. Start proxy with initial config
    // 2. Modify config file (add server)
    // 3. Verify new server is available
    // 4. Send request to new server
}
```

**Validation:**
```bash
cargo test --test proxy_integration
```

---

### 7. Documentation Polish ⏱️ 1-2 hours
**Why:** Keep docs in sync with implementation

**Files to update:**

1. **ARCHITECTURE.md**
   - Add "Implementation Status" section
   - Update RBAC permissions list
   - Change "ProxyError" → "Error"

2. **PROJECT_SUMMARY.md**
   - Change "Achieved" → "Target" for performance metrics
   - Update phase completion percentages
   - Add note about current implementation status

3. **API_REFERENCE.md**
   - Add error code constants table from error.rs
   - Mark stub endpoints as "In Development"

**Quick search/replace:**
```bash
# Change all ProxyError references
grep -r "ProxyError" docs/ | wc -l
# Then update manually or with sed
```

---

## 📅 Timeline & Milestones

### This Week (Week 3)
**Goal:** Complete critical foundations

- [ ] Day 1: Extract types (2h) + Add metrics (1h)
- [ ] Day 2: Config loading (4h)
- [ ] Day 3: Start handlers (4h)
- [ ] Day 4: Continue handlers (4h)
- [ ] Day 5: Buffer / testing

**Week End Target:** Can load config and route basic requests

### Next Week (Week 4)
**Goal:** Complete handlers and testing

- [ ] Day 1-2: Finish handlers (6h)
- [ ] Day 3: Health checking (4h)
- [ ] Day 4-5: Integration tests (6h)

**Week End Target:** Phase 1 MVP functional

### Following Week (Week 5)
**Goal:** Polish and release

- [ ] Day 1: Documentation updates (2h)
- [ ] Day 2-3: End-to-end testing (4h)
- [ ] Day 4: Bug fixes (4h)
- [ ] Day 5: Phase 1 release prep (2h)

**Week End Target:** Phase 1 v0.1.0 Release

---

## ✅ Definition of Done

### For Each Task
- [ ] Code compiles without warnings
- [ ] Unit tests pass
- [ ] Integration tests pass (if applicable)
- [ ] Documentation updated (if needed)
- [ ] PR reviewed (if team workflow)

### For Phase 1 MVP
- [ ] All critical tasks (1-3) complete
- [ ] All high priority tasks (4-5) complete
- [ ] Basic integration tests passing
- [ ] Configuration loading works
- [ ] Can route requests to backends
- [ ] Health checking functional
- [ ] Documentation up to date
- [ ] CLI commands work
- [ ] Performance targets met (initial)

---

## 🚀 Quick Start Commands

**After completing critical tasks:**

```bash
# Build project
cargo build

# Run tests
cargo test

# Validate a config file
cargo run -- validate config/templates/solo.yaml

# Start server
cargo run -- start --config config/templates/solo.yaml

# Check health
curl http://localhost:8080/health

# Get metrics
curl http://localhost:8080/api/v1/admin/metrics

# Send test request
curl -X POST http://localhost:8080/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/list",
    "id": 1
  }'
```

---

## 📊 Progress Tracking

### Use this checklist to track progress:

**Critical (Do First):**
- [ ] Extract MCP types to types/mod.rs
- [ ] Add lazy_static! metrics declarations
- [ ] Implement config loading (YAML/TOML)

**High Priority (Do Next):**
- [ ] Implement handle_jsonrpc_request
- [ ] Implement ServerRegistry methods
- [ ] Implement request router integration
- [ ] Implement active health checking
- [ ] Write basic integration tests

**Medium Priority (After Above):**
- [ ] Complete all integration test scenarios
- [ ] Update documentation (ARCHITECTURE.md)
- [ ] Update documentation (PROJECT_SUMMARY.md)
- [ ] Update documentation (API_REFERENCE.md)
- [ ] Create TROUBLESHOOTING.md

**Phase 1 Complete When:**
- [ ] All critical + high priority items done
- [ ] Integration tests passing
- [ ] Can demo end-to-end request flow
- [ ] Documentation accurate and complete

---

## 🆘 Need Help?

**Stuck on a task?**
1. Check CLAUDE.local.md for context
2. Review ref_docs/14 for implementation patterns
3. Look at existing similar code (e.g., circuit_breaker.rs for patterns)

**Found a blocker?**
1. Document in CLAUDE.local.md under "Known Issues"
2. Assess if it's architectural or tactical
3. Consider workarounds or scope adjustments

**Questions about architecture?**
1. Check ARCHITECTURE_ALIGNMENT_AUDIT.md Section 3-6
2. Review architecture diagrams in ref_docs/21
3. Verify against PHASE_1_PLAN.md requirements

---

**Last Updated:** October 14, 2025
**Next Review:** After completing critical tasks (1-3)

*Keep this file updated as you complete tasks!*
