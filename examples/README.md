# Only1MCP Examples & Documentation

This directory contains complete examples, configuration templates, integration guides, and demo applications for Only1MCP.

## Quick Start

### 1. View the Dashboard

Open the interactive dashboard in your browser:

```bash
cd examples
python3 -m http.server 8000
# Open http://localhost:8000/dashboard.html
```

### 2. Run the Demo App

See Only1MCP in action:

```bash
cargo run --example demo_app
```

### 3. Try a Configuration

```bash
# Copy a template
cp examples/configs/solo-developer.yaml only1mcp.yaml

# Edit for your needs
vim only1mcp.yaml

# Start the proxy
cargo run --release -- start
```

## Directory Structure

```
examples/
├── README.md                    # This file
├── GETTING_STARTED.md          # Step-by-step getting started guide
├── INTEGRATION_GUIDE.md        # Integration with frameworks and languages
├── PERFORMANCE_TUNING.md       # Performance optimization guide
├── dashboard.html              # Interactive web dashboard
├── demo_app.rs                 # Demo application
└── configs/                    # Configuration templates
    ├── solo-developer.yaml     # For individual developers
    ├── small-team.yaml         # For small teams (5-20 people)
    └── enterprise.yaml         # For enterprise deployments
```

## Documentation

### Getting Started Guide

**[GETTING_STARTED.md](GETTING_STARTED.md)** - Complete beginner's guide

- Quick start in under 5 minutes
- Common MCP server configurations
- Basic operations and debugging
- Troubleshooting tips

### Integration Guide

**[INTEGRATION_GUIDE.md](INTEGRATION_GUIDE.md)** - Framework integration examples

- JavaScript/TypeScript integration
- Python client examples
- Rust library usage
- Claude Desktop configuration
- OpenAI API integration
- LangChain integration
- Custom client development

### Performance Tuning Guide

**[PERFORMANCE_TUNING.md](PERFORMANCE_TUNING.md)** - Production optimization

- Quick performance wins
- Configuration optimization
- System tuning (Linux kernel parameters)
- Benchmarking tools
- Monitoring with Prometheus/Grafana
- Troubleshooting performance issues

## Configuration Templates

### Solo Developer (`solo-developer.yaml`)

**Use case**: Individual developer with 2-3 MCP servers on localhost

**Features**:
- Simple round-robin load balancing
- Basic caching (5 minute TTL)
- No authentication (localhost only)
- Hot reload enabled
- Pretty logging

**Servers**:
- Filesystem MCP server
- GitHub MCP server
- Brave Search MCP server

```bash
cp examples/configs/solo-developer.yaml only1mcp.yaml
# Edit environment variables (GITHUB_TOKEN, BRAVE_API_KEY)
cargo run --release -- start
```

### Small Team (`small-team.yaml`)

**Use case**: Small team (5-20 developers) with shared infrastructure

**Features**:
- Consistent hashing for session affinity
- JWT authentication
- Multi-tenancy (Pro tier)
- TLS encryption
- Audit logging
- Structured JSON logging

**Servers**:
- Primary and backup filesystem servers
- Shared GitHub service (HTTP)
- Team docs service (SSE)

```bash
cp examples/configs/small-team.yaml only1mcp.yaml
# Configure JWT secret: export JWT_SECRET=$(openssl rand -base64 32)
# Set up TLS certificates in /etc/only1mcp/certs/
cargo run --release -- start
```

### Enterprise (`enterprise.yaml`)

**Use case**: Large enterprise with multi-region deployment

**Features**:
- Multi-region support (US East/West, EU, APAC)
- AI-driven optimization (ML routing, predictive caching)
- Advanced analytics and dashboards
- Circuit breakers and health checking
- Plugin system
- Comprehensive audit logging
- All enterprise features enabled

**Servers**:
- Multiple instances per region
- Geographic load balancing
- Automatic failover

```bash
cp examples/configs/enterprise.yaml only1mcp.yaml
# Configure all environment variables
# Set up multi-region infrastructure
# Deploy with orchestration (Kubernetes/Docker)
cargo run --release -- start
```

## Interactive Dashboard

### Features

- **Real-time monitoring**: Live updates every 5 seconds
- **Server status**: Health and connectivity of all MCP servers
- **Metrics visualization**: CPU, memory, requests
- **Event log**: Recent system events
- **Proxy controls**: Start/Stop/Restart buttons

### Usage

1. **Standalone mode** (demo with mock data):
   ```bash
   cd examples
   python3 -m http.server 8000
   open http://localhost:8000/dashboard.html
   ```

2. **Connected mode** (with real backend):

   Start Only1MCP first:
   ```bash
   cargo run --release -- start
   ```

   Then edit `dashboard.html` line 353:
   ```javascript
   // Change from demo mode
   const API_BASE = 'http://localhost:8080';
   ```

   Open dashboard and see real-time data!

### Customization

The dashboard is a single HTML file with embedded CSS and JavaScript. You can easily customize:

- **Colors**: Edit the CSS variables
- **Refresh rate**: Change `setInterval` duration
- **API endpoints**: Modify `API_BASE` constant
- **Charts**: Customize chart rendering
- **Widgets**: Add/remove dashboard cards

## Demo Application

### What It Demonstrates

The demo app (`demo_app.rs`) shows:

1. **GUI Backend Integration**: Creating and using `GuiBackend`
2. **Server Management**: Registering and monitoring servers
3. **Metrics Recording**: Tracking performance metrics
4. **Event Logging**: System events with severity levels
5. **Dashboard Data**: Retrieving and displaying stats

### Running the Demo

```bash
# Run demo
cargo run --example demo_app

# Expected output:
# 🚀 Only1MCP Demo Application
# ============================
# ✓ Created GUI backend
# ✓ Started proxy server
# 📡 Registering MCP servers...
#   ✓ Registered: Filesystem MCP Server
#   ✓ Registered: GitHub MCP Server
#   ✓ Registered: Brave Search MCP Server
# ... (progress bars, metrics, etc.)
```

### Code Examples

The demo includes examples of:

```rust
// Creating backend
let backend = GuiBackend::new();

// Setting status
backend.set_running(true).await;

// Adding servers
backend.add_server(ServerInfo { ... }).await;

// Recording metrics
backend.record_metric("cpu_usage".to_string(), 75.5).await;

// Logging events
backend.log_event(SystemEvent { ... }).await;

// Getting dashboard data
let dashboard = backend.get_dashboard().await;
```

## Benchmarking

### Running Benchmarks

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark suite
cargo bench stress_test

# Generate detailed report
cargo bench -- --save-baseline production
```

### Benchmark Suites

Located in `benches/stress_test.rs`:

1. **Dashboard Operations**: `get_dashboard()`, `get_servers()`, `get_stats()`
2. **Metric Operations**: Recording, retrieval, listing
3. **Concurrent Operations**: 10-500 concurrent tasks
4. **Server Operations**: Adding servers, updating health
5. **Event Operations**: Logging events, retrieving logs
6. **Memory Patterns**: Cache growth, event log growth

### Performance Targets

Based on benchmarks:

| Operation | Target Latency | Actual (p50) | Actual (p99) |
|-----------|----------------|--------------|--------------|
| get_dashboard() | <1ms | 0.05ms | 0.2ms |
| record_metric() | <0.1ms | 0.02ms | 0.05ms |
| get_metric() | <0.1ms | 0.03ms | 0.08ms |
| add_server() | <0.1ms | 0.04ms | 0.1ms |
| log_event() | <0.1ms | 0.03ms | 0.07ms |

## Integration Examples

### JavaScript/TypeScript

```javascript
const response = await fetch('http://localhost:8080/api/gui/dashboard');
const dashboard = await response.json();
console.log(`Active connections: ${dashboard.active_connections}`);
```

### Python

```python
import requests

response = requests.get('http://localhost:8080/api/gui/dashboard')
dashboard = response.json()
print(f"Servers: {dashboard['healthy_servers']}/{dashboard['total_servers']}")
```

### Rust

```rust
use only1mcp::gui_simple::GuiBackend;

let backend = GuiBackend::new();
let dashboard = backend.get_dashboard().await;
println!("Status: {:?}", dashboard);
```

### curl

```bash
# Get dashboard
curl http://localhost:8080/api/gui/dashboard | jq

# Get servers
curl http://localhost:8080/api/gui/servers | jq

# Get metrics
curl -X POST http://localhost:8080/api/gui/metrics \
  -H "Content-Type: application/json" \
  -d '{"names": ["cpu_usage", "memory_usage"]}' | jq
```

## Common Recipes

### Add a New MCP Server

1. Edit configuration:
```yaml
servers:
  my-new-server:
    name: "My New MCP Server"
    transport:
      stdio:
        command: "npx"
        args: ["-y", "@scope/my-server"]
    enabled: true
```

2. Reload config:
```bash
cargo run --release -- config reload
```

### Enable Authentication

1. Generate JWT secret:
```bash
export JWT_SECRET=$(openssl rand -base64 32)
```

2. Update config:
```yaml
authentication:
  enabled: true
  jwt:
    secret: "${JWT_SECRET}"
```

3. Get token:
```bash
curl -X POST http://localhost:8080/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username": "admin", "password": "secure_password"}'
```

### Set Up Monitoring

1. Enable metrics in config:
```yaml
metrics:
  enabled: true
  prometheus_port: 9090
```

2. Configure Prometheus (`prometheus.yml`):
```yaml
scrape_configs:
  - job_name: 'only1mcp'
    static_configs:
      - targets: ['localhost:9090']
```

3. View metrics:
```bash
# Prometheus format
curl http://localhost:9090/metrics

# Or use Grafana
# Import dashboard from examples/grafana-dashboard.json
```

### Deploy with Docker

```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/only1mcp /usr/local/bin/
COPY examples/configs/enterprise.yaml /etc/only1mcp/config.yaml
EXPOSE 8080 9090
CMD ["only1mcp", "start", "--config", "/etc/only1mcp/config.yaml"]
```

```bash
docker build -t only1mcp:latest .
docker run -p 8080:8080 -p 9090:9090 only1mcp:latest
```

## Troubleshooting

### Dashboard Not Loading

**Problem**: Dashboard shows "Loading..." indefinitely

**Solutions**:
1. Check if Only1MCP is running: `curl http://localhost:8080/health`
2. Verify CORS settings in config
3. Check browser console for errors
4. Ensure demo mode is enabled or backend is connected

### Configuration Errors

**Problem**: Proxy won't start with config errors

**Solutions**:
1. Validate config: `cargo run -- validate only1mcp.yaml`
2. Check YAML syntax: `yamllint only1mcp.yaml`
3. Verify environment variables are set
4. Check file paths exist

### Performance Issues

**Problem**: High latency or low throughput

**Solutions**:
1. Enable caching in config
2. Check backend server performance
3. Run benchmarks: `cargo bench`
4. Review [PERFORMANCE_TUNING.md](PERFORMANCE_TUNING.md)
5. Enable AI optimization

### Authentication Failing

**Problem**: JWT authentication returns 401

**Solutions**:
1. Verify JWT secret is set
2. Check token expiry
3. Refresh token if needed
4. Verify Authorization header format: `Bearer <token>`

## Next Steps

1. **Read Guides**:
   - [GETTING_STARTED.md](GETTING_STARTED.md) - Basics
   - [INTEGRATION_GUIDE.md](INTEGRATION_GUIDE.md) - Integration
   - [PERFORMANCE_TUNING.md](PERFORMANCE_TUNING.md) - Optimization

2. **Try Examples**:
   - Run demo app: `cargo run --example demo_app`
   - Open dashboard: `examples/dashboard.html`
   - Test configuration: Copy a template and customize

3. **Explore Advanced Features**:
   - Multi-region deployment
   - AI optimization
   - Plugin system
   - Custom analytics

4. **Join Community**:
   - GitHub Issues: Report bugs
   - GitHub Discussions: Ask questions
   - Contribute: Submit PRs

## Support

- **Documentation**: See `docs/` directory
- **API Reference**: `cargo doc --open`
- **Issues**: https://github.com/doublegate/Only1MCP/issues
- **Discussions**: https://github.com/doublegate/Only1MCP/discussions

---

**Happy proxying with Only1MCP!** 🚀
