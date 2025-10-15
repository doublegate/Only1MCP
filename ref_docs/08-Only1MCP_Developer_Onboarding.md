# Only1MCP Developer Onboarding & Contributing Guide
## Getting Started with Only1MCP Development

**Document Version:** 1.0  
**Target Audience:** New contributors, core team members, external developers  
**Prerequisites:** Intermediate Rust knowledge, familiarity with async programming  
**Date:** October 14, 2025  
**Status:** Living Document - Updated with each major release

---

## TABLE OF CONTENTS

1. [Welcome to Only1MCP](#welcome-to-only1mcp)
2. [Development Environment Setup](#development-environment-setup)
3. [Codebase Architecture Overview](#codebase-architecture-overview)
4. [Development Workflow](#development-workflow)
5. [Coding Standards & Best Practices](#coding-standards--best-practices)
6. [Testing Guidelines](#testing-guidelines)
7. [Documentation Requirements](#documentation-requirements)
8. [Pull Request Process](#pull-request-process)
9. [Common Development Tasks](#common-development-tasks)
10. [Troubleshooting & FAQ](#troubleshooting--faq)

---

## WELCOME TO ONLY1MCP

### Project Mission

Only1MCP is the **fastest, most intelligent MCP (Model Context Protocol) server aggregator** designed to solve critical pain points in AI tool integration:

- **Performance**: 30-60% faster than existing solutions through Rust-native implementation
- **Context Efficiency**: Unique 50-70% reduction in token consumption via intelligent caching and batching
- **Zero Configuration**: Visual management eliminating manual JSON editing
- **Enterprise-Ready**: RBAC, OAuth2, audit logging - all open-source

### Why Contribute?

- **Impact**: Your code will directly improve AI agent efficiency for thousands of developers
- **Learning**: Work with cutting-edge async Rust, distributed systems, and AI protocols
- **Community**: Join a growing ecosystem around Model Context Protocol
- **Recognition**: All contributors credited in releases and documentation

### Code of Conduct

We follow the [Contributor Covenant](https://www.contributor-covenant.org/version/2/1/code_of_conduct/). In summary:

- Be respectful and inclusive
- Accept constructive criticism
- Focus on what's best for the community
- Show empathy towards other contributors

Report violations to conduct@only1mcp.dev.

---

## DEVELOPMENT ENVIRONMENT SETUP

### System Requirements

**Operating System:**
- Linux (Ubuntu 22.04+, Fedora 38+, Arch)
- macOS 12+ (Intel or Apple Silicon)
- Windows 11 with WSL2

**Hardware (Recommended):**
- CPU: 4+ cores
- RAM: 8GB minimum, 16GB recommended
- Disk: 20GB free space (for Rust toolchain + dependencies)
- Internet: Required for initial setup and MCP server testing

### Step 1: Install Rust Toolchain

```bash
# Install rustup (Rust installer)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Default to stable channel
rustup default stable

# Verify installation
rustc --version  # Should be 1.70+
cargo --version

# Install additional components
rustup component add rustfmt clippy
```

**Alternative (for specific versions):**
```bash
# Install specific stable version
rustup install 1.75.0
rustup default 1.75.0

# Install nightly (for experimental features)
rustup install nightly
rustup component add rustfmt clippy --toolchain nightly
```

### Step 2: Install Development Tools

**Essential Tools:**
```bash
# Git (if not already installed)
# Ubuntu/Debian
sudo apt-get install git

# macOS
brew install git

# Windows (via Chocolatey)
choco install git

# cargo-watch (auto-rebuild on file changes)
cargo install cargo-watch

# cargo-nextest (faster test runner)
cargo install cargo-nextest

# cargo-tarpaulin (code coverage)
cargo install cargo-tarpaulin

# cargo-audit (security audit)
cargo install cargo-audit
```

**Optional but Recommended:**
```bash
# bacon (background compiler + test runner with UI)
cargo install bacon

# cargo-expand (macro expansion viewer)
cargo install cargo-expand

# cargo-flamegraph (performance profiling)
cargo install flamegraph

# tokio-console (async runtime inspector)
cargo install tokio-console

# cargo-udeps (unused dependency detection)
cargo install cargo-udeps

# cargo-deny (dependency license/security checks)
cargo install cargo-deny
```

### Step 3: IDE/Editor Setup

**Visual Studio Code (Recommended):**

1. Install [VS Code](https://code.visualstudio.com/)
2. Install extensions:
   ```json
   {
     "recommendations": [
       "rust-lang.rust-analyzer",        // Rust language server
       "vadimcn.vscode-lldb",            // Debugger
       "serayuzgur.crates",              // Cargo.toml management
       "tamasfe.even-better-toml",       // TOML syntax
       "streetsidesoftware.code-spell-checker", // Spell check
       "EditorConfig.EditorConfig"       // Code formatting
     ]
   }
   ```
3. Configure `settings.json`:
   ```json
   {
     "rust-analyzer.checkOnSave.command": "clippy",
     "rust-analyzer.cargo.features": "all",
     "rust-analyzer.inlayHints.enable": true,
     "editor.formatOnSave": true,
     "editor.rulers": [100],
     "files.insertFinalNewline": true,
     "files.trimTrailingWhitespace": true
   }
   ```

**Alternative IDEs:**
- **IntelliJ IDEA / CLion**: Install Rust plugin
- **Vim/Neovim**: Use CoC + rust-analyzer or native LSP
- **Emacs**: Use rustic-mode or lsp-mode

### Step 4: Clone Repository

```bash
# Clone via HTTPS
git clone https://github.com/doublegate/Only1MCP.git

# OR via SSH (if you have GitHub SSH keys)
git clone git@github.com:doublegate/Only1MCP.git

# Enter directory
cd only1mcp

# Verify structure
ls -la
# You should see: Cargo.toml, src/, tests/, docs/, etc.
```

### Step 5: Build Project

```bash
# First build (downloads dependencies, may take 5-10 minutes)
cargo build

# Optimized build (for testing performance)
cargo build --release

# Verify build succeeded
./target/debug/only1mcp --version
```

**Common Build Issues:**

| Error | Solution |
|-------|----------|
| `linker 'cc' not found` | Install build-essential (Ubuntu: `sudo apt install build-essential`) |
| `openssl-sys` errors | Install libssl-dev (Ubuntu: `sudo apt install libssl-dev pkg-config`) |
| `Out of memory` | Reduce parallel jobs: `cargo build -j 2` |
| Slow incremental builds | Clear cache: `cargo clean` and rebuild |

### Step 6: Run Tests

```bash
# Run all tests (unit + integration)
cargo test

# Run with output visible
cargo test -- --nocapture

# Run specific test
cargo test test_consistent_hash

# Run tests in parallel with nextest (faster)
cargo nextest run

# Generate code coverage
cargo tarpaulin --out Html --output-dir ./coverage
open coverage/index.html  # View report
```

### Step 7: Install MCP Test Servers

For integration testing, you need real MCP servers:

```bash
# Install Node.js (if not installed)
# Ubuntu
sudo apt install nodejs npm

# macOS
brew install node

# Install official MCP servers
npm install -g @modelcontextprotocol/server-filesystem
npm install -g @modelcontextprotocol/server-memory
npm install -g @modelcontextprotocol/server-github

# Verify installation
npx @modelcontextprotocol/server-filesystem --version
```

### Step 8: Configure Test Environment

```bash
# Copy example config
cp config.example.yaml config.test.yaml

# Edit to point to local MCP servers
# config.test.yaml
cat > config.test.yaml << 'EOF'
version: "1.0"

proxy:
  host: "127.0.0.1"
  port: 8081  # Use different port for dev

servers:
  - id: "test-filesystem"
    name: "Test Filesystem"
    transport: "stdio"
    command: "npx"
    args:
      - "@modelcontextprotocol/server-filesystem"
      - "/tmp/only1mcp-test-files"

  - id: "test-memory"
    name: "Test Memory"
    transport: "stdio"
    command: "npx"
    args:
      - "@modelcontextprotocol/server-memory"

logging:
  level: "debug"
  format: "pretty"  # Human-readable for dev
EOF

# Create test file directory
mkdir -p /tmp/only1mcp-test-files
echo "Test file" > /tmp/only1mcp-test-files/hello.txt
```

### Step 9: Run Development Server

```bash
# Run with auto-reload (watches for file changes)
cargo watch -x 'run -- start --config config.test.yaml'

# OR run directly
cargo run -- start --config config.test.yaml

# In another terminal, test it
curl -X POST http://localhost:8081/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "tools/list"
  }'
```

You should see a JSON response listing available tools!

### Step 10: Set Up Pre-Commit Hooks

```bash
# Install pre-commit framework (optional but recommended)
pip install pre-commit

# Or via your package manager
brew install pre-commit  # macOS
sudo apt install pre-commit  # Ubuntu

# Install hooks
pre-commit install

# Test hooks manually
pre-commit run --all-files
```

Our `.pre-commit-config.yaml`:
```yaml
repos:
  - repo: local
    hooks:
      - id: cargo-fmt
        name: cargo fmt
        entry: cargo fmt --all -- --check
        language: system
        types: [rust]
        pass_filenames: false
      
      - id: cargo-clippy
        name: cargo clippy
        entry: cargo clippy --all-targets --all-features -- -D warnings
        language: system
        types: [rust]
        pass_filenames: false
      
      - id: cargo-test
        name: cargo test
        entry: cargo test --lib
        language: system
        types: [rust]
        pass_filenames: false
```

---

## CODEBASE ARCHITECTURE OVERVIEW

### Directory Structure

```
only1mcp/
├── Cargo.toml              # Project manifest
├── Cargo.lock              # Dependency lock file
├── README.md               # Project overview
├── CONTRIBUTING.md         # This document (symlink)
├── LICENSE-APACHE          # Apache 2.0 license
├── LICENSE-MIT             # MIT license
├── .github/
│   ├── workflows/          # CI/CD pipelines
│   │   ├── ci.yml
│   │   ├── release.yml
│   │   └── security.yml
│   └── ISSUE_TEMPLATE/
├── src/
│   ├── main.rs             # CLI entry point
│   ├── lib.rs              # Library root
│   ├── config/             # Configuration management
│   │   ├── mod.rs
│   │   ├── schema.rs       # YAML/TOML schemas
│   │   ├── loader.rs       # File loading
│   │   └── validator.rs    # Config validation
│   ├── proxy/              # Core proxy logic
│   │   ├── mod.rs
│   │   ├── router.rs       # Request routing
│   │   ├── registry.rs     # Server registry
│   │   ├── cache.rs        # Response caching
│   │   └── load_balancer.rs
│   ├── transport/          # MCP transports
│   │   ├── mod.rs
│   │   ├── stdio.rs        # STDIO transport
│   │   ├── http.rs         # HTTP/SSE transport
│   │   └── streamable.rs   # Streamable HTTP
│   ├── auth/               # Authentication
│   │   ├── mod.rs
│   │   ├── jwt.rs
│   │   ├── api_key.rs
│   │   └── rbac.rs
│   ├── health/             # Health checking
│   │   ├── mod.rs
│   │   ├── checker.rs
│   │   └── circuit_breaker.rs
│   ├── metrics/            # Observability
│   │   ├── mod.rs
│   │   ├── prometheus.rs
│   │   └── tracing.rs
│   ├── api/                # Management API
│   │   ├── mod.rs
│   │   ├── admin.rs
│   │   └── handlers.rs
│   ├── error.rs            # Error types
│   └── types.rs            # Shared types
├── tests/
│   ├── common/             # Test utilities
│   │   ├── mod.rs
│   │   ├── harness.rs      # Test harness
│   │   └── mock_server.rs  # Mock MCP server
│   ├── integration/        # Integration tests
│   │   ├── proxy_test.rs
│   │   ├── hot_reload_test.rs
│   │   └── failover_test.rs
│   └── e2e/                # End-to-end tests
│       └── full_workflow_test.rs
├── benches/
│   ├── proxy_benchmarks.rs
│   └── cache_benchmarks.rs
├── examples/
│   ├── basic_usage.rs
│   └── custom_router.rs
├── docs/
│   ├── api/                # API documentation
│   ├── guides/             # User guides
│   └── architecture/       # Architecture docs
├── scripts/
│   ├── deploy.sh
│   └── test-integration.sh
└── config.example.yaml     # Example configuration
```

### Module Dependency Graph

```
main.rs
  ├─> config::loader ───> config::schema
  ├─> proxy::router
  │     ├─> proxy::registry
  │     ├─> transport::*
  │     ├─> auth::*
  │     └─> metrics::*
  ├─> api::handlers
  │     └─> proxy::registry
  └─> health::checker
        └─> proxy::registry
```

### Key Data Structures

**ServerInfo (src/types.rs):**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    /// Unique identifier for the server
    pub id: String,
    
    /// Human-readable name
    pub name: String,
    
    /// Transport type (stdio, http, streamable_http)
    pub transport: Transport,
    
    /// Server URL (for HTTP transports)
    pub url: Option<String>,
    
    /// Command to execute (for STDIO transport)
    pub command: Option<String>,
    
    /// Command arguments
    pub args: Vec<String>,
    
    /// Authentication config
    pub auth: Option<AuthConfig>,
    
    /// Health status
    pub status: ServerStatus,
    
    /// Available tools
    pub tools: Vec<ToolInfo>,
}
```

**ServerRegistry (src/proxy/registry.rs):**
```rust
pub struct ServerRegistry {
    /// Map of server ID to ServerInfo
    servers: Arc<RwLock<HashMap<String, ServerInfo>>>,
    
    /// Consistent hash ring for load balancing
    hash_ring: Arc<RwLock<ConsistentHash>>,
    
    /// Configuration watcher for hot-reload
    config_watcher: watch::Receiver<Config>,
}
```

### Critical Async Patterns

**Request Routing (src/proxy/router.rs):**
```rust
pub async fn route_request(
    State(state): State<AppState>,
    Json(request): Json<McpRequest>,
) -> Result<Json<McpResponse>, ApiError> {
    // 1. Check cache
    let cache_key = compute_cache_key(&request);
    if let Some(cached) = state.cache.get(&cache_key) {
        return Ok(Json(cached));
    }
    
    // 2. Select backend server
    let server_id = state.registry
        .select_server(&request.method)
        .await?;
    
    // 3. Get server info
    let server = state.registry
        .get_server(&server_id)
        .await?;
    
    // 4. Forward request
    let response = forward_to_backend(&server, &request).await?;
    
    // 5. Cache result
    state.cache.insert(cache_key, response.clone());
    
    // 6. Record metrics
    REQUESTS_TOTAL.inc();
    
    Ok(Json(response))
}
```

---

## DEVELOPMENT WORKFLOW

### Git Branching Strategy

We use **GitHub Flow** (simplified Git Flow):

```
main (protected)
  └─> feature/context-optimization
  └─> fix/stdio-memory-leak
  └─> docs/api-reference
```

**Branch Naming:**
- Features: `feature/short-description`
- Bug fixes: `fix/issue-number-short-description`
- Documentation: `docs/short-description`
- Refactoring: `refactor/component-name`

### Typical Development Cycle

**1. Pick an Issue**
```bash
# Find "good first issue" or "help wanted" labels
# https://github.com/doublegate/Only1MCP/issues?q=is%3Aissue+is%3Aopen+label%3A%22good+first+issue%22

# Comment on issue to claim it
# "I'd like to work on this!"
```

**2. Create Feature Branch**
```bash
# Ensure main is up-to-date
git checkout main
git pull origin main

# Create and switch to feature branch
git checkout -b feature/add-prometheus-metrics

# Verify current branch
git branch
```

**3. Implement Feature**
```bash
# Make changes
vim src/metrics/prometheus.rs

# Run tests frequently
cargo test

# Run formatter
cargo fmt

# Run linter
cargo clippy --fix

# Build to ensure no errors
cargo build
```

**4. Write Tests**
```rust
// tests/integration/metrics_test.rs
#[tokio::test]
async fn test_prometheus_metrics_endpoint() {
    let harness = TestHarness::new().await;
    
    // Send some requests
    for _ in 0..10 {
        harness.proxy.call_tool("test_tool", json!({})).await.unwrap();
    }
    
    // Fetch metrics
    let response = reqwest::get("http://localhost:9090/metrics")
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    
    // Verify metrics present
    assert!(response.contains("only1mcp_requests_total"));
    assert!(response.contains("only1mcp_request_duration_seconds"));
    
    harness.shutdown().await;
}
```

**5. Commit Changes**
```bash
# Stage changes
git add src/metrics/prometheus.rs
git add tests/integration/metrics_test.rs

# Commit with descriptive message
git commit -m "feat: Add Prometheus metrics endpoint

- Implement /metrics endpoint using prometheus crate
- Export request count, duration, error rate
- Add integration test verifying metric collection
- Update README with metrics documentation

Closes #123"

# Commit message format:
# <type>: <subject>
#
# <body>
#
# <footer>

# Types: feat, fix, docs, style, refactor, test, chore
```

**6. Push and Create PR**
```bash
# Push branch to remote
git push origin feature/add-prometheus-metrics

# GitHub CLI (if installed)
gh pr create --title "feat: Add Prometheus metrics endpoint" --body "Closes #123"

# OR: Go to GitHub and click "Create Pull Request"
```

### Code Review Process

**As a Contributor:**

1. **Respond to Feedback**: Address all review comments
2. **Push Updates**: Use `git push` (don't force-push unless necessary)
3. **Mark Resolved**: Mark conversations as resolved after fixing
4. **Be Patient**: Reviews may take 1-3 days

**As a Reviewer:**

1. **Be Constructive**: Suggest improvements, don't just criticize
2. **Ask Questions**: "Why did you choose X over Y?" instead of "X is wrong"
3. **Test Locally**: Check out the branch and verify it works
4. **Approve or Request Changes**: Use GitHub review tools

### Merge Process

**Requirements:**
- âœ… All CI checks passing
- âœ… At least 1 approving review from maintainer
- âœ… No unresolved conversations
- âœ… Branch up-to-date with main
- âœ… Code coverage >80% for new code

**Merge Method:** Squash and merge (creates single commit on main)

---

## CODING STANDARDS & BEST PRACTICES

### Rust Style Guide

We follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/) and enforce via `rustfmt` + `clippy`.

**Formatting (rustfmt):**
```toml
# rustfmt.toml
max_width = 100
hard_tabs = false
tab_spaces = 4
newline_style = "Unix"
use_small_heuristics = "Default"
reorder_imports = true
reorder_modules = true
remove_nested_parens = true
edition = "2021"
```

**Linting (clippy):**
```toml
# .clippy.toml
warn-on-all-wildcard-imports = true
disallowed-types = [
    # Prefer `Arc<Mutex<T>>` over `Mutex<Arc<T>>`
    { path = "std::sync::Mutex<std::sync::Arc<T>>", reason = "Prefer Arc<Mutex<T>>" }
]
```

### Naming Conventions

| Item | Convention | Example |
|------|-----------|---------|
| Crates | snake_case | `only1mcp` |
| Modules | snake_case | `proxy::router` |
| Types | UpperCamelCase | `ServerInfo` |
| Traits | UpperCamelCase | `Transport` |
| Functions | snake_case | `route_request()` |
| Methods | snake_case | `get_server()` |
| Constants | SCREAMING_SNAKE_CASE | `MAX_RETRIES` |
| Statics | SCREAMING_SNAKE_CASE | `REQUESTS_TOTAL` |

### Documentation Standards

**File-Level Documentation:**
```rust
//! Proxy routing logic for Only1MCP.
//!
//! This module handles incoming MCP requests and routes them to appropriate
//! backend servers based on the request method and configured routing strategy.
//!
//! # Features
//!
//! - Consistent hashing for load balancing
//! - Response caching with TTL
//! - Automatic failover to healthy backends
//! - Streaming support for long-running requests
//!
//! # Example
//!
//! ```rust
//! use only1mcp::proxy::Router;
//!
//! let router = Router::new(registry, cache);
//! let response = router.route(request).await?;
//! ```

use std::sync::Arc;
use tokio::sync::RwLock;
```

**Function-Level Documentation:**
```rust
/// Routes an MCP request to the appropriate backend server.
///
/// This function performs the following steps:
/// 1. Checks response cache for matching result
/// 2. Selects backend using consistent hashing
/// 3. Forwards request to backend
/// 4. Caches response if successful
/// 5. Records metrics
///
/// # Arguments
///
/// * `request` - The MCP JSON-RPC request to route
///
/// # Returns
///
/// * `Ok(McpResponse)` - Successful response from backend
/// * `Err(ApiError)` - Routing or backend error
///
/// # Errors
///
/// Returns `ApiError::NoBackendAvailable` if all backends are unhealthy.
/// Returns `ApiError::BackendTimeout` if backend exceeds configured timeout.
///
/// # Example
///
/// ```rust
/// let response = router.route_request(request).await?;
/// println!("Response: {:?}", response);
/// ```
pub async fn route_request(
    &self,
    request: &McpRequest,
) -> Result<McpResponse, ApiError> {
    // Implementation...
}
```

### Error Handling

**Use Result Types:**
```rust
// âœ… Good
pub async fn connect_server(config: &ServerConfig) -> Result<Connection, ConnectError> {
    // ...
}

// âŒ Bad (avoid panics in library code)
pub async fn connect_server(config: &ServerConfig) -> Connection {
    // ...
    config.url.unwrap()  // PANICS if None!
}
```

**Custom Error Types:**
```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("No backend available for tool: {0}")]
    NoBackendAvailable(String),
    
    #[error("Backend timeout after {timeout_ms}ms")]
    BackendTimeout { timeout_ms: u64 },
    
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
    
    #[error("Transport error: {0}")]
    Transport(#[from] TransportError),
}
```

**Error Context:**
```rust
use anyhow::Context;

async fn load_config(path: &str) -> anyhow::Result<Config> {
    let content = tokio::fs::read_to_string(path)
        .await
        .context(format!("Failed to read config file: {}", path))?;
    
    let config: Config = serde_yaml::from_str(&content)
        .context("Failed to parse YAML config")?;
    
    config.validate()
        .context("Configuration validation failed")?;
    
    Ok(config)
}
```

### Async Best Practices

**Use Tokio Runtime:**
```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Main function uses tokio runtime
    run_server().await
}
```

**Avoid Blocking in Async:**
```rust
// âŒ Bad (blocks async thread)
async fn process_file(path: &str) -> Result<String, Error> {
    std::fs::read_to_string(path)?  // Blocking!
}

// âœ… Good (uses async I/O)
async fn process_file(path: &str) -> Result<String, Error> {
    tokio::fs::read_to_string(path).await?
}
```

**Use Timeouts:**
```rust
use tokio::time::{timeout, Duration};

async fn call_backend(url: &str) -> Result<Response, Error> {
    timeout(
        Duration::from_secs(5),
        reqwest::get(url)
    ).await
        .map_err(|_| Error::Timeout)?
        .map_err(Error::from)
}
```

### Security Guidelines

**Input Validation:**
```rust
pub fn validate_server_id(id: &str) -> Result<(), ValidationError> {
    if id.is_empty() {
        return Err(ValidationError::EmptyId);
    }
    
    if id.len() > 255 {
        return Err(ValidationError::IdTooLong);
    }
    
    if !id.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        return Err(ValidationError::InvalidCharacters);
    }
    
    Ok(())
}
```

**Command Execution (STDIO servers):**
```rust
const ALLOWED_COMMANDS: &[&str] = &["node", "npx", "python", "python3", "uvx"];

fn validate_command(cmd: &str) -> Result<(), Error> {
    if !ALLOWED_COMMANDS.contains(&cmd) {
        return Err(Error::CommandNotAllowed(cmd.to_string()));
    }
    Ok(())
}
```

**Secrets Management:**
```rust
// âŒ Bad (hardcoded secret)
const API_KEY: &str = "sk-1234567890";

// âœ… Good (from environment)
let api_key = std::env::var("API_KEY")
    .context("API_KEY environment variable not set")?;

// Even better (use keyring for persistent storage)
use keyring::Entry;
let entry = Entry::new("only1mcp", "api_key")?;
let api_key = entry.get_password()?;
```

---

## TESTING GUIDELINES

### Test Organization

**Location:**
- Unit tests: Same file as code in `#[cfg(test)] mod tests`
- Integration tests: `tests/integration/`
- E2E tests: `tests/e2e/`

**Naming:**
- Test functions: `test_<functionality>_<condition>_<expected_result>`
- Example: `test_route_request_with_cache_hit_returns_cached_response`

### Writing Good Tests

**Test Structure (Arrange-Act-Assert):**
```rust
#[tokio::test]
async fn test_consistent_hash_distributes_evenly() {
    // Arrange: Set up test data
    let mut hash = ConsistentHash::new(200);
    hash.add_server("server1");
    hash.add_server("server2");
    hash.add_server("server3");
    
    // Act: Perform operation
    let mut counts = HashMap::new();
    for i in 0..1000 {
        let server = hash.get_server(&format!("key_{}", i)).unwrap();
        *counts.entry(server.clone()).or_insert(0) += 1;
    }
    
    // Assert: Verify results
    for (server, count) in counts {
        assert!(
            count >= 250 && count <= 400,
            "Server {} got {} requests (expected 250-400)",
            server, count
        );
    }
}
```

**Test Fixtures:**
```rust
// tests/common/fixtures.rs
pub fn sample_server_config() -> ServerConfig {
    ServerConfig {
        id: "test-server".to_string(),
        name: "Test Server".to_string(),
        transport: Transport::Http,
        url: Some("http://localhost:9000".to_string()),
        ..Default::default()
    }
}

// Use in tests
use crate::common::fixtures::*;

#[test]
fn test_server_validation() {
    let config = sample_server_config();
    assert!(config.validate().is_ok());
}
```

### Running Tests Efficiently

```bash
# Run all tests
cargo test

# Run only unit tests
cargo test --lib

# Run only integration tests
cargo test --test '*'

# Run specific test file
cargo test --test proxy_test

# Run tests matching pattern
cargo test cache

# Run with test filtering
cargo nextest run -E 'test(cache)'

# Run ignored tests (marked with #[ignore])
cargo test -- --ignored

# Show test output even on success
cargo test -- --nocapture
```

---

## DOCUMENTATION REQUIREMENTS

All public APIs must be documented:

```rust
// âŒ Bad (no documentation)
pub struct ServerInfo {
    pub id: String,
}

// âœ… Good (full documentation)
/// Information about a registered MCP backend server.
///
/// This struct contains all metadata needed to route requests,
/// perform health checks, and manage server lifecycle.
pub struct ServerInfo {
    /// Unique identifier for the server.
    ///
    /// Must be unique across all registered servers. Used for
    /// routing, health checks, and management operations.
    pub id: String,
}
```

---

## PULL REQUEST PROCESS

### PR Checklist

Before submitting:

- [ ] Tests written and passing (`cargo test`)
- [ ] Code formatted (`cargo fmt`)
- [ ] No clippy warnings (`cargo clippy`)
- [ ] Documentation updated (if API changed)
- [ ] CHANGELOG.md entry added
- [ ] Commit messages follow convention
- [ ] Branch up-to-date with main

### PR Template

```markdown
## Description

Brief description of changes and motivation.

## Related Issue

Closes #123

## Type of Change

- [ ] Bug fix (non-breaking change fixing an issue)
- [ ] New feature (non-breaking change adding functionality)
- [ ] Breaking change (fix or feature causing existing functionality to change)
- [ ] Documentation update

## Testing

- [ ] Unit tests added/updated
- [ ] Integration tests added/updated
- [ ] Manual testing performed

## Screenshots (if applicable)

## Checklist

- [ ] My code follows the style guidelines
- [ ] I have performed a self-review
- [ ] I have commented my code, particularly in hard-to-understand areas
- [ ] I have made corresponding changes to the documentation
- [ ] My changes generate no new warnings
- [ ] I have added tests that prove my fix is effective or that my feature works
- [ ] New and existing unit tests pass locally with my changes
```

---

## COMMON DEVELOPMENT TASKS

### Adding a New MCP Transport

1. Create new file: `src/transport/my_transport.rs`
2. Implement `Transport` trait
3. Add to `src/transport/mod.rs`
4. Update `ServerConfig` enum
5. Write tests in `tests/integration/my_transport_test.rs`

### Adding a New Metric

```rust
// src/metrics/prometheus.rs
lazy_static! {
    pub static ref MY_NEW_METRIC: Counter = 
        register_counter!("only1mcp_my_metric_total", "Description").unwrap();
}

// In your code
use crate::metrics::MY_NEW_METRIC;
MY_NEW_METRIC.inc();
```

### Debugging a Test Failure

```bash
# Run failing test with backtrace
RUST_BACKTRACE=1 cargo test test_name -- --nocapture

# Run with debug logging
RUST_LOG=debug cargo test test_name

# Use debugger (lldb on macOS, gdb on Linux)
rust-lldb target/debug/deps/only1mcp-<hash>
# Set breakpoint, run test
(lldb) breakpoint set --file src/proxy/router.rs --line 42
(lldb) run test_name
```

---

## TROUBLESHOOTING & FAQ

### Common Issues

**Q: Cargo build is very slow**

A: Try these optimizations:
```bash
# Use faster linker
cargo install -f cargo-binutils
rustup component add llvm-tools-preview

# Use mold linker (Linux)
sudo apt install mold
export RUSTFLAGS="-C link-arg=-fuse-ld=mold"

# Reduce parallel jobs if low memory
cargo build -j 2
```

**Q: "Too many open files" error**

A: Increase file descriptor limit:
```bash
# Temporary (current session)
ulimit -n 10000

# Permanent (add to ~/.bashrc or ~/.zshrc)
echo "ulimit -n 10000" >> ~/.bashrc
```

**Q: Tests fail with "Address already in use"**

A: Use random ports in tests:
```rust
use std::net::TcpListener;

fn get_available_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}
```

### Getting Help

- **Documentation**: https://docs.only1mcp.dev
- **GitHub Discussions**: https://github.com/doublegate/Only1MCP/discussions
- **Discord**: https://discord.gg/only1mcp
- **Stack Overflow**: Tag questions with `only1mcp`

---

## NEXT STEPS

Now that you're set up:

1. **Pick a "good first issue"**: https://github.com/doublegate/Only1MCP/issues?q=label%3A%22good+first+issue%22
2. **Read related docs**: Review documents 02-07 for deeper technical understanding
3. **Join community**: Introduce yourself in GitHub Discussions
4. **Start coding**: Create your first PR!

Welcome to the Only1MCP community! We're excited to have you contribute. 🚀

---

**Document Status:** âœ… COMPLETE  
**Next Update:** With each major release  
**Maintained By:** Only1MCP Maintainers  
**Questions:** dev@only1mcp.dev
