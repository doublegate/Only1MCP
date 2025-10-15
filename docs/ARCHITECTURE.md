# Only1MCP Architecture

## System Overview

Only1MCP is a high-performance proxy and aggregator for Model Context Protocol (MCP) servers, implemented in Rust for maximum performance and safety.

### Design Goals

1. **Performance**: <5ms latency overhead, 10k+ req/s throughput
2. **Reliability**: Circuit breakers, health checks, automatic failover
3. **Efficiency**: 50-70% context reduction via caching and batching
4. **Security**: OAuth2, JWT, RBAC, audit logging
5. **Scalability**: Horizontal scaling, load balancing
6. **Maintainability**: Clean architecture, comprehensive tests

---

## High-Level Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│                         AI Application                           │
│              (Claude Desktop, Cursor, Custom Clients)            │
└────────────────────────┬─────────────────────────────────────────┘
                         │ HTTP/WebSocket
                         │ (MCP JSON-RPC 2.0)
                         ▼
┌──────────────────────────────────────────────────────────────────┐
│                         Only1MCP Proxy                           │
│                                                                  │
│  ┌─────────────┐  ┌──────────┐  ┌──────┐  ┌────────┐             │
│  │   Router    │→ │  Cache   │→ │ Auth │→ │Metrics │             │
│  │ (Load Bal)  │  │ (DashMap)│  │(JWT) │  │(Prom)  │             │
│  └─────────────┘  └──────────┘  └──────┘  └────────┘             │
│         │                                                        │
│         ▼                                                        │
│  ┌─────────────────────────────────────────────────────────┐     │
│  │              Server Registry                            │     │
│  │        Arc<RwLock<HashMap<ServerId, ServerInfo>>>       │     │
│  └─────────────────────────────────────────────────────────┘     │
│         │                                                        │
└─────────┼────────────────────────────────────────────────────────┘
          │
          ├─────────────┬──────────────┬──────────────┐
          ▼             ▼              ▼              ▼
   ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐
   │  STDIO   │  │   HTTP   │  │   SSE    │  │WebSocket │
   │Transport │  │Transport │  │Transport │  │Transport │
   └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘
        │             │              │              │
        ▼             ▼              ▼              ▼
   ┌─────────────────────────────────────────────────────┐
   │               MCP Backend Servers                   │
   │  (Filesystem, GitHub, Database, Browser, etc.)      │
   └─────────────────────────────────────────────────────┘
```

---

## Core Components

### 1. Proxy Server (src/proxy/server.rs)

**Responsibilities:**
- HTTP server setup (Axum framework)
- Request/response lifecycle management
- Middleware stack configuration
- Graceful shutdown handling

**Key Technologies:**
- Axum 0.7+ for HTTP server
- Tokio for async runtime
- Tower for middleware
- Tower-HTTP for common middleware (compression, CORS, tracing)

**Implementation:**
```rust
pub struct ProxyServer {
    config: Arc<Config>,
    registry: Arc<RwLock<ServerRegistry>>,
    cache: Arc<ResponseCache>,
    pools: Arc<ConnectionPools>,
    metrics: Arc<Metrics>,
}
```

---

### 2. Request Router (src/proxy/router.rs)

**Responsibilities:**
- Routing requests to appropriate backend servers
- Load balancing algorithm selection
- Health-aware routing (skip unhealthy backends)
- Tool-to-server mapping

**Routing Algorithms:**
- **Round-Robin**: Simple, fair distribution
- **Least Connections**: Route to server with fewest active connections
- **Consistent Hashing**: Session affinity using virtual nodes
- **Weighted Random**: Distribute based on server weights

**Flow:**
1. Extract tool name from request
2. Find servers that provide the tool
3. Filter by health status
4. Apply routing algorithm
5. Return selected server

---

### 3. Server Registry (src/proxy/registry.rs)

**Responsibilities:**
- Maintain list of configured MCP servers
- Thread-safe read/write access
- Hot-swap capability (add/remove without restart)
- Server metadata storage

**Data Structure:**
```rust
pub struct ServerInfo {
    pub id: ServerId,
    pub name: String,
    pub transport: TransportConfig,
    pub status: ServerStatus, // Healthy, Degraded, Unhealthy
    pub tools: Vec<ToolName>,
    pub metrics: ServerMetrics,
}

pub type ServerRegistry = Arc<RwLock<HashMap<ServerId, ServerInfo>>>;
```

**Concurrency Strategy:**
- Read-heavy workload (99% reads, 1% writes)
- Use Arc<RwLock> for multiple readers, exclusive writer
- Atomic updates for hot-swap

---

### 4. Transport Layer (src/transport/)

#### STDIO Transport (src/transport/stdio.rs)

**Responsibilities:**
- Spawn MCP server as child process
- Communicate via stdin/stdout pipes
- Process lifecycle management
- Resource limits enforcement

**Implementation:**
```rust
pub struct StdioTransport {
    process: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    stderr: BufReader<ChildStderr>,
}
```

**Security:**
- Command allowlist (node, npx, python, uvx only)
- Resource limits (CPU, memory, file descriptors)
- User/group isolation (Linux)
- No shell injection (direct process spawn)

#### HTTP Transport (src/transport/http.rs)

**Responsibilities:**
- HTTP client with connection pooling
- Request forwarding
- Response streaming
- Retry logic

**Implementation:**
```rust
pub struct HttpTransport {
    client: reqwest::Client,
    pool: ConnectionPool,
}
```

#### WebSocket Transport (src/transport/websocket.rs)

**Responsibilities:**
- Full-duplex communication
- Message framing
- Connection management
- Ping/pong keepalive

---

### 5. Caching Layer (src/cache/mod.rs)

**Responsibilities:**
- Response caching for idempotent requests
- TTL-based expiration
- LRU eviction
- Size limit enforcement

**Data Structure:**
```rust
pub struct CacheEntry {
    response: McpResponse,
    cached_at: Instant,
    ttl: Duration,
}

pub type ResponseCache = Arc<DashMap<String, CacheEntry>>;
```

**Cache Key Generation:**
```rust
fn cache_key(request: &McpRequest) -> String {
    let mut hasher = Sha256::new();
    hasher.update(request.method.as_bytes());
    hasher.update(serde_json::to_vec(&request.params).unwrap());
    format!("{:x}", hasher.finalize())
}
```

**Benefits:**
- 70%+ cache hit rate for repeated queries
- Reduces backend load
- Improves response time (from ms to µs)
- Context window savings (cached responses smaller)

---

### 6. Health Checking (src/health/)

#### Active Health Checks (src/health/checker.rs)

**Responsibilities:**
- Periodic health pings (10s interval)
- Timeout detection (5s default)
- Status updates (Healthy/Unhealthy)
- Failure threshold tracking

**Algorithm:**
```
fall_threshold = 3  # Failures before marking unhealthy
rise_threshold = 2  # Successes before marking healthy

if consecutive_failures >= fall_threshold:
    status = Unhealthy
if consecutive_successes >= rise_threshold:
    status = Healthy
```

#### Circuit Breaker (src/health/circuit_breaker.rs)

**Responsibilities:**
- Prevent cascading failures
- Fast-fail when backend is down
- Automatic recovery testing

**States:**
- **Closed**: Normal operation, requests pass through
- **Open**: Too many failures, reject all requests
- **Half-Open**: Testing recovery, allow limited requests

**Configuration:**
```yaml
circuit_breaker:
  failure_threshold: 5      # Failures to open circuit
  success_threshold: 2      # Successes to close circuit
  timeout_seconds: 60       # Time before half-open
```

---

### 7. Authentication & Authorization (src/auth/)

#### JWT Validation (src/auth/jwt.rs)

**Responsibilities:**
- Verify JWT signatures
- Extract claims (user ID, roles)
- Token expiry checking

**Flow:**
1. Extract bearer token from Authorization header
2. Verify signature using secret/public key
3. Check expiry (exp claim)
4. Extract user info and roles
5. Attach claims to request context

#### RBAC (src/auth/rbac.rs)

**Responsibilities:**
- Role-based permission checking
- Per-tool access control
- Admin operation authorization

**Model:**
```rust
pub struct Role {
    name: String,
    permissions: Vec<Permission>,
}

pub enum Permission {
    ToolUse(ToolName),
    ToolAll,
    ServerRead,
    ServerWrite,
    ConfigRead,
    ConfigWrite,
    MetricsRead,
    AdminAll,
}
```

**Check Logic:**
```rust
fn check_permission(user_roles: &[Role], required: Permission) -> bool {
    user_roles.iter().any(|role| {
        role.permissions.iter().any(|perm| {
            perm == &required || perm == &Permission::AdminAll
        })
    })
}
```

---

### 8. Metrics & Observability (src/metrics/)

#### Prometheus Metrics

**Collected Metrics:**
- Request counters (total, per method, per backend)
- Response histograms (latency distributions)
- Cache hit/miss rates
- Active connection gauges
- Error rates
- Health check results

**Example:**
```rust
lazy_static! {
    static ref REQUEST_COUNT: Counter =
        register_counter!("mcp_requests_total", "Total requests").unwrap();

    static ref REQUEST_DURATION: Histogram =
        register_histogram!("mcp_request_duration_seconds", "Request duration").unwrap();

    static ref CACHE_HITS: Counter =
        register_counter!("mcp_cache_hits_total", "Cache hits").unwrap();
}
```

#### Tracing

**OpenTelemetry Integration:**
- Distributed tracing across components
- Span instrumentation for all operations
- Export to Jaeger/Zipkin
- Sampling configuration (0.1% in prod, 100% in dev)

---

## Data Flow

### Request Flow (Normal)

```
1. Client sends MCP request to Only1MCP
   ↓
2. Proxy receives, parses JSON-RPC
   ↓
3. Authentication middleware validates token
   ↓
4. Router selects backend server (load balancing)
   ↓
5. Check cache for matching response
   ↓ (if miss)
6. Forward request to backend via transport
   ↓
7. Backend processes and returns response
   ↓
8. Cache response (if cacheable)
   ↓
9. Return response to client
   ↓
10. Update metrics (latency, success/failure)
```

### Request Flow (With Failover)

```
1-4. Same as normal flow
   ↓
5. Primary backend is unhealthy (circuit open)
   ↓
6. Router selects secondary backend
   ↓
7. Forward request to secondary
   ↓
8-10. Same as normal flow
```

### Hot-Reload Flow

```
1. User modifies config file
   ↓
2. File watcher detects change
   ↓
3. Parse and validate new config
   ↓ (if valid)
4. Acquire write lock on registry
   ↓
5. Atomically swap server list
   ↓
6. Release write lock
   ↓
7. Send tools/listChanged notification to clients
   ↓
8. Clients request updated tool list
```

---

## Performance Optimizations

### 1. Zero-Copy Streaming
- Use `Bytes` crate for reference-counted buffers
- Avoid unnecessary data copying
- Stream responses directly from backend to client

### 2. Connection Pooling
- Maintain pool of HTTP connections per backend
- Reuse connections for multiple requests
- Configurable limits (max 100 per backend)
- Idle timeout (close unused connections after 5 min)

### 3. Async Everything
- Tokio async runtime for all I/O
- Non-blocking operations throughout
- Parallel request processing

### 4. Lock-Free Data Structures
- DashMap for concurrent cache access
- Arc<RwLock> for registry (read-heavy)
- Atomic operations where possible

### 5. Request Batching (Optional)
- Combine multiple tool calls into single batch
- Reduces round-trips
- Configurable window (100ms default)

---

## Security Architecture

### Defense in Depth

**Layer 1: Network**
- TLS 1.3 encryption
- Certificate validation
- mTLS for client authentication (optional)

**Layer 2: Authentication**
- JWT token validation
- OAuth2 authorization flow
- API key authentication (fallback)

**Layer 3: Authorization**
- RBAC per-tool access control
- Admin operation restrictions
- Rate limiting

**Layer 4: Input Validation**
- JSON schema validation
- Command allowlist (STDIO)
- Path traversal prevention
- SQL injection prevention

**Layer 5: Process Isolation**
- Sandbox MCP processes (Linux namespaces)
- Resource limits (cgroups)
- User/group isolation

**Layer 6: Audit**
- Comprehensive audit logging
- All tool invocations logged
- Admin operation tracking
- Immutable log storage

---

## Deployment Architecture

### Single Instance (Development/Solo)

```
┌────────────┐
│  Only1MCP  │ :8080
│  (Single)  │
└──────┬─────┘
       │
       ├─→ MCP Server 1 (STDIO)
       ├─→ MCP Server 2 (STDIO)
       └─→ MCP Server 3 (HTTP)
```

### High Availability (Production)

```
       ┌──────────────┐
       │ Load Balancer│
       │  (HAProxy)   │
       └──────┬───────┘
              │
    ┌─────────┼─────────┐
    ▼         ▼         ▼
┌────────┐┌────────┐┌────────┐
│Only1MCP││Only1MCP││Only1MCP│
│Instance││Instance││Instance│
│   #1   ││   #2   ││   #3   │
└────┬───┘└────┬───┘└────┬───┘
     │         │         │
     └─────────┴─────────┘
              │
         ┌────┴────┐
         ▼         ▼
    ┌────────┐┌────────┐
    │ Redis  ││Backend │
    │ Cache  ││Servers │
    │(Shared)││(MCP)   │
    └────────┘└────────┘
```

**Features:**
- Multiple Only1MCP instances for redundancy
- Shared Redis cache for consistency
- Load balancer for request distribution
- No single point of failure

---

## Technology Stack

### Core
- **Language**: Rust (1.70+)
- **Async Runtime**: Tokio
- **HTTP Server**: Axum
- **HTTP Client**: Reqwest

### Data Structures
- **Concurrent HashMap**: DashMap
- **Lock-Free Updates**: arc-swap
- **Efficient Locks**: parking_lot

### Serialization
- **JSON**: serde_json
- **YAML**: serde_yaml
- **TOML**: toml

### Observability
- **Metrics**: Prometheus
- **Tracing**: OpenTelemetry
- **Logging**: tracing + tracing-subscriber

### Security
- **TLS**: Rustls
- **JWT**: jsonwebtoken
- **Hashing**: sha2, xxhash-rust

### Testing
- **Unit**: Built-in test framework
- **Integration**: tokio-test
- **Benchmarking**: criterion
- **Property**: proptest
- **Mocking**: wiremock

---

## Code Organization

```
src/
├── main.rs              # CLI entry point
├── lib.rs               # Library API
├── error.rs             # Error types
├── types.rs             # Common types (McpRequest, McpResponse)
├── config/              # Configuration
│   ├── mod.rs           # Config struct and loading
│   ├── schema.rs        # YAML/TOML schemas
│   ├── validation.rs    # Config validation
│   └── loader.rs        # File discovery
├── proxy/               # Core proxy logic
│   ├── mod.rs
│   ├── server.rs        # Axum server setup
│   ├── router.rs        # Request routing
│   ├── registry.rs      # Server registry
│   └── handler.rs       # Request handlers
├── transport/           # Transport implementations
│   ├── mod.rs
│   ├── stdio.rs         # STDIO transport
│   ├── http.rs          # HTTP transport
│   ├── sse.rs           # SSE transport
│   └── websocket.rs     # WebSocket transport
├── routing/             # Routing algorithms
│   ├── mod.rs
│   ├── consistent_hash.rs
│   └── load_balancer.rs
├── cache/               # Response caching
│   └── mod.rs
├── health/              # Health checking
│   ├── mod.rs
│   ├── checker.rs
│   └── circuit_breaker.rs
├── auth/                # Authentication
│   ├── mod.rs
│   ├── jwt.rs
│   ├── oauth.rs
│   └── rbac.rs
└── metrics/             # Metrics collection
    └── mod.rs
```

---

## Extension Points

### 1. Plugin System (Future)

**Goals:**
- Allow custom routing algorithms
- Support custom authentication providers
- Enable protocol extensions

**Approaches:**
- Dynamic library loading (dlopen)
- WebAssembly modules (wasmtime)
- Trait-based plugin API

### 2. AI-Driven Optimization (Future)

**Goals:**
- Learn optimal routing from historical data
- Predict server performance
- Auto-tune cache TTL values

**Implementation:**
- Collect telemetry data
- Train lightweight ML model (offline)
- Integrate inference in routing decisions

---

## Performance Targets

| Metric | Target | Measurement |
|--------|--------|-------------|
| Latency Overhead (p50) | <2ms | Criterion benchmarks |
| Latency Overhead (p99) | <5ms | Criterion benchmarks |
| Throughput | >10k req/s | wrk load test |
| Memory Usage | <50MB | Valgrind massif |
| Cache Hit Rate | >70% | Prometheus metrics |
| Availability | 99.9% | Uptime monitoring |

---

## Future Enhancements

### Short-Term (Phase 2-3)
- Request batching for efficiency
- Advanced caching strategies (Bloom filter, probabilistic)
- Streaming response support (chunked transfer)
- Multiple load balancing algorithms
- Rate limiting per client/tool

### Medium-Term (Phase 4)
- Plugin system for extensibility
- AI-driven routing optimization
- Multi-region deployment support
- Container orchestration integration

### Long-Term (Post V1.0)
- Service mesh integration
- Advanced security (PII detection, DLP)
- Compliance certifications (SOC2, HIPAA)
- GraphQL support
- gRPC support

---

**Last Updated:** October 14, 2025
**Maintained By:** Only1MCP Development Team
**Next Review:** After MVP release
