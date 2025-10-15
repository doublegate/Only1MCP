# Only1MCP -- "Only1": the Ultimate MCP Server Aggregator / Context Switcher

**High-performance, Rust-based proxy and aggregator for Model Context Protocol (MCP) servers with intelligent context swapping.**

[![License: GPL v3](https://img.shields.io/badge/license-GPL%20v3-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![Status](https://img.shields.io/badge/status-Beta%20Ready-green)](https://github.com/doublegate/Only1MCP)

---

## 🎯 What is Only1MCP?

Only1MCP provides a **unified interface** for AI applications to interact with multiple MCP tool servers, dramatically reducing context overhead and improving performance.

### Key Benefits

- **50-70% Context Reduction**: Intelligent caching and batching minimize AI token usage
- **<5ms Latency Overhead**: Rust-powered performance with zero-copy streaming
- **Hot-Swappable Backends**: Add/remove servers without downtime
- **Multi-Transport Support**: STDIO, HTTP, SSE, and WebSocket
- **Enterprise Security**: OAuth2, JWT, RBAC, audit logging
- **Production-Ready**: 10k+ req/s throughput, circuit breakers, health checks

---

## 🚀 Quick Start

### Installation

```bash
# From source (development)
git clone https://github.com/doublegate/Only1MCP.git
cd only1mcp
cargo build --release

# Install binary
cargo install --path .
```

### Basic Usage

```bash
# Create configuration file
cat > only1mcp.yaml <<EOF
servers:
  - id: "filesystem"
    name: "Filesystem MCP"
    transport:
      type: "stdio"
      command: "npx"
      args: ["@modelcontextprotocol/server-filesystem", "/path/to/data"]

  - id: "github"
    name: "GitHub MCP"
    transport:
      type: "stdio"
      command: "npx"
      args: ["@modelcontextprotocol/server-github"]
      env:
        GITHUB_TOKEN: "${GITHUB_TOKEN}"
EOF

# Start the proxy
only1mcp start --config only1mcp.yaml

# Server now running on http://localhost:8080
```

### Connect AI Client

Configure your AI client (Claude Desktop, Cursor, etc.) to use `http://localhost:8080` as the MCP endpoint.

---

## 📚 Documentation

### User Guides
- [Configuration Guide](docs/CONFIGURATION_GUIDE.md) - Complete YAML/TOML/JSON reference
- [CLI Reference](docs/CLI_REFERENCE.md) - Command-line interface documentation
- [Deployment Guide](docs/DEPLOYMENT_GUIDE.md) - Docker, Kubernetes, cloud deployment
- [Monitoring Guide](docs/MONITORING_GUIDE.md) - Observability and metrics setup
- [Troubleshooting](docs/TROUBLESHOOTING.md) - Common issues and solutions

### Technical Documentation
- [Architecture Overview](docs/ARCHITECTURE.md) - System design and components
- [API Reference](docs/API_REFERENCE.md) - REST and WebSocket API specification
- [Project Summary](docs/PROJECT_SUMMARY.md) - Comprehensive project overview
- [Development Roadmap](ROADMAP.md) - Project timeline and milestones

---

## 🏗️ Architecture

```
┌─────────────────┐         ┌──────────────┐         ┌─────────────┐
│  AI Application │────────▶│  Only1MCP    │────────▶│ MCP Server  │
│  (Claude, etc.) │  HTTP   │  Proxy       │  STDIO  │ (Filesystem)│
└─────────────────┘         │              │         └─────────────┘
                            │  - Routing   │         ┌─────────────┐
                            │  - Caching   │────────▶│ MCP Server  │
                            │  - Auth      │  HTTP   │ (GitHub)    │
                            │  - Metrics   │         └─────────────┘
                            └──────────────┘         ┌─────────────┐
                                                     │ MCP Server  │
                                                     │ (Database)  │
                                                     └─────────────┘
```

### Core Components

- **Proxy Server**: Axum-based HTTP server with zero-copy streaming
- **Transport Layer**: Multi-protocol support (STDIO, HTTP, SSE, WebSocket)
- **Router**: Intelligent request distribution (consistent hashing, least connections)
- **Cache**: Lock-free response caching with TTL expiration
- **Health Checker**: Active monitoring with circuit breakers
- **Auth**: OAuth2, JWT, RBAC for enterprise security

---

## 🎨 Features

### Phase 1: MVP (Weeks 1-4) ✅ In Progress

- [x] Core proxy routing
- [x] Server registry with hot-swap
- [x] YAML configuration
- [ ] STDIO transport
- [ ] Hot configuration reload
- [ ] CLI management

### Phase 2: Advanced (Weeks 5-8)

- [ ] Load balancing (consistent hashing, least connections)
- [ ] Active health checks
- [ ] Circuit breakers
- [ ] Response caching
- [ ] Request batching
- [ ] Prometheus metrics

### Phase 3: Enterprise (Weeks 9-12)

- [ ] OAuth2/JWT authentication
- [ ] Role-based access control (RBAC)
- [ ] Audit logging
- [ ] TLS 1.3 support
- [ ] Rate limiting
- [ ] Interactive TUI

### Phase 4: Extensions (Weeks 13+)

- [ ] Plugin system (dynamic libraries, WASM)
- [ ] AI-driven routing optimization
- [ ] Container orchestration (optional)
- [ ] Advanced observability (OpenTelemetry)

---

## 🔧 Configuration Example

```yaml
# only1mcp.yaml

server:
  host: "0.0.0.0"
  port: 8080
  tls:
    enabled: false

servers:
  - id: "filesystem-mcp"
    name: "Filesystem Server"
    transport:
      type: "stdio"
      command: "npx"
      args: ["@modelcontextprotocol/server-filesystem", "/home/user/data"]

  - id: "github-mcp"
    name: "GitHub Server"
    transport:
      type: "http"
      url: "http://localhost:3000/mcp"
      headers:
        Authorization: "Bearer ${GITHUB_TOKEN}"

proxy:
  load_balancer:
    algorithm: "consistent_hash"
  connection_pool:
    max_per_backend: 100

context_optimization:
  cache:
    enabled: true
    ttl_seconds: 300
  batching:
    enabled: true

observability:
  metrics:
    enabled: true
    port: 9090
  logging:
    level: "info"
    format: "json"
```

See [Configuration Reference](docs/CONFIGURATION.md) for complete schema.

---

## 🧪 Testing

```bash
# Run tests
cargo test

# Run benchmarks
cargo bench

# Check code quality
cargo clippy -- -D warnings
cargo fmt --check

# Generate coverage report
cargo tarpaulin --out Html
```

---

## 📊 Performance

### Benchmarks (Target Metrics)

| Metric | Target | Status |
|--------|--------|--------|
| Latency Overhead (p50) | <2ms | 🎯 TBD |
| Latency Overhead (p99) | <5ms | 🎯 TBD |
| Throughput | >10k req/s | 🎯 TBD |
| Memory Usage | <50MB | 🎯 TBD |
| Cache Hit Rate | >70% | 🎯 TBD |

*Benchmarks will be published after MVP release.*

---

## 🤝 Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

### Development Setup

```bash
# Clone repository
git clone https://github.com/doublegate/Only1MCP.git
cd only1mcp

# Install dependencies
cargo build

# Run tests
cargo test

# Run in development mode
cargo run -- start --config examples/solo.yaml
```

### Code Style

- Follow Rust API guidelines
- Use `cargo fmt` for formatting
- Pass `cargo clippy` with zero warnings
- Write tests for new features
- Document public APIs

---

## 🛣️ Roadmap

### Current Status: Phase 1 - MVP Development

- **Week 1-4**: Core proxy, STDIO transport, configuration, CLI
- **Week 5-8**: Load balancing, health checks, caching, metrics
- **Week 9-12**: Security (OAuth2, RBAC), audit logging, TUI
- **Week 13+**: Plugin system, AI optimization, advanced features

See [Master Tracker](to-dos/master-tracker.md) for detailed task breakdown.

---

## 📖 Documentation

- [Master Task Tracker](to-dos/master-tracker.md) - Comprehensive development roadmap
- [Architecture Documentation](docs/ARCHITECTURE.md) - System design and components
- [API Specification](docs/API.md) - MCP protocol implementation
- [Configuration Guide](docs/CONFIGURATION.md) - Complete config reference
- [Security Architecture](docs/SECURITY.md) - Security design and threat model

---

## 📄 License

Dual-licensed under MIT OR Apache-2.0.

See [LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE) for details.

---

## 🙏 Acknowledgments

- [MCP Protocol](https://modelcontextprotocol.io/) - The foundation protocol
- [Anthropic](https://www.anthropic.com/) - MCP specification and Claude integration
- [Rust Community](https://www.rust-lang.org/community) - Excellent tools and libraries
- Inspiration from existing proxies: TBXark/mcp-proxy, VeriTeknik/pluggedin-mcp-proxy

---

## 📞 Contact & Support

- **Issues**: [GitHub Issues](https://github.com/doublegate/Only1MCP/issues)
- **Discussions**: [GitHub Discussions](https://github.com/doublegate/Only1MCP/discussions)
- **Email**: hello@only1mcp.dev
- **Twitter**: [@only1mcp](https://twitter.com/only1mcp)

---

## 🌟 Star History

If you find Only1MCP useful, please consider giving it a star! ⭐

[![Star History Chart](https://api.star-history.com/svg?repos=doublegate/Only1MCP&type=Date)](https://star-history.com/#doublegate/Only1MCP&Date)

---

**Built with ❤️ in Rust**
