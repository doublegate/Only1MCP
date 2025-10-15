# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**Only1MCP** is a high-performance, Rust-based MCP (Model Context Protocol) server aggregator that provides a unified proxy interface for AI applications to interact with multiple MCP tool servers. It dramatically reduces context overhead (50-70% reduction) while improving performance (<5ms latency, 10k+ req/s throughput).

## Quick Start Commands

```bash
# Build and run
cargo build              # Debug build
cargo build --release    # Production build
cargo run -- --help      # Show CLI help

# Development workflow
cargo check             # Quick compilation check
cargo clippy           # Linting
cargo fmt              # Format code
cargo test             # Run tests
cargo doc --open       # Generate and view docs

# Running the proxy
cargo run -- start --host 0.0.0.0 --port 8080
cargo run -- validate config.yaml
cargo run -- config generate --template solo > my-config.yaml
```

## Project Structure

```
Only1MCP/
├── src/                        # Rust source code
│   ├── main.rs                # CLI entry point (clap commands)
│   ├── lib.rs                 # Library API
│   ├── error.rs               # Error types and handling
│   ├── types/                 # MCP protocol types
│   ├── config/                # Configuration system
│   │   ├── mod.rs            # Config loading and validation
│   │   ├── schema.rs         # Config schema definitions
│   │   ├── validation.rs     # Config validation logic
│   │   └── loader.rs         # Hot-reload support
│   ├── proxy/                 # Core proxy implementation
│   │   ├── server.rs         # Axum HTTP server
│   │   ├── router.rs         # Request routing engine
│   │   ├── registry.rs       # MCP server registry
│   │   └── handler.rs        # Request/response handling
│   ├── transport/             # Transport implementations
│   │   ├── stdio.rs          # STDIO process spawning
│   │   ├── http.rs           # HTTP/HTTPS client
│   │   ├── sse.rs            # Server-Sent Events
│   │   └── websocket.rs      # WebSocket support
│   ├── routing/               # Load balancing algorithms
│   │   ├── consistent_hash.rs # Consistent hashing
│   │   └── load_balancer.rs  # LB strategies
│   ├── cache/                 # Response caching
│   ├── health/                # Health checking
│   │   ├── checker.rs        # Active/passive checks
│   │   └── circuit_breaker.rs # Circuit breaker pattern
│   ├── auth/                  # Authentication/authorization
│   │   ├── jwt.rs            # JWT validation
│   │   ├── oauth.rs          # OAuth2 flows
│   │   └── rbac.rs           # Role-based access
│   └── metrics/               # Prometheus metrics
├── config/templates/          # Configuration templates
│   ├── solo.yaml             # Solo developer config
│   ├── team.yaml             # Small team config
│   └── enterprise.yaml       # Enterprise config
├── tests/                     # Integration tests
├── benches/                   # Performance benchmarks
├── docs/                      # Documentation
│   └── ARCHITECTURE.md       # System architecture
└── to-dos/
    └── master-tracker.md      # Development roadmap

```

## Key Architecture Components

### 1. Proxy Server (`src/proxy/server.rs`)
- **Framework**: Axum web framework on Tokio runtime
- **Middleware Stack**: Auth → CORS → Compression → Rate Limiting → Router
- **Key Types**: `ProxyServer`, `ServerState`
- **Entry Point**: `ProxyServer::new()` → `run()`

### 2. Transport Layer (`src/transport/`)
- **STDIO**: Process spawning with security sandboxing
- **HTTP**: Connection pooling via bb8, keep-alive optimization
- **SSE**: Long-lived connections for streaming
- **WebSocket**: Full-duplex communication

### 3. Routing Engine (`src/routing/`)
- **Consistent Hash**: Virtual nodes for even distribution
- **Load Balancing**: Round-robin, least-connections, weighted
- **Health-Aware**: Automatic failover on unhealthy backends

### 4. Context Optimization (`src/cache/`)
- **Request Batching**: Combine multiple calls in 100ms windows
- **Response Caching**: TTL-based with LRU eviction
- **Compression**: Gzip/Brotli/Zstd based on client support

## Development Phases

Currently in **Phase 1: MVP (Weeks 1-4)**

1. **Phase 1: CLI Core** (Current)
   - Basic proxy routing ✓
   - STDIO transport
   - Configuration system ✓
   - Hot-reload support

2. **Phase 2: Advanced Features** (Weeks 5-8)
   - Load balancing algorithms
   - Health checking & circuit breakers
   - Response caching
   - TUI interface

3. **Phase 3: Enterprise** (Weeks 9-12)
   - OAuth2/JWT authentication
   - RBAC authorization
   - Audit logging
   - Web dashboard

4. **Phase 4: Extensions** (Weeks 13+)
   - Plugin system
   - AI-driven optimization
   - GUI application (Tauri)

## Common Development Tasks

### Adding a New MCP Server Transport

1. Create new file in `src/transport/`
2. Implement `Transport` trait
3. Add to `TransportType` enum
4. Update router in `src/proxy/router.rs`

### Implementing a Load Balancing Algorithm

1. Add to `src/routing/load_balancer.rs`
2. Implement `LoadBalancer` trait
3. Add configuration option
4. Write tests in `tests/routing/`

### Adding Authentication Provider

1. Create in `src/auth/` (e.g., `saml.rs`)
2. Implement `AuthProvider` trait
3. Add to config schema
4. Update middleware stack

## Testing Strategy

```bash
# Unit tests
cargo test --lib

# Integration tests
cargo test --test '*'

# Specific module
cargo test proxy::

# With output
cargo test -- --nocapture

# Benchmarks
cargo bench
```

## Performance Targets

- **Latency**: <5ms proxy overhead
- **Throughput**: 10,000+ requests/second
- **Memory**: <100MB for 100 backend servers
- **Connections**: 50,000 concurrent
- **Context Reduction**: 50-70% via optimization

## Configuration

The system looks for configuration in this order:
1. CLI flag: `--config path/to/config.yaml`
2. Current directory: `only1mcp.yaml` or `only1mcp.toml`
3. Home directory: `~/.only1mcp/config.yaml`
4. System: `/etc/only1mcp/config.yaml`

### Hot-Reload
Configuration changes are automatically detected and applied without restart using the `notify` crate watching for file changes.

## Important Implementation Notes

### Error Handling
- Use `Result<T, Error>` from `src/error.rs`
- Implement proper error propagation with `?`
- Log errors with context via `tracing`

### Async/Await
- All I/O operations must be async
- Use `tokio::spawn` for background tasks
- Proper cancellation via `CancellationToken`

### Security
- STDIO transport runs in sandboxed processes
- All auth tokens are validated before routing
- TLS 1.3 minimum for production

### State Management
- Use `Arc<RwLock<T>>` for shared state
- `DashMap` for concurrent hashmaps
- `ArcSwap` for hot-reload config

## Debugging

```bash
# Enable debug logging
RUST_LOG=debug cargo run -- start

# Trace specific module
RUST_LOG=only1mcp::proxy=trace cargo run

# Full backtrace on panic
RUST_BACKTRACE=full cargo run

# Memory profiling
cargo build --release
valgrind --tool=memcheck ./target/release/only1mcp
```

## Dependencies to Note

- **tokio**: Async runtime (full features)
- **axum**: Web framework (with WebSocket support)
- **clap**: CLI argument parsing
- **serde**: Serialization for JSON/YAML/TOML
- **dashmap**: High-performance concurrent hashmap
- **bb8**: Async connection pooling
- **prometheus**: Metrics collection
- **tracing**: Structured logging

## Next Steps for Implementation

1. **Immediate** (from `to-dos/master-tracker.md`):
   - Complete `src/proxy/server.rs` - Basic Axum server
   - Implement `src/transport/http.rs` - HTTP forwarding
   - Add `src/proxy/handler.rs` - JSON-RPC handling

2. **This Week**:
   - STDIO transport for local MCP servers
   - Basic health checking
   - Integration tests

3. **Next Sprint**:
   - Load balancing algorithms
   - Response caching
   - TUI interface start

## Resources

- **Documentation**: See `ref_docs/` for complete specifications
- **Roadmap**: `to-dos/master-tracker.md` for detailed tasks
- **Architecture**: `docs/ARCHITECTURE.md` for system design
- **Examples**: `config/templates/` for configuration examples

## Contact & Support

- GitHub: https://github.com/doublegate/Only1MCP
- Issues: Report bugs and feature requests
- Discussions: Architecture and design decisions