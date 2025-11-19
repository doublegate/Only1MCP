# Getting Started with Only1MCP

This guide will help you get Only1MCP up and running in under 5 minutes.

## Quick Start

### 1. Install Only1MCP

```bash
# Clone the repository
git clone https://github.com/doublegate/Only1MCP.git
cd Only1MCP

# Build the project
cargo build --release

# The binary will be in target/release/only1mcp
```

### 2. Create Your First Configuration

Choose a template that matches your needs:

**Solo Developer:**
```bash
cp examples/configs/solo-developer.yaml only1mcp.yaml
```

**Small Team:**
```bash
cp examples/configs/small-team.yaml only1mcp.yaml
```

**Enterprise:**
```bash
cp examples/configs/enterprise.yaml only1mcp.yaml
```

### 3. Configure Your MCP Servers

Edit `only1mcp.yaml` and add your MCP servers:

```yaml
servers:
  my-mcp-server:
    name: "My MCP Server"
    transport:
      stdio:
        command: "npx"
        args: ["-y", "@modelcontextprotocol/server-filesystem", "/path/to/files"]
    enabled: true
    priority: 100
    weight: 1
```

### 4. Start the Proxy

```bash
# Start in foreground
./target/release/only1mcp start

# Or start as daemon
./target/release/only1mcp daemon start
```

### 5. Verify It's Running

```bash
# Check status
curl http://localhost:8080/health

# View metrics
curl http://localhost:9090/metrics

# Or use the TUI
./target/release/only1mcp tui
```

## Common MCP Servers

### Filesystem Server

```yaml
filesystem:
  name: "Filesystem MCP Server"
  transport:
    stdio:
      command: "npx"
      args: ["-y", "@modelcontextprotocol/server-filesystem", "/home/user/projects"]
  enabled: true
```

### GitHub Server

```yaml
github:
  name: "GitHub MCP Server"
  transport:
    stdio:
      command: "npx"
      args: ["-y", "@modelcontextprotocol/server-github"]
      env:
        GITHUB_TOKEN: "your_github_token"
  enabled: true
```

### Brave Search Server

```yaml
brave-search:
  name: "Brave Search"
  transport:
    stdio:
      command: "npx"
      args: ["-y", "@modelcontextprotocol/server-brave-search"]
      env:
        BRAVE_API_KEY: "your_brave_api_key"
  enabled: true
```

### HTTP-based MCP Server

```yaml
remote-mcp:
  name: "Remote MCP Server"
  transport:
    http:
      url: "https://mcp.example.com:3000"
      timeout_ms: 5000
  enabled: true
  health_check:
    enabled: true
    path: "/health"
    interval_secs: 30
```

## Connecting Your AI Application

Point your AI application to the Only1MCP proxy instead of individual MCP servers:

```javascript
// Before: Direct connection to MCP server
const client = new MCPClient("npx -y @modelcontextprotocol/server-filesystem /path");

// After: Connection through Only1MCP proxy
const client = new MCPClient("http://localhost:8080");
```

## Using the Dashboard

Open `examples/dashboard.html` in your browser to see a visual interface:

```bash
# Serve the dashboard with Python
cd examples
python3 -m http.server 8000

# Open http://localhost:8000/dashboard.html
```

## Monitoring and Debugging

### View Logs

```bash
# Tail logs in real-time
tail -f /var/log/only1mcp/proxy.log

# Or use the TUI for live monitoring
./target/release/only1mcp tui
```

### Prometheus Metrics

Access metrics at `http://localhost:9090/metrics`:

```
# HELP only1mcp_requests_total Total number of requests
# TYPE only1mcp_requests_total counter
only1mcp_requests_total 1234

# HELP only1mcp_request_duration_seconds Request duration
# TYPE only1mcp_request_duration_seconds histogram
only1mcp_request_duration_seconds_bucket{le="0.005"} 100
```

### Health Checks

```bash
# Overall proxy health
curl http://localhost:8080/health

# Individual server health
curl http://localhost:8080/health/servers
```

## Common Tasks

### Adding a New Server

1. Edit `only1mcp.yaml`
2. Add server configuration
3. Reload: `./target/release/only1mcp config reload`

### Enabling Features

Enable features in your config:

```yaml
# Enable AI optimization
ai_optimization:
  enabled: true
  ml_routing:
    enabled: true

# Enable multi-tenancy
multi_tenancy:
  enabled: true
  default_tier: "pro"

# Enable analytics
analytics:
  enabled: true
```

### Setting Up Authentication

1. Generate JWT secret:
```bash
openssl rand -base64 32
```

2. Configure in `only1mcp.yaml`:
```yaml
authentication:
  enabled: true
  jwt:
    secret: "your_generated_secret"
    algorithm: "HS256"
```

3. Get a token:
```bash
curl -X POST http://localhost:8080/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username": "admin", "password": "admin"}'
```

## Troubleshooting

### Proxy Won't Start

**Check port availability:**
```bash
lsof -i :8080
```

**Check configuration:**
```bash
./target/release/only1mcp validate only1mcp.yaml
```

### Servers Not Connecting

**Test server directly:**
```bash
npx -y @modelcontextprotocol/server-filesystem /path/to/test
```

**Check logs:**
```bash
./target/release/only1mcp start --log-level debug
```

### High Latency

**Enable caching:**
```yaml
cache:
  enabled: true
  ttl: 300
```

**Enable batching:**
```yaml
batching:
  enabled: true
  window_ms: 100
```

### Memory Issues

**Reduce cache size:**
```yaml
cache:
  max_size: 100  # Reduce from default
```

**Limit metrics retention:**
```yaml
analytics:
  retention_hours: 24  # Reduce from default
```

## Next Steps

- Read [INTEGRATION_GUIDE.md](INTEGRATION_GUIDE.md) for advanced integration patterns
- Explore [benchmark utilities](../benches/) for performance testing
- Check out [example plugins](../plugins/) for extending functionality
- Review [ARCHITECTURE.md](../docs/ARCHITECTURE.md) for system internals

## Getting Help

- **Documentation**: See `docs/` directory
- **Examples**: See `examples/` directory
- **Issues**: https://github.com/doublegate/Only1MCP/issues
- **Discussions**: https://github.com/doublegate/Only1MCP/discussions
