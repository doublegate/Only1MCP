# Only1MCP Implementation Roadmap & Sprint Planning
## Detailed Development Strategy from Concept to Production

**Document Version:** 1.0  
**Planning Horizon:** 12-16 weeks (MVP to V1.0)  
**Methodology:** Agile with 2-week sprints  
**Date:** October 14, 2025  
**Status:** Strategic Implementation Plan

---

## TABLE OF CONTENTS

1. [Executive Summary](#executive-summary)
2. [Development Philosophy](#development-philosophy)
3. [Phase 1: MVP - CLI Core (Weeks 1-4)](#phase-1-mvp---cli-core-weeks-1-4)
4. [Phase 2: Advanced Features (Weeks 5-8)](#phase-2-advanced-features-weeks-5-8)
5. [Phase 3: Enterprise Features (Weeks 9-12)](#phase-3-enterprise-features-weeks-9-12)
6. [Phase 4: Polish & Extensions (Weeks 13+)](#phase-4-polish--extensions-weeks-13)
7. [Resource Allocation](#resource-allocation)
8. [Risk Management](#risk-management)
9. [Success Metrics & KPIs](#success-metrics--kpis)
10. [Release Strategy](#release-strategy)

---

## EXECUTIVE SUMMARY

### Project Timeline Overview

```
Week 1-4:  MVP (CLI Core) ━━━━━━━━━━━━━━━━━━━━━━━━━━━━●
Week 5-8:  Advanced Features ━━━━━━━━━━━━━━━━━━━━━━━━●
Week 9-12: Enterprise Features ━━━━━━━━━━━━━━━━━━━━━●
Week 13+:  Polish & Extensions ━━━━━━━━━━━━━━━━━━━━━→
```

### Critical Path Items (Blockers)
1. **Week 1-2**: Core proxy routing & server registry (foundation for everything)
2. **Week 3**: STDIO transport implementation (needed for 80% of MCP servers)
3. **Week 5**: Consistent hashing (enables load balancing & failover)
4. **Week 9**: OAuth2/RBAC (enterprise blocker)

### Team Composition (Recommended)
- **Phase 1 (MVP)**: 2 senior Rust engineers + 1 testing engineer
- **Phase 2-3**: Add 1 frontend engineer (for UI), 1 DevOps engineer (for CI/CD)
- **Phase 4**: Add 1 technical writer (documentation), 1 community manager

### Key Deliverables by Phase
| Phase | Week | Deliverable | User Impact |
|-------|------|-------------|-------------|
| 1 | 4 | Working CLI proxy with hot-reload | Replace manual multi-server config |
| 2 | 8 | Performance dashboard + CLI tools | Real-time monitoring & debugging |
| 3 | 12 | Enterprise-ready (RBAC, audit logs) | Production deployment for teams |
| 4 | 16+ | Plugin system + advanced optimization | Community extensions & ML routing |

---

## DEVELOPMENT PHILOSOPHY

### Principles

**1. Ship Early, Iterate Fast**
- MVP in 4 weeks with core value proposition
- Weekly user feedback loops starting Week 5
- Feature flags for experimental functionality

**2. Test-Driven Development**
```rust
// Write tests FIRST, then implementation
#[tokio::test]
async fn test_hot_reload_preserves_connections() {
    let proxy = setup_test_proxy().await;
    let active_requests = simulate_load(&proxy, 100).await;
    
    // Trigger hot reload
    proxy.reload_config(new_config).await.unwrap();
    
    // Verify no dropped connections
    assert_eq!(active_requests.completed().await, 100);
}
```

**3. Performance as Feature**
- Benchmark every PR that touches hot paths
- <5ms latency overhead target enforced via CI
- Weekly performance regression testing

**4. Documentation is Code**
- Every public API requires doc comment with example
- Architecture Decision Records (ADRs) for major choices
- Auto-generated docs via mdBook deployed on every commit

### Technology Decisions (Locked for MVP)

**Approved Stack:**
- **HTTP Server**: Axum 0.7+ (benchmarks show 30-60% faster than alternatives)
- **Async Runtime**: Tokio 1.x (multi-threaded, industry standard)
- **Caching**: DashMap (lock-free, proven in rust-rpxy)
- **CLI**: Clap 4.x (derive API for maintainability)
- **Serialization**: Serde + serde_json
- **TLS**: Rustls (memory-safe, no OpenSSL dependency)
- **Metrics**: Prometheus + tracing crates

**Alternatives Considered & Rejected:**
- ❌ Actix-web: Higher performance but more complex actor model
- ❌ Redis for caching: External dependency, unnecessary for MVP
- ❌ Python for scripting: Adds runtime dependency, violates standalone goal

---

## PHASE 1: MVP - CLI CORE (WEEKS 1-4)

### Goals
✅ Prove technical feasibility  
✅ Deliver immediate value to early adopters  
✅ Validate architecture decisions  
✅ Establish CI/CD pipeline  

### Sprint 1 (Weeks 1-2): Foundation

**Epic 1.1: Project Scaffolding (Week 1, Days 1-2)**
```bash
# Initial setup
cargo new only1mcp --bin
cd only1mcp

# Add core dependencies
cargo add tokio@1 --features full
cargo add axum@0.7
cargo add serde@1 --features derive
cargo add serde_json@1
cargo add clap@4 --features derive
cargo add tracing@0.1
cargo add tracing-subscriber@0.3

# Dev dependencies
cargo add --dev criterion@0.5 --features html_reports
cargo add --dev tokio-test@0.4
```

**Tasks:**
- [ ] Initialize Git repo with .gitignore, LICENSE (Apache 2.0 + MIT dual)
- [ ] Set up GitHub Actions CI (lint, test, build) - see 06-Testing_Strategy.md
- [ ] Create CONTRIBUTING.md with PR template
- [ ] Configure rustfmt.toml and clippy.toml for team standards
- [ ] Set up directory structure:
```
src/
├── main.rs              # CLI entry point
├── lib.rs               # Library API
├── config/              # Configuration parsing
│   ├── mod.rs
│   └── schema.rs        # YAML/TOML schemas
├── proxy/               # Core proxy logic
│   ├── mod.rs
│   ├── router.rs        # Request routing
│   └── registry.rs      # Server registry
├── transport/           # MCP transports
│   ├── mod.rs
│   ├── stdio.rs
│   └── http.rs
└── error.rs             # Error types
```

**Epic 1.2: Basic Proxy Routing (Week 1, Days 3-5)**

**Feature:** Accept HTTP requests and forward to single backend server.

```rust
// src/proxy/router.rs
use axum::{
    extract::{State, Json},
    http::StatusCode,
    response::IntoResponse,
};
use serde_json::Value;

#[derive(Clone)]
pub struct AppState {
    backend_url: String,
    client: reqwest::Client,
}

/// Proxy handler - forwards JSON-RPC requests to backend
pub async fn proxy_handler(
    State(state): State<AppState>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    // Forward request to backend
    let response = state.client
        .post(&state.backend_url)
        .json(&payload)
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;
    
    // Parse and return response
    let result: Value = response.json()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;
    
    Ok(Json(result))
}
```

**Tests:**
```rust
#[tokio::test]
async fn test_basic_proxy() {
    // Start mock MCP server
    let mock_server = start_mock_mcp_server().await;
    
    // Start proxy pointing to mock
    let proxy = start_proxy(mock_server.url()).await;
    
    // Send request through proxy
    let response: Value = send_json_rpc(&proxy, "tools/list", json!({}))
        .await
        .unwrap();
    
    // Verify response
    assert!(response["result"]["tools"].is_array());
}
```

**Epic 1.3: Server Registry (Week 2, Days 1-3)**

**Feature:** Manage multiple backend servers with thread-safe access.

```rust
// src/proxy/registry.rs
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ServerInfo {
    pub id: String,
    pub name: String,
    pub url: String,
    pub transport: Transport,
    pub status: ServerStatus,
}

#[derive(Debug, Clone, Copy)]
pub enum ServerStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

pub type ServerRegistry = Arc<RwLock<HashMap<String, ServerInfo>>>;

pub async fn add_server(
    registry: &ServerRegistry,
    server: ServerInfo,
) -> Result<(), RegistryError> {
    let mut reg = registry.write().await;
    
    // Check for duplicate IDs
    if reg.contains_key(&server.id) {
        return Err(RegistryError::DuplicateId(server.id));
    }
    
    reg.insert(server.id.clone(), server);
    Ok(())
}

pub async fn list_servers(
    registry: &ServerRegistry,
) -> Vec<ServerInfo> {
    let reg = registry.read().await;
    reg.values().cloned().collect()
}
```

**Epic 1.4: Configuration System (Week 2, Days 4-5)**

**Feature:** Load/save configuration from YAML files.

```yaml
# Example config.yaml
version: "1.0"

servers:
  - id: "filesystem-mcp"
    name: "Filesystem Server"
    transport: "stdio"
    command: "npx"
    args: ["@modelcontextprotocol/server-filesystem", "/path/to/files"]
    
  - id: "web-search-mcp"
    name: "Web Search"
    transport: "http"
    url: "https://search.example.com/mcp"
    auth:
      type: "bearer"
      token_env: "SEARCH_API_KEY"

proxy:
  host: "0.0.0.0"
  port: 8080
  tls:
    enabled: false
```

```rust
// src/config/schema.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub version: String,
    pub servers: Vec<ServerConfig>,
    pub proxy: ProxyConfig,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ServerConfig {
    pub id: String,
    pub name: String,
    pub transport: String,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub auth: Option<AuthConfig>,
}

impl Config {
    pub fn from_file(path: &str) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = serde_yaml::from_str(&content)?;
        config.validate()?;
        Ok(config)
    }
    
    fn validate(&self) -> Result<(), ConfigError> {
        // Ensure all server IDs are unique
        let mut ids = HashSet::new();
        for server in &self.servers {
            if !ids.insert(&server.id) {
                return Err(ConfigError::DuplicateServerId(server.id.clone()));
            }
        }
        Ok(())
    }
}
```

**Sprint 1 Deliverables:**
- [x] Basic HTTP proxy forwarding requests to single backend
- [x] Server registry with add/remove/list operations
- [x] YAML configuration loading with validation
- [x] CLI skeleton with `--config` flag
- [x] Unit tests for registry operations
- [x] Integration test with mock MCP server
- [x] CI pipeline (GitHub Actions) running tests on every commit

### Sprint 2 (Weeks 3-4): STDIO Transport & Hot Reload

**Epic 2.1: STDIO Transport (Week 3)**

**Feature:** Launch and communicate with STDIO-based MCP servers.

```rust
// src/transport/stdio.rs
use tokio::process::{Command, Child, ChildStdin, ChildStdout};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

pub struct StdioTransport {
    process: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl StdioTransport {
    pub async fn spawn(config: &ServerConfig) -> Result<Self, TransportError> {
        // Validate command is in allowlist
        let allowed_cmds = ["node", "npx", "python", "python3", "uvx"];
        let cmd = config.command.as_deref()
            .ok_or(TransportError::MissingCommand)?;
        
        if !allowed_cmds.contains(&cmd) {
            return Err(TransportError::CommandNotAllowed(cmd.to_string()));
        }
        
        // Spawn process
        let mut process = Command::new(cmd)
            .args(&config.args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?;
        
        let stdin = process.stdin.take()
            .ok_or(TransportError::StdinUnavailable)?;
        let stdout = process.stdout.take()
            .ok_or(TransportError::StdoutUnavailable)?;
        let stdout = BufReader::new(stdout);
        
        Ok(Self { process, stdin, stdout })
    }
    
    pub async fn send_request(
        &mut self,
        request: &JsonRpcRequest,
    ) -> Result<JsonRpcResponse, TransportError> {
        // Serialize request
        let json = serde_json::to_string(request)?;
        
        // Write to stdin with newline
        self.stdin.write_all(json.as_bytes()).await?;
        self.stdin.write_all(b"\n").await?;
        self.stdin.flush().await?;
        
        // Read response line from stdout
        let mut line = String::new();
        self.stdout.read_line(&mut line).await?;
        
        // Parse response
        let response: JsonRpcResponse = serde_json::from_str(&line)?;
        Ok(response)
    }
}
```

**Tests:**
- [ ] Spawn real STDIO server (e.g., @modelcontextprotocol/server-filesystem)
- [ ] Send `initialize` request and verify response
- [ ] Send `tools/list` and verify tool schemas
- [ ] Send `tools/call` and verify execution
- [ ] Test error handling for crashed processes
- [ ] Test timeout handling (5 second default)

**Epic 2.2: Hot Configuration Reload (Week 4)**

**Feature:** Update server list without restarting proxy or dropping connections.

```rust
// src/proxy/hot_reload.rs
use tokio::sync::watch;
use notify::{Watcher, RecommendedWatcher, RecursiveMode};

pub struct HotReloadManager {
    config_path: PathBuf,
    tx: watch::Sender<Config>,
    _watcher: RecommendedWatcher,
}

impl HotReloadManager {
    pub fn new(config_path: PathBuf) -> Result<(Self, watch::Receiver<Config>), HotReloadError> {
        // Load initial config
        let config = Config::from_file(&config_path)?;
        let (tx, rx) = watch::channel(config);
        
        // Set up file watcher
        let tx_clone = tx.clone();
        let path_clone = config_path.clone();
        
        let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
            match res {
                Ok(event) if event.kind.is_modify() => {
                    // Config file modified, reload
                    match Config::from_file(&path_clone) {
                        Ok(new_config) => {
                            let _ = tx_clone.send(new_config);
                            tracing::info!("Configuration reloaded successfully");
                        }
                        Err(e) => {
                            tracing::error!("Failed to reload config: {}", e);
                        }
                    }
                }
                _ => {}
            }
        })?;
        
        watcher.watch(&config_path, RecursiveMode::NonRecursive)?;
        
        Ok((Self { config_path, tx, _watcher: watcher }, rx))
    }
}

// In main.rs: wire up hot reload to registry
pub async fn apply_config_updates(
    mut rx: watch::Receiver<Config>,
    registry: ServerRegistry,
) {
    while rx.changed().await.is_ok() {
        let new_config = rx.borrow().clone();
        
        // Atomic swap of server list
        let mut reg = registry.write().await;
        *reg = build_registry_from_config(&new_config);
        
        tracing::info!("Server registry updated: {} servers", reg.len());
    }
}
```

**Tests:**
- [ ] Modify config file and verify registry updates without restart
- [ ] Ensure in-flight requests complete during reload
- [ ] Test error handling for invalid config (should not crash)
- [ ] Benchmark reload time (<100ms target)

**Sprint 2 Deliverables:**
- [x] STDIO transport fully functional
- [x] File-based hot reload working
- [x] CLI commands: `only1mcp start --config config.yaml`
- [x] Integration tests with real MCP servers
- [x] Basic logging with `tracing` crate
- [x] Performance baseline: <5ms latency overhead for proxy forwarding

---

## PHASE 2: ADVANCED FEATURES (WEEKS 5-8)

### Sprint 3 (Weeks 5-6): Load Balancing & Health Checks

**Epic 3.1: Consistent Hashing (Week 5)**

**Feature:** Distribute requests across multiple instances of same tool server.

```rust
// src/proxy/load_balancer.rs
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};
use twox_hash::XxHash64;

pub struct ConsistentHash {
    ring: BTreeMap<u64, String>, // hash -> server_id
    virtual_nodes: usize,
}

impl ConsistentHash {
    pub fn new(virtual_nodes: usize) -> Self {
        Self {
            ring: BTreeMap::new(),
            virtual_nodes,
        }
    }
    
    pub fn add_server(&mut self, server_id: &str) {
        // Add virtual nodes for this server
        for i in 0..self.virtual_nodes {
            let key = format!("{}-vnode-{}", server_id, i);
            let hash = self.hash_key(&key);
            self.ring.insert(hash, server_id.to_string());
        }
    }
    
    pub fn get_server(&self, request_key: &str) -> Option<&String> {
        if self.ring.is_empty() {
            return None;
        }
        
        let hash = self.hash_key(request_key);
        
        // Find first server >= hash (clockwise on ring)
        self.ring.range(hash..)
            .next()
            .or_else(|| self.ring.iter().next())
            .map(|(_, server_id)| server_id)
    }
    
    fn hash_key(&self, key: &str) -> u64 {
        let mut hasher = XxHash64::default();
        key.hash(&mut hasher);
        hasher.finish()
    }
}
```

**Tests:**
- [ ] Verify uniform distribution across servers
- [ ] Test minimal remapping when adding/removing server
- [ ] Benchmark: <1µs per lookup
- [ ] Property test: same key always maps to same server

**Epic 3.2: Active Health Checks (Week 6)**

**Feature:** Periodically ping servers and mark unhealthy ones.

```rust
// src/proxy/health.rs
use std::time::Duration;
use tokio::time::interval;

pub struct HealthChecker {
    registry: ServerRegistry,
    check_interval: Duration,
    timeout: Duration,
}

impl HealthChecker {
    pub async fn start(self) {
        let mut ticker = interval(self.check_interval);
        
        loop {
            ticker.tick().await;
            self.check_all_servers().await;
        }
    }
    
    async fn check_all_servers(&self) {
        let servers: Vec<_> = {
            let reg = self.registry.read().await;
            reg.values().cloned().collect()
        };
        
        // Check all servers in parallel
        let checks: Vec<_> = servers.iter()
            .map(|server| self.check_server(server))
            .collect();
        
        let results = futures::future::join_all(checks).await;
        
        // Update statuses
        let mut reg = self.registry.write().await;
        for (server, is_healthy) in servers.iter().zip(results) {
            if let Some(info) = reg.get_mut(&server.id) {
                info.status = if is_healthy {
                    ServerStatus::Healthy
                } else {
                    ServerStatus::Unhealthy
                };
            }
        }
    }
    
    async fn check_server(&self, server: &ServerInfo) -> bool {
        // Send minimal JSON-RPC request (e.g., ping or capabilities)
        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "ping",
        });
        
        match tokio::time::timeout(
            self.timeout,
            send_request_to_server(server, &request)
        ).await {
            Ok(Ok(_)) => true,
            _ => false,
        }
    }
}
```

**Sprint 3 Deliverables:**
- [x] Consistent hashing with configurable virtual nodes (default 200)
- [x] Active health checks (10s interval, 5s timeout)
- [x] Automatic failover to healthy servers
- [x] Metrics: requests per server, error rates, response times

### Sprint 4 (Weeks 7-8): Context Optimization & CLI Tools

**Epic 4.1: Request Batching (Week 7)**

**Feature:** Combine multiple tool calls into single MCP request when possible.

```rust
// Pseudocode - actual implementation depends on MCP batch support
pub struct BatchProcessor {
    pending: Vec<(JsonRpcRequest, oneshot::Sender<JsonRpcResponse>)>,
    max_batch_size: usize,
    max_wait_ms: u64,
}

impl BatchProcessor {
    pub async fn submit_request(
        &mut self,
        request: JsonRpcRequest,
    ) -> JsonRpcResponse {
        let (tx, rx) = oneshot::channel();
        self.pending.push((request, tx));
        
        // If batch full or timeout, flush
        if self.pending.len() >= self.max_batch_size {
            self.flush_batch().await;
        }
        
        rx.await.unwrap()
    }
    
    async fn flush_batch(&mut self) {
        let batch: Vec<_> = self.pending.drain(..).collect();
        
        // Send as JSON-RPC batch request
        let batch_request: Vec<_> = batch.iter()
            .map(|(req, _)| req.clone())
            .collect();
        
        let responses = send_batch_to_backend(&batch_request).await;
        
        // Distribute responses back to callers
        for ((_, tx), response) in batch.into_iter().zip(responses) {
            let _ = tx.send(response);
        }
    }
}
```

**Epic 4.2: Response Caching (Week 7)**

**Feature:** Cache idempotent tool responses to reduce backend load.

```rust
// src/proxy/cache.rs
use dashmap::DashMap;
use std::time::{Duration, Instant};

#[derive(Clone)]
pub struct CacheEntry {
    pub response: JsonRpcResponse,
    pub cached_at: Instant,
    pub ttl: Duration,
}

pub type ResponseCache = Arc<DashMap<String, CacheEntry>>;

pub fn cache_key(request: &JsonRpcRequest) -> String {
    // Hash based on method + params
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(request.method.as_bytes());
    hasher.update(serde_json::to_string(&request.params).unwrap().as_bytes());
    format!("{:x}", hasher.finalize())
}

pub fn get_cached_or_fetch(
    cache: &ResponseCache,
    request: &JsonRpcRequest,
    ttl: Duration,
    fetch_fn: impl std::future::Future<Output = JsonRpcResponse>,
) -> impl std::future::Future<Output = JsonRpcResponse> {
    let key = cache_key(request);
    
    async move {
        // Check cache
        if let Some(entry) = cache.get(&key) {
            if entry.cached_at.elapsed() < entry.ttl {
                return entry.response.clone();
            } else {
                // Expired, remove
                cache.remove(&key);
            }
        }
        
        // Fetch from backend
        let response = fetch_fn.await;
        
        // Cache result
        cache.insert(key, CacheEntry {
            response: response.clone(),
            cached_at: Instant::now(),
            ttl,
        });
        
        response
    }
}
```

**Epic 4.3: CLI Management Tools (Week 8)**

```bash
# New CLI commands
only1mcp list                    # List all servers
only1mcp add <name> <url>        # Add server
only1mcp remove <name>           # Remove server
only1mcp status                  # Show health status
only1mcp logs [--server <name>]  # View logs
only1mcp test <name>             # Test server connection
only1mcp benchmark               # Run performance benchmarks
```

**Sprint 4 Deliverables:**
- [x] Request batching (opt-in via config)
- [x] Response caching with configurable TTL
- [x] CLI tools for runtime management
- [x] Basic performance dashboard (terminal output)
- [x] Prometheus metrics export on `/metrics`

---

## PHASE 3: ENTERPRISE FEATURES (WEEKS 9-12)

### Sprint 5 (Weeks 9-10): Security & RBAC

**Epic 5.1: OAuth2/JWT Authentication (Week 9)**

**Feature:** Secure proxy with token-based authentication.

```rust
// src/auth/jwt.rs
use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};

#[derive(Debug, Deserialize)]
pub struct Claims {
    pub sub: String,     // User ID
    pub roles: Vec<String>,
    pub exp: usize,
}

pub async fn verify_jwt(token: &str, secret: &[u8]) -> Result<Claims, AuthError> {
    let validation = Validation::new(Algorithm::HS256);
    
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret),
        &validation,
    )?;
    
    Ok(token_data.claims)
}

// Axum middleware
pub async fn auth_middleware(
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Extract bearer token
    let token = headers.get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;
    
    // Verify token
    let claims = verify_jwt(token, JWT_SECRET.as_bytes())
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;
    
    // Add claims to request extensions
    request.extensions_mut().insert(claims);
    
    Ok(next.run(request).await)
}
```

**Epic 5.2: Role-Based Access Control (Week 10)**

**Feature:** Restrict tool access by user role.

```yaml
# RBAC config
roles:
  - name: "admin"
    permissions:
      - "*"  # All tools
      
  - name: "developer"
    permissions:
      - "filesystem:*"
      - "github:*"
      - "web_search"
      
  - name: "readonly"
    permissions:
      - "filesystem:read"
      - "web_search"
```

```rust
pub fn check_permission(
    claims: &Claims,
    tool_name: &str,
) -> bool {
    for role in &claims.roles {
        if let Some(permissions) = get_role_permissions(role) {
            if permissions.allows(tool_name) {
                return true;
            }
        }
    }
    false
}
```

**Epic 5.3: Audit Logging (Week 10)**

**Feature:** Log all tool invocations for compliance.

```rust
// src/audit/logger.rs
use serde::Serialize;

#[derive(Serialize)]
pub struct AuditLog {
    timestamp: DateTime<Utc>,
    user_id: String,
    tool_name: String,
    arguments: Value,
    result: String,  // "success" or "error"
    duration_ms: u64,
}

pub async fn log_tool_call(
    user_id: &str,
    tool_name: &str,
    arguments: &Value,
    result: &Result<JsonRpcResponse, Error>,
    duration: Duration,
) {
    let log = AuditLog {
        timestamp: Utc::now(),
        user_id: user_id.to_string(),
        tool_name: tool_name.to_string(),
        arguments: arguments.clone(),
        result: if result.is_ok() { "success" } else { "error" }.to_string(),
        duration_ms: duration.as_millis() as u64,
    };
    
    // Write to structured log (JSON)
    tracing::info!(target: "audit", "{}", serde_json::to_string(&log).unwrap());
    
    // Optionally: persist to database or send to SIEM
}
```

**Sprint 5 Deliverables:**
- [x] JWT authentication on all endpoints
- [x] RBAC with configurable role permissions
- [x] Comprehensive audit logging
- [x] TLS 1.3 support (Rustls)
- [x] Rate limiting (60 req/min per user by default)

### Sprint 6 (Weeks 11-12): Observability & TUI

**Epic 6.1: OpenTelemetry Tracing (Week 11)**

```rust
use tracing_opentelemetry::OpenTelemetryLayer;
use opentelemetry::sdk::trace::TracerProvider;

// In main.rs
let tracer = opentelemetry_otlp::new_pipeline()
    .tracing()
    .with_exporter(opentelemetry_otlp::new_exporter().http())
    .install_batch(opentelemetry::runtime::Tokio)
    .unwrap();

tracing_subscriber::registry()
    .with(OpenTelemetryLayer::new(tracer))
    .with(tracing_subscriber::fmt::layer())
    .init();
```

**Epic 6.2: Interactive TUI (Week 12)**

**Feature:** Terminal dashboard with real-time monitoring.

```rust
// Using ratatui
use ratatui::{
    backend::CrosstermBackend,
    widgets::{Block, Borders, List, ListItem},
    Terminal,
};

pub struct TuiApp {
    registry: ServerRegistry,
    metrics: MetricsCollector,
}

impl TuiApp {
    pub async fn run(&mut self) -> Result<(), Box<dyn Error>> {
        let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
        
        loop {
            terminal.draw(|f| {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Percentage(30),
                        Constraint::Percentage(70),
                    ])
                    .split(f.size());
                
                // Server list
                let servers = self.render_server_list();
                f.render_widget(servers, chunks[0]);
                
                // Metrics chart
                let metrics = self.render_metrics();
                f.render_widget(metrics, chunks[1]);
            })?;
            
            // Handle keyboard input
            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Char('r') => self.reload_servers().await?,
                        _ => {}
                    }
                }
            }
        }
        
        Ok(())
    }
}
```

**Sprint 6 Deliverables:**
- [x] OpenTelemetry integration (Jaeger-compatible)
- [x] Structured JSON logging
- [x] Interactive TUI (`only1mcp tui`)
- [x] Grafana dashboard templates
- [x] Complete documentation set (user guide, API docs, examples)

---

## PHASE 4: POLISH & EXTENSIONS (WEEKS 13+)

### Sprint 7+: Advanced Features

**Epic 7.1: Plugin System (Week 13-14)**
- Dynamic library loading for custom routers
- WASM module support for sandboxed extensions
- Plugin API documentation

**Epic 7.2: AI-Driven Optimization (Week 15-16)**
- Lightweight ML model for request routing predictions
- Integration with `candle` or `burn` crates
- Historical data collection & training pipeline

**Epic 7.3: Container Orchestration (Week 17+)**
- Optional Bollard integration for Docker/Podman
- On-the-fly image building based on config
- Lifecycle management (start/stop/update containers)

---

## RESOURCE ALLOCATION

### Team Structure by Phase

```
Phase 1 (Weeks 1-4): Core Team
├── Senior Rust Engineer #1 (Lead) - 40h/week
├── Senior Rust Engineer #2 - 40h/week
└── QA Engineer - 20h/week

Phase 2 (Weeks 5-8): Expanded Team
├── Core Team (above)
├── Frontend Engineer - 30h/week (UI work)
└── DevOps Engineer - 20h/week (CI/CD, infrastructure)

Phase 3 (Weeks 9-12): Full Team
├── Core + Expanded (above)
├── Technical Writer - 30h/week
└── Security Auditor - 10h/week (consultant)

Phase 4 (Weeks 13+): Maintenance
├── Senior Rust Engineer #1 - 20h/week
├── Community Manager - 15h/week
└── On-call rotation
```

### Budget Estimates

**Personnel Costs (12 weeks):**
- Senior Rust Engineers (2 × $150/hr × 480h): $144,000
- QA Engineer ($100/hr × 240h): $24,000
- Frontend Engineer ($120/hr × 360h): $43,200
- DevOps Engineer ($130/hr × 240h): $31,200
- Technical Writer ($80/hr × 360h): $28,800
- Security Auditor ($200/hr × 120h): $24,000
**Total Personnel: $295,200**

**Infrastructure:**
- GitHub Actions CI/CD: $0 (free for open-source)
- Test environments (AWS): $500/month × 3 = $1,500
- Domain & hosting: $200
**Total Infrastructure: $1,700**

**Grand Total (MVP to V1.0): ~$297,000**

---

## RISK MANAGEMENT

### Critical Risks & Mitigation

**RISK 1: MCP Protocol Changes (HIGH)**
- **Impact**: Breaking changes to API spec
- **Probability**: MEDIUM (protocol is young, evolving)
- **Mitigation**:
  - Monitor official MCP repo closely
  - Attend community meetings
  - Design adapter layer to isolate protocol changes
  - Version our API separately from MCP spec
- **Contingency**: 2-week buffer in schedule for emergency refactoring

**RISK 2: Performance Targets Not Met (MEDIUM)**
- **Impact**: <5ms latency goal missed, reducing value prop
- **Probability**: LOW (Rust + Axum are proven fast)
- **Mitigation**:
  - Benchmark continuously from Week 1
  - Profile with flamegraph weekly
  - Keep architecture simple (avoid premature optimization)
- **Contingency**: Allocate Week 17 for optimization sprint if needed

**RISK 3: STDIO Transport Instability (MEDIUM)**
- **Impact**: Frequent crashes with community servers
- **Probability**: MEDIUM (many servers are alpha quality)
- **Mitigation**:
  - Robust error handling + process restart logic
  - Comprehensive logging for debugging
  - Work with server maintainers to fix issues
- **Contingency**: Fallback to HTTP-only for unreliable servers

**RISK 4: Key Developer Departure (LOW)**
- **Impact**: Project stalls
- **Probability**: LOW
- **Mitigation**:
  - Thorough documentation + ADRs
  - Pair programming on complex modules
  - Cross-train team members
- **Contingency**: Contract with Rust consulting firm for temporary backfill

### Weekly Risk Reviews

**Every Friday:**
- Review risk register
- Update probabilities based on new info
- Trigger contingencies if thresholds crossed
- Communicate status to stakeholders

---

## SUCCESS METRICS & KPIS

### Technical KPIs (Measured Weekly)

| Metric | Target | Measurement |
|--------|--------|-------------|
| Latency Overhead (p50) | <2ms | Criterion benchmarks |
| Latency Overhead (p99) | <5ms | Criterion benchmarks |
| Throughput | >10k req/s | Load testing (wrk) |
| Memory Usage (steady state) | <50MB | Valgrind massif |
| Test Coverage | >80% | cargo-tarpaulin |
| CI Build Time | <5 min | GitHub Actions metrics |
| Hot Reload Time | <100ms | Custom benchmark |
| Cache Hit Rate | >70% | Prometheus metrics |

### User KPIs (Measured After Launch)

| Metric | Target (3 months) | Measurement |
|--------|-------------------|-------------|
| GitHub Stars | 500+ | GitHub API |
| Downloads (crates.io) | 1,000+ | crates.io stats |
| Active Weekly Users | 100+ | Telemetry (opt-in) |
| Configuration Time | <5 min | User surveys |
| Reported Bugs (Critical) | <3/month | GitHub Issues |

### Business KPIs

| Metric | Target | Measurement |
|--------|--------|-------------|
| Enterprise Pilot Users | 3+ | Direct outreach |
| Community Contributions | 10+ PRs | GitHub Activity |
| Documentation Completeness | 100% | Manual audit |

---

## RELEASE STRATEGY

### Version Numbering (SemVer)

```
0.1.0 - Week 4:  MVP Release (CLI core)
0.2.0 - Week 8:  Advanced features
0.3.0 - Week 12: Enterprise features
1.0.0 - Week 16: Production-ready
```

### Pre-Release Testing

**Alpha (Weeks 2-4):**
- Internal testing only
- Focus on correctness

**Beta (Weeks 5-12):**
- Limited external users (invite-only)
- Stability + performance focus
- Breaking changes allowed

**Release Candidate (Week 13):**
- Public announcement
- No breaking changes
- 2-week soak period

**V1.0 GA (Week 16):**
- Production-ready guarantee
- LTS support commitment

### Release Checklist

**Before Each Release:**
- [ ] All tests passing on CI
- [ ] Benchmarks run and reviewed
- [ ] Security audit completed (automated + manual for major versions)
- [ ] CHANGELOG.md updated with user-facing changes
- [ ] Documentation updated
- [ ] Cross-platform builds tested (Linux, macOS, Windows)
- [ ] Migration guide written (if breaking changes)
- [ ] Release notes drafted
- [ ] Docker image built and pushed (optional)

**Post-Release:**
- [ ] Announce on GitHub, Reddit (r/rust, r/mcp), Discord
- [ ] Update website/docs with new version
- [ ] Monitor error tracking for 48 hours
- [ ] Respond to user feedback

---

## APPENDIX A: SPRINT CEREMONIES

### Daily Standup (15 min)
- **Format**: Async via Slack for remote team
- **Questions**:
  1. What did you complete yesterday?
  2. What will you work on today?
  3. Any blockers?

### Sprint Planning (2 hours, every 2 weeks)
- Review backlog
- Estimate stories (Fibonacci points)
- Commit to sprint goal
- Assign tasks

### Sprint Review (1 hour, every 2 weeks)
- Demo completed features
- Gather stakeholder feedback
- Update roadmap

### Sprint Retrospective (1 hour, every 2 weeks)
- What went well?
- What could improve?
- Action items for next sprint

---

## APPENDIX B: DEFINITION OF DONE

**A story is "done" when:**
- [ ] Code written and reviewed (PR approved)
- [ ] Unit tests written and passing
- [ ] Integration tests written (if applicable)
- [ ] Benchmarks run (if performance-critical)
- [ ] Documentation updated (code comments + user docs)
- [ ] CHANGELOG.md entry added
- [ ] Tested on all platforms (via CI)
- [ ] No clippy warnings
- [ ] Deployed to staging environment
- [ ] Product owner approval

---

**Document Status:** ✅ COMPLETE  
**Next Update:** Weekly during active development  
**Maintained By:** Only1MCP Development Team  
**Questions:** roadmap@only1mcp.dev
