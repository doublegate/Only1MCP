# Only1MCP Testing & Quality Assurance Strategy
## Comprehensive Test Coverage from Unit to Production

**Document Version:** 1.0  
**Test Framework:** Rust (tokio-test, criterion, proptest, cargo-tarpaulin)  
**Coverage Target:** 80%+ for critical paths  
**Date:** October 14, 2025  
**Status:** Quality Assurance Specification

---

## TABLE OF CONTENTS

1. [Testing Philosophy](#testing-philosophy)
2. [Test Pyramid Structure](#test-pyramid-structure)
3. [Unit Testing Strategy](#unit-testing-strategy)
4. [Integration Testing](#integration-testing)
5. [End-to-End Testing](#end-to-end-testing)
6. [Performance Testing](#performance-testing)
7. [Security Testing](#security-testing)
8. [Test Infrastructure & CI/CD](#test-infrastructure--cicd)
9. [Quality Gates](#quality-gates)
10. [Test Data Management](#test-data-management)

---

## TESTING PHILOSOPHY

### Core Principles

**1. Test Behavior, Not Implementation**
```rust
// ❌ BAD: Testing implementation details
#[test]
fn test_registry_uses_hashmap() {
    let registry = ServerRegistry::new();
    assert!(registry.inner.is_empty());  // Exposes internal structure
}

// ✅ GOOD: Testing behavior
#[test]
fn test_registry_starts_empty() {
    let registry = ServerRegistry::new();
    assert_eq!(registry.list_servers().len(), 0);
}
```

**2. Fast Feedback Loops**
- Unit tests run in <5 seconds
- Integration tests run in <30 seconds
- Full test suite (including E2E) runs in <5 minutes
- Use test parallelization (`cargo nextest`)

**3. Deterministic Tests**
- No flaky tests tolerated
- Mock external dependencies
- Use fixed timestamps/UUIDs in tests
- Isolate filesystem/network operations

**4. Test as Documentation**
```rust
/// Test demonstrating typical usage pattern
#[tokio::test]
async fn test_proxy_handles_concurrent_requests() {
    // Setup: Start proxy with 2 backend servers
    let proxy = TestProxy::builder()
        .with_backend("server1", mock_server_1())
        .with_backend("server2", mock_server_2())
        .build()
        .await;
    
    // Action: Send 100 concurrent requests
    let futures: Vec<_> = (0..100)
        .map(|i| proxy.call_tool(&format!("tool_{}", i % 2)))
        .collect();
    let results = join_all(futures).await;
    
    // Assert: All requests succeed
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 100);
    
    // Verify: Load balanced evenly
    assert!(proxy.backend_call_count("server1") >= 45);
    assert!(proxy.backend_call_count("server2") >= 45);
}
```

**5. Shift-Left Testing**
- Catch bugs early with compile-time checks (Rust's type system)
- Run linters/formatters pre-commit
- Fast unit tests in development loop
- Integration tests before PR

---

## TEST PYRAMID STRUCTURE

### Distribution (Recommended)

```
        ╱╲
       ╱  ╲      E2E Tests (5%)
      ╱────╲     ~20 scenarios
     ╱      ╲    
    ╱────────╲   Integration Tests (25%)
   ╱          ╲  ~100 scenarios
  ╱────────────╲
 ╱              ╲ Unit Tests (70%)
╱────────────────╲ ~500 tests
```

### By Test Type

| Type | Count | Avg Runtime | Total Time | Run When |
|------|-------|-------------|------------|----------|
| Unit | 500 | 5ms | 2.5s | Every commit |
| Integration | 100 | 100ms | 10s | Every commit |
| E2E | 20 | 5s | 100s | Pre-merge, nightly |
| Performance | 10 | 30s | 5min | Weekly, pre-release |
| Security | 5 | 60s | 5min | Pre-release, monthly |

**Total for CI (unit + integration): ~12.5 seconds**

---

## UNIT TESTING STRATEGY

### Scope
- Individual functions and methods
- Logic branches (if/else, match arms)
- Error handling paths
- Edge cases and boundary conditions

### Structure (Arrange-Act-Assert)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_consistent_hash_distributes_evenly() {
        // Arrange: Set up hash ring with 3 servers
        let mut hash = ConsistentHash::new(200); // 200 virtual nodes
        hash.add_server("server1");
        hash.add_server("server2");
        hash.add_server("server3");
        
        // Act: Hash 1000 keys and count distribution
        let mut counts = HashMap::new();
        for i in 0..1000 {
            let server = hash.get_server(&format!("key_{}", i)).unwrap();
            *counts.entry(server.clone()).or_insert(0) += 1;
        }
        
        // Assert: Each server gets 20-40% of keys (allow variance)
        for count in counts.values() {
            assert!(*count >= 200 && *count <= 400, 
                    "Uneven distribution: {}", count);
        }
    }
}
```

### Testing Async Code

```rust
// Using tokio-test for synchronous assertion of async behavior
use tokio_test::{assert_pending, assert_ready, task};

#[test]
fn test_request_timeout() {
    let mut task = task::spawn(async {
        tokio::time::timeout(
            Duration::from_secs(1),
            never_resolves()  // Simulates hung backend
        ).await
    });
    
    // Should be pending initially
    assert_pending!(task.poll());
    
    // After 1 second, should resolve with timeout error
    task.advance_time(Duration::from_secs(1));
    assert_ready!(task.poll()).unwrap_err();
}

// Using #[tokio::test] for integration-style unit tests
#[tokio::test]
async fn test_server_registry_concurrent_access() {
    let registry = ServerRegistry::new();
    
    // Spawn 10 concurrent tasks adding servers
    let tasks: Vec<_> = (0..10)
        .map(|i| {
            let reg = registry.clone();
            tokio::spawn(async move {
                reg.add_server(ServerInfo {
                    id: format!("server{}", i),
                    name: format!("Server {}", i),
                    ..Default::default()
                }).await
            })
        })
        .collect();
    
    // Wait for all to complete
    for task in tasks {
        task.await.unwrap().unwrap();
    }
    
    // Verify all servers added
    assert_eq!(registry.list_servers().await.len(), 10);
}
```

### Property-Based Testing (proptest)

**Use Cases:**
- Parsing/serialization logic
- Data structure invariants
- Compression/decompression round-trips

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_config_roundtrip(config in arbitrary_config()) {
        // Property: Deserialize(Serialize(x)) == x
        let yaml = serde_yaml::to_string(&config).unwrap();
        let parsed: Config = serde_yaml::from_str(&yaml).unwrap();
        prop_assert_eq!(config, parsed);
    }
    
    #[test]
    fn test_cache_key_deterministic(request in arbitrary_request()) {
        // Property: Same request always produces same cache key
        let key1 = cache_key(&request);
        let key2 = cache_key(&request);
        prop_assert_eq!(key1, key2);
    }
}

fn arbitrary_config() -> impl Strategy<Value = Config> {
    (
        prop::string::string_regex("[a-z0-9-]{1,20}").unwrap(),
        prop::collection::vec(arbitrary_server_config(), 1..10)
    ).prop_map(|(version, servers)| Config {
        version,
        servers,
        ..Default::default()
    })
}
```

### Test Organization

```
src/
├── proxy/
│   ├── router.rs
│   ├── router_tests.rs        # Colocated tests
│   ├── registry.rs
│   └── registry_tests.rs
```

OR (module-level tests):

```rust
// src/proxy/router.rs
pub fn route_request(...) { ... }

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_route_request() { ... }
}
```

**Recommendation:** Use colocated test files for complex modules (>500 LOC), inline `mod tests` for smaller ones.

### Mocking Dependencies

```rust
// Using mockall for trait-based mocking
use mockall::predicate::*;
use mockall::*;

#[automock]
trait HttpClient {
    async fn post(&self, url: &str, body: &Value) -> Result<Response>;
}

#[tokio::test]
async fn test_proxy_retries_on_failure() {
    let mut mock_client = MockHttpClient::new();
    
    // First call fails, second succeeds
    mock_client.expect_post()
        .times(1)
        .returning(|_, _| Err(Error::Timeout));
    mock_client.expect_post()
        .times(1)
        .returning(|_, _| Ok(Response { status: 200, body: json!({}) }));
    
    let proxy = Proxy::new(Arc::new(mock_client));
    let result = proxy.forward_request(&test_request()).await;
    
    assert!(result.is_ok());
}
```

---

## INTEGRATION TESTING

### Scope
- Multi-module interactions
- Real filesystem/network I/O (but isolated)
- STDIO process spawning
- Configuration loading
- Hot reload mechanisms

### Test Harness Design

```rust
// tests/common/mod.rs
pub struct TestHarness {
    pub proxy: TestProxy,
    pub mock_servers: Vec<MockMcpServer>,
    pub temp_dir: TempDir,
}

impl TestHarness {
    pub async fn new() -> Self {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.yaml");
        
        // Write test config
        std::fs::write(&config_path, TEST_CONFIG_YAML).unwrap();
        
        // Start mock servers
        let mock_servers = vec![
            MockMcpServer::start_http("http://localhost:9001").await,
            MockMcpServer::start_stdio("npx", &["@mock/server"]).await,
        ];
        
        // Start proxy
        let proxy = TestProxy::start(&config_path).await;
        
        Self { proxy, mock_servers, temp_dir }
    }
    
    pub async fn shutdown(self) {
        self.proxy.shutdown().await;
        for server in self.mock_servers {
            server.shutdown().await;
        }
    }
}

// tests/integration_test.rs
#[tokio::test]
async fn test_end_to_end_tool_call() {
    let harness = TestHarness::new().await;
    
    // Configure mock server to return specific response
    harness.mock_servers[0]
        .expect_tool_call("web_search")
        .with_args(json!({"query": "rust"}))
        .return_response(json!({"results": [...]}));
    
    // Send request through proxy
    let response = harness.proxy
        .call_tool("web_search", json!({"query": "rust"}))
        .await
        .unwrap();
    
    // Verify
    assert_eq!(response["results"].as_array().unwrap().len(), 10);
    
    harness.shutdown().await;
}
```

### Filesystem Testing

```rust
use tempfile::TempDir;

#[test]
fn test_config_hot_reload() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.yaml");
    
    // Write initial config
    std::fs::write(&config_path, "version: \"1.0\"\nservers: []").unwrap();
    
    let (_manager, mut rx) = HotReloadManager::new(config_path.clone()).unwrap();
    
    // Modify config
    std::fs::write(&config_path, "version: \"1.0\"\nservers:\n  - id: new_server").unwrap();
    
    // Wait for reload notification
    tokio::time::sleep(Duration::from_millis(500)).await;
    rx.changed().await.unwrap();
    
    let new_config = rx.borrow().clone();
    assert_eq!(new_config.servers.len(), 1);
}
```

### STDIO Process Testing

```rust
#[tokio::test]
async fn test_stdio_transport_real_server() {
    // Assumes @modelcontextprotocol/server-filesystem is installed
    let config = ServerConfig {
        command: Some("npx".to_string()),
        args: vec![
            "@modelcontextprotocol/server-filesystem".to_string(),
            "/tmp/test_files".to_string(),
        ],
        ..Default::default()
    };
    
    let mut transport = StdioTransport::spawn(&config).await.unwrap();
    
    // Send initialize request
    let init_request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: 1,
        method: "initialize".to_string(),
        params: json!({
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {
                "name": "only1mcp-test",
                "version": "0.1.0"
            }
        }),
    };
    
    let response = transport.send_request(&init_request)
        .await
        .unwrap();
    
    assert_eq!(response.result["protocolVersion"], "2025-06-18");
    
    transport.shutdown().await;
}
```

### Database Testing (Future)

```rust
// When adding persistence layer
use sqlx::PgPool;

#[tokio::test]
async fn test_audit_log_persistence() {
    let pool = PgPool::connect("postgresql://localhost/only1mcp_test").await.unwrap();
    
    // Run migrations
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    
    // Insert audit log
    let log = AuditLog { ... };
    insert_audit_log(&pool, &log).await.unwrap();
    
    // Query back
    let logs = query_audit_logs(&pool, &log.user_id).await.unwrap();
    assert_eq!(logs.len(), 1);
    
    // Cleanup
    sqlx::query("TRUNCATE TABLE audit_logs").execute(&pool).await.unwrap();
}
```

---

## END-TO-END TESTING

### Scope
- Full user workflows
- Real AI client interactions (Claude Desktop, Cursor)
- Multi-server scenarios
- Error recovery

### E2E Test Scenarios

**Scenario 1: First-Time Setup**
```rust
#[tokio::test]
#[ignore] // Run with: cargo test --ignored
async fn test_e2e_first_time_setup() {
    // 1. User installs binary
    assert!(Command::new("only1mcp").arg("--version").status().unwrap().success());
    
    // 2. User creates config
    let config = r#"
    version: "1.0"
    servers:
      - id: filesystem
        transport: stdio
        command: npx
        args: ["@modelcontextprotocol/server-filesystem", "/tmp"]
    "#;
    std::fs::write("test_config.yaml", config).unwrap();
    
    // 3. User starts proxy
    let mut proxy_process = Command::new("only1mcp")
        .arg("start")
        .arg("--config")
        .arg("test_config.yaml")
        .spawn()
        .unwrap();
    
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    // 4. User sends request (simulate AI client)
    let client = reqwest::Client::new();
    let response = client.post("http://localhost:8080/mcp")
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/list"
        }))
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 200);
    
    // 5. Cleanup
    proxy_process.kill().unwrap();
    std::fs::remove_file("test_config.yaml").unwrap();
}
```

**Scenario 2: Hot Server Addition**
```rust
#[tokio::test]
#[ignore]
async fn test_e2e_hot_add_server() {
    let harness = E2EHarness::new().await;
    
    // Initial state: 1 server, 5 tools
    let tools = harness.list_tools().await;
    assert_eq!(tools.len(), 5);
    
    // Add new server via CLI
    Command::new("only1mcp")
        .args(["add", "new-server", "http://localhost:9002"])
        .status()
        .unwrap();
    
    tokio::time::sleep(Duration::from_secs(1)).await;
    
    // Verify new tools available without restart
    let tools = harness.list_tools().await;
    assert_eq!(tools.len(), 10);
    
    harness.shutdown().await;
}
```

**Scenario 3: Failover Under Load**
```rust
#[tokio::test]
#[ignore]
async fn test_e2e_failover_under_load() {
    let harness = E2EHarness::new_with_redundancy().await; // 2 instances of each server
    
    // Start load generator (100 req/s)
    let load = harness.start_load_generator(100).await;
    
    // Kill primary server after 10 seconds
    tokio::time::sleep(Duration::from_secs(10)).await;
    harness.mock_servers[0].crash().await;
    
    // Load should continue without errors
    tokio::time::sleep(Duration::from_secs(10)).await;
    load.stop().await;
    
    // Verify: <1% error rate
    assert!(load.error_rate() < 0.01);
    
    harness.shutdown().await;
}
```

### Running E2E Tests

```bash
# In CI (nightly)
cargo test --ignored --test-threads=1

# Locally
cargo test --ignored -- --nocapture test_e2e_first_time_setup
```

---

## PERFORMANCE TESTING

### Benchmarking with Criterion

**Setup:**
```toml
# Cargo.toml
[dev-dependencies]
criterion = { version = "0.5", features = ["html_reports", "async_tokio"] }

[[bench]]
name = "proxy_benchmarks"
harness = false
```

**Micro-Benchmarks:**
```rust
// benches/proxy_benchmarks.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use criterion::async_executor::FuturesExecutor;

fn bench_consistent_hash(c: &mut Criterion) {
    let mut hash = ConsistentHash::new(200);
    for i in 0..10 {
        hash.add_server(&format!("server{}", i));
    }
    
    c.bench_function("consistent_hash_lookup", |b| {
        b.iter(|| {
            hash.get_server(black_box("test_key"))
        })
    });
}

fn bench_proxy_forwarding(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (proxy, _mock) = rt.block_on(setup_test_proxy());
    
    c.bench_function("proxy_forward_request", |b| {
        b.to_async(&rt).iter(|| async {
            proxy.forward_request(black_box(&test_request())).await
        })
    });
}

criterion_group!(benches, bench_consistent_hash, bench_proxy_forwarding);
criterion_main!(benches);
```

**Macro-Benchmarks:**
```rust
fn bench_concurrent_load(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (proxy, _mock) = rt.block_on(setup_test_proxy());
    
    let mut group = c.benchmark_group("concurrent_load");
    
    for concurrency in [1, 10, 100, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(concurrency),
            concurrency,
            |b, &n| {
                b.to_async(&rt).iter(|| async move {
                    let futures: Vec<_> = (0..n)
                        .map(|_| proxy.forward_request(&test_request()))
                        .collect();
                    futures::future::join_all(futures).await
                });
            }
        );
    }
    
    group.finish();
}
```

**Running Benchmarks:**
```bash
# Run all benchmarks
cargo bench

# Run specific benchmark
cargo bench -- consistent_hash

# Generate flamegraph
cargo flamegraph --bench proxy_benchmarks
```

**CI Integration:**
```yaml
# .github/workflows/benchmark.yml
name: Benchmark
on:
  pull_request:
    branches: [main]

jobs:
  benchmark:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: dtolnay/rust-toolchain@stable
      
      - name: Run benchmarks
        run: cargo bench --bench proxy_benchmarks -- --save-baseline pr
      
      - name: Compare with main
        run: |
          git checkout main
          cargo bench --bench proxy_benchmarks -- --save-baseline main
          cargo bench --bench proxy_benchmarks -- --baseline main
```

### Load Testing (wrk)

```bash
# Install wrk
brew install wrk  # macOS
sudo apt install wrk  # Ubuntu

# Run load test
wrk -t4 -c100 -d30s --latency \
    -s scripts/mcp_request.lua \
    http://localhost:8080/mcp

# scripts/mcp_request.lua
wrk.method = "POST"
wrk.headers["Content-Type"] = "application/json"
wrk.body = [[
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "test_tool",
    "arguments": {}
  }
}
]]
```

**Expected Results (Target):**
```
Requests/sec:  10000+
Latency (50%): <2ms
Latency (99%): <5ms
Errors:        <0.1%
```

### Soak Testing

```bash
# Run for 24 hours to detect memory leaks
cargo build --release
./target/release/only1mcp start --config config.yaml &
PROXY_PID=$!

# Monitor memory usage
while true; do
  ps aux | grep $PROXY_PID | awk '{print $6}'
  sleep 60
done > memory_usage.log

# Generate steady load
wrk -t4 -c50 -d24h http://localhost:8080/mcp
```

---

## SECURITY TESTING

### Static Analysis

**Clippy (Enhanced):**
```bash
# Run with pedantic lints
cargo clippy -- \
  -W clippy::all \
  -W clippy::pedantic \
  -W clippy::nursery \
  -W clippy::cargo \
  -A clippy::missing_errors_doc  # Adjust as needed
```

**Dependency Audits:**
```bash
# Install cargo-audit
cargo install cargo-audit

# Check for known vulnerabilities
cargo audit

# Automated in CI (daily)
# .github/workflows/security.yml
- name: Security audit
  uses: actions-rust-lang/audit@v1
```

### Dynamic Analysis

**Fuzzing (cargo-fuzz):**
```bash
# Install
cargo install cargo-fuzz

# Generate fuzz target
cargo fuzz init

# Fuzz config parser
# fuzz/fuzz_targets/config_parser.rs
#![no_main]
use libfuzzer_sys::fuzz_target;
use only1mcp::config::Config;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = serde_yaml::from_str::<Config>(s);
    }
});

# Run fuzzer
cargo fuzz run config_parser
```

**OWASP ZAP Scanning:**
```bash
# Run proxy
./target/release/only1mcp start &

# Run ZAP baseline scan
docker run -t owasp/zap2docker-stable zap-baseline.py \
  -t http://localhost:8080 \
  -r zap_report.html
```

### Penetration Testing Checklist

**Manual Tests:**
- [ ] SQL injection in tool arguments
- [ ] Command injection in STDIO commands
- [ ] Path traversal in resource URIs
- [ ] SSRF via backend URLs
- [ ] JWT token forgery
- [ ] Rate limit bypass
- [ ] Authentication bypass
- [ ] Privilege escalation (RBAC)

**Tools:**
- Burp Suite: Manual testing
- sqlmap: SQL injection
- nmap: Port scanning
- Nessus: Vulnerability scanning (enterprise)

---

## TEST INFRASTRUCTURE & CI/CD

### GitHub Actions Workflow

```yaml
# .github/workflows/ci.yml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  test:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]
        rust: [stable, nightly]
    
    steps:
      - uses: actions/checkout@v3
      
      - name: Install Rust
        uses: dtolnay/rust-toolchain@master
        with:
          toolchain: ${{ matrix.rust }}
          components: rustfmt, clippy
      
      - name: Cache dependencies
        uses: Swatinem/rust-cache@v2
      
      - name: Check formatting
        run: cargo fmt --all -- --check
      
      - name: Clippy
        run: cargo clippy --all-targets -- -D warnings
      
      - name: Build
        run: cargo build --verbose
      
      - name: Run unit tests
        run: cargo test --lib
      
      - name: Run integration tests
        run: cargo test --test '*'
      
      - name: Generate coverage
        if: matrix.os == 'ubuntu-latest' && matrix.rust == 'stable'
        run: |
          cargo install cargo-tarpaulin
          cargo tarpaulin --out Xml
      
      - name: Upload coverage
        if: matrix.os == 'ubuntu-latest' && matrix.rust == 'stable'
        uses: codecov/codecov-action@v3
        with:
          files: ./cobertura.xml

  benchmark:
    runs-on: ubuntu-latest
    if: github.event_name == 'pull_request'
    steps:
      - uses: actions/checkout@v3
      - uses: dtolnay/rust-toolchain@stable
      
      - name: Run benchmarks
        run: cargo bench -- --save-baseline pr
      
      - name: Compare with main
        run: |
          git fetch origin main
          git checkout origin/main
          cargo bench -- --save-baseline main
          git checkout -
          cargo bench -- --baseline main --load-baseline pr
```

### Pre-Commit Hooks

```bash
# .git/hooks/pre-commit
#!/bin/bash
set -e

echo "Running pre-commit checks..."

# Format check
cargo fmt -- --check

# Clippy
cargo clippy --all-targets -- -D warnings

# Quick tests
cargo test --lib

echo "✅ All checks passed!"
```

Install with:
```bash
chmod +x .git/hooks/pre-commit
# Or use husky/pre-commit framework
```

### Test Reporting

**JUnit XML for CI:**
```bash
# Install cargo2junit
cargo install cargo2junit

# Run tests with JSON output
cargo test -- -Z unstable-options --format json | cargo2junit > results.xml

# Upload to GitHub Actions
# (automatically parsed and displayed)
```

---

## QUALITY GATES

### PR Merge Requirements

**Automated Checks (Must Pass):**
- ✅ All unit tests passing
- ✅ All integration tests passing
- ✅ Code coverage ≥80% for modified files
- ✅ No clippy warnings
- ✅ Code formatted (rustfmt)
- ✅ Benchmarks: No >5% regression
- ✅ Security audit clean

**Manual Checks:**
- ✅ Code review approved by 1+ maintainer
- ✅ Documentation updated (if public API changed)
- ✅ CHANGELOG.md entry added

**Release Criteria (Additional):**
- ✅ All E2E tests passing
- ✅ Load testing completed (10k req/s achieved)
- ✅ Security scan clean (ZAP, cargo-audit)
- ✅ Cross-platform builds successful
- ✅ Documentation complete

### Coverage Targets

| Component | Target Coverage |
|-----------|----------------|
| Core Proxy | 90%+ |
| Server Registry | 85%+ |
| Transports | 80%+ |
| Auth/Security | 95%+ |
| CLI | 70%+ |
| **Overall** | **80%+** |

---

## TEST DATA MANAGEMENT

### Fixtures

```rust
// tests/fixtures/mod.rs
pub fn sample_config() -> Config {
    Config {
        version: "1.0".to_string(),
        servers: vec![
            ServerConfig {
                id: "test-server".to_string(),
                name: "Test Server".to_string(),
                transport: "http".to_string(),
                url: Some("http://localhost:9000".to_string()),
                ..Default::default()
            }
        ],
        proxy: ProxyConfig {
            host: "127.0.0.1".to_string(),
            port: 8080,
            ..Default::default()
        },
    }
}

pub fn sample_json_rpc_request() -> JsonRpcRequest {
    JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: 1,
        method: "tools/call".to_string(),
        params: json!({
            "name": "test_tool",
            "arguments": {}
        }),
    }
}
```

### Test Databases (Future)

```sql
-- tests/fixtures/schema.sql
CREATE TABLE audit_logs (
    id SERIAL PRIMARY KEY,
    timestamp TIMESTAMPTZ NOT NULL,
    user_id VARCHAR(255) NOT NULL,
    tool_name VARCHAR(255) NOT NULL,
    result VARCHAR(50) NOT NULL
);

-- Insert test data
INSERT INTO audit_logs (timestamp, user_id, tool_name, result)
VALUES 
    (NOW(), 'test_user', 'web_search', 'success'),
    (NOW(), 'test_user', 'file_read', 'error');
```

---

## APPENDIX A: TEST COMMANDS REFERENCE

```bash
# Unit tests
cargo test --lib

# Integration tests
cargo test --test '*'

# E2E tests (ignored by default)
cargo test --ignored

# Specific test
cargo test test_consistent_hash

# With output
cargo test -- --nocapture

# Single-threaded (for debugging)
cargo test -- --test-threads=1

# Benchmarks
cargo bench
cargo bench -- --save-baseline my_baseline

# Coverage
cargo tarpaulin --out Html

# Fuzzing
cargo fuzz run config_parser

# Load testing
wrk -t4 -c100 -d30s http://localhost:8080/mcp
```

---

## APPENDIX B: MOCK SERVER IMPLEMENTATION

```rust
// tests/common/mock_mcp_server.rs
use axum::{Router, routing::post, Json};
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct MockMcpServer {
    pub url: String,
    pub expectations: Arc<Mutex<Vec<Expectation>>>,
    shutdown_tx: tokio::sync::oneshot::Sender<()>,
}

struct Expectation {
    method: String,
    params: Value,
    response: Value,
}

impl MockMcpServer {
    pub async fn start_http(url: &str) -> Self {
        let expectations = Arc::new(Mutex::new(Vec::new()));
        let expectations_clone = expectations.clone();
        
        let app = Router::new()
            .route("/mcp", post(move |Json(req): Json<Value>| {
                let exp = expectations_clone.clone();
                async move {
                    let mut exps = exp.lock().await;
                    for (i, expectation) in exps.iter().enumerate() {
                        if req["method"] == expectation.method 
                           && req["params"] == expectation.params {
                            let response = expectation.response.clone();
                            exps.remove(i);
                            return Json(response);
                        }
                    }
                    Json(json!({"error": "Unexpected request"}))
                }
            }));
        
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        
        tokio::spawn(async move {
            axum::Server::bind(&url.parse().unwrap())
                .serve(app.into_make_service())
                .with_graceful_shutdown(async {
                    shutdown_rx.await.ok();
                })
                .await
                .unwrap();
        });
        
        Self {
            url: url.to_string(),
            expectations,
            shutdown_tx,
        }
    }
    
    pub async fn expect_tool_call(&self, tool_name: &str) -> ExpectationBuilder {
        ExpectationBuilder {
            server: self,
            method: "tools/call".to_string(),
            tool_name: Some(tool_name.to_string()),
            args: None,
            response: None,
        }
    }
    
    pub async fn shutdown(self) {
        let _ = self.shutdown_tx.send(());
    }
}

pub struct ExpectationBuilder<'a> {
    server: &'a MockMcpServer,
    method: String,
    tool_name: Option<String>,
    args: Option<Value>,
    response: Option<Value>,
}

impl<'a> ExpectationBuilder<'a> {
    pub fn with_args(mut self, args: Value) -> Self {
        self.args = Some(args);
        self
    }
    
    pub async fn return_response(mut self, response: Value) {
        self.response = Some(response);
        
        let expectation = Expectation {
            method: self.method,
            params: json!({
                "name": self.tool_name.unwrap(),
                "arguments": self.args.unwrap_or(json!({}))
            }),
            response: json!({
                "jsonrpc": "2.0",
                "id": 1,
                "result": self.response.unwrap()
            }),
        };
        
        self.server.expectations.lock().await.push(expectation);
    }
}
```

---

**Document Status:** ✅ COMPLETE  
**Next Review:** Monthly during active development  
**Maintained By:** Only1MCP QA Team  
**Questions:** testing@only1mcp.dev
