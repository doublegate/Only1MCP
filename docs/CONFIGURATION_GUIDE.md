# Only1MCP Configuration Guide

## 📚 Table of Contents

1. [Overview](#overview)
2. [Configuration File Formats](#configuration-file-formats)
3. [Configuration Discovery](#configuration-discovery)
4. [Core Configuration](#core-configuration)
5. [Server Configuration](#server-configuration)
6. [Transport Configuration](#transport-configuration)
7. [Routing Configuration](#routing-configuration)
8. [Caching Configuration](#caching-configuration)
9. [Security Configuration](#security-configuration)
10. [Monitoring Configuration](#monitoring-configuration)
11. [Advanced Configuration](#advanced-configuration)
12. [Environment Variables](#environment-variables)
13. [Configuration Templates](#configuration-templates)
14. [Configuration Validation](#configuration-validation)
15. [Hot-Reload Configuration](#hot-reload-configuration)

---

## Overview

Only1MCP uses a hierarchical configuration system that supports YAML, TOML, and JSON formats. Configuration can be provided via files, environment variables, or command-line arguments, with later sources overriding earlier ones.

### Configuration Priority (highest to lowest)
1. Command-line arguments
2. Environment variables
3. Configuration file specified via `--config`
4. `only1mcp.yaml` in current directory
5. `~/.only1mcp/config.yaml` in home directory
6. `/etc/only1mcp/config.yaml` system-wide
7. Built-in defaults

---

## Configuration File Formats

### YAML (Recommended)
```yaml
# only1mcp.yaml
version: "1.0"
proxy:
  host: 0.0.0.0
  port: 8080

servers:
  - id: server1
    name: "Primary MCP Server"
    transport: stdio
    command: ["mcp-server", "--port", "9001"]
```

### TOML
```toml
# only1mcp.toml
version = "1.0"

[proxy]
host = "0.0.0.0"
port = 8080

[[servers]]
id = "server1"
name = "Primary MCP Server"
transport = "stdio"
command = ["mcp-server", "--port", "9001"]
```

### JSON
```json
{
  "version": "1.0",
  "proxy": {
    "host": "0.0.0.0",
    "port": 8080
  },
  "servers": [
    {
      "id": "server1",
      "name": "Primary MCP Server",
      "transport": "stdio",
      "command": ["mcp-server", "--port", "9001"]
    }
  ]
}
```

---

## Configuration Discovery

Only1MCP searches for configuration files in the following order:

```bash
# 1. Explicit path
only1mcp start --config /custom/path/config.yaml

# 2. Current directory
./only1mcp.yaml
./only1mcp.toml
./only1mcp.json

# 3. User home directory
~/.only1mcp/config.yaml
~/.config/only1mcp/config.yaml  # XDG on Linux

# 4. System-wide
/etc/only1mcp/config.yaml
C:\ProgramData\only1mcp\config.yaml  # Windows
```

---

## Core Configuration

### Basic Settings

```yaml
# Core proxy configuration
version: "1.0"  # Configuration version

proxy:
  # Network binding
  host: 0.0.0.0           # Listen address (0.0.0.0 for all interfaces)
  port: 8080              # Listen port

  # Performance
  worker_threads: 4       # Number of worker threads (default: CPU cores)
  max_connections: 10000  # Maximum concurrent connections

  # Timeouts
  request_timeout: 30s    # Global request timeout
  idle_timeout: 60s       # Connection idle timeout
  shutdown_timeout: 30s   # Graceful shutdown timeout

  # TLS Configuration (optional)
  tls:
    enabled: true
    cert_file: /path/to/cert.pem
    key_file: /path/to/key.pem
    ca_file: /path/to/ca.pem     # For client verification
    client_auth: optional         # none, optional, required
```

### Logging Configuration

```yaml
logging:
  level: info                    # trace, debug, info, warn, error
  format: json                   # json, pretty, compact
  output: stdout                 # stdout, stderr, file
  file_path: /var/log/only1mcp/proxy.log
  max_size: 100MB               # Log rotation size
  max_age: 30d                  # Log retention
  max_backups: 10               # Number of old logs to keep

  # Per-module logging levels
  modules:
    only1mcp::proxy: debug
    only1mcp::transport: trace
    only1mcp::cache: info
```

---

## Server Configuration

### MCP Server Definitions

```yaml
servers:
  - id: primary-server          # Unique identifier
    name: "Primary MCP Server"  # Display name
    enabled: true                # Enable/disable server

    # Transport configuration
    transport: stdio             # stdio, http, sse, websocket

    # STDIO transport
    command: ["mcp-server"]      # Command to execute
    args: ["--port", "9001"]     # Command arguments
    working_dir: /opt/mcp        # Working directory
    env:                         # Environment variables
      MCP_MODE: production
      LOG_LEVEL: debug

    # Resource limits (optional)
    limits:
      memory: 512MB              # Memory limit
      cpu: 2.0                   # CPU cores limit
      timeout: 60s               # Execution timeout

    # Health checking
    health_check:
      enabled: true
      interval: 30s              # Check interval
      timeout: 5s                # Check timeout
      retries: 3                 # Retries before unhealthy

  - id: http-server
    name: "HTTP MCP Server"
    transport: http

    # HTTP transport
    endpoint: http://localhost:9002
    headers:                     # Custom headers
      Authorization: "Bearer ${API_KEY}"
      X-Client-Id: "only1mcp"

    # Connection pooling
    pool:
      min_connections: 1
      max_connections: 10
      idle_timeout: 60s
      connection_timeout: 10s
```

---

## Transport Configuration

### STDIO Transport

```yaml
transports:
  stdio:
    # Security sandboxing
    sandbox:
      enabled: true
      allowed_paths:
        - /opt/mcp/data
        - /tmp
      blocked_syscalls:
        - execve
        - fork
      resource_limits:
        nofile: 1024            # Max file descriptors
        nproc: 32               # Max processes
        memlock: 64MB           # Max locked memory
```

### HTTP Transport

```yaml
transports:
  http:
    # Global HTTP client settings
    user_agent: "Only1MCP/1.0"
    timeout: 30s
    connect_timeout: 10s

    # Keep-alive
    keep_alive: true
    keep_alive_timeout: 90s

    # Compression
    compression: true           # Enable gzip/brotli

    # Retry policy
    retry:
      max_attempts: 3
      initial_delay: 100ms
      max_delay: 10s
      multiplier: 2             # Exponential backoff
```

### WebSocket Transport

```yaml
transports:
  websocket:
    # WebSocket settings
    max_frame_size: 64KB
    max_message_size: 10MB
    compression: true

    # Ping/pong keepalive
    ping_interval: 30s
    pong_timeout: 10s

    # Reconnection
    auto_reconnect: true
    reconnect_delay: 1s
    max_reconnect_delay: 30s
```

---

## Routing Configuration

### Load Balancing

```yaml
routing:
  # Default routing strategy
  strategy: consistent_hash     # round_robin, weighted, least_connections, consistent_hash

  # Consistent hashing
  consistent_hash:
    virtual_nodes: 150          # Virtual nodes per server
    hash_function: xxh3         # xxh3, blake3, sha256

  # Weighted routing
  weights:
    primary-server: 70
    secondary-server: 30

  # Sticky sessions
  sticky_sessions:
    enabled: true
    ttl: 1h                     # Session affinity duration
    cookie_name: "X-Server-Id"

  # Failover
  failover:
    enabled: true
    max_attempts: 3
    backoff: exponential
```

### Circuit Breaker

```yaml
circuit_breaker:
  enabled: true

  # Failure thresholds
  failure_threshold: 5          # Failures to open circuit
  success_threshold: 3          # Successes to close circuit

  # Timing
  timeout: 30s                  # Time before half-open
  half_open_requests: 3         # Test requests in half-open

  # Error rate
  error_rate_threshold: 0.5     # 50% error rate
  request_volume_threshold: 20  # Minimum requests for statistics
```

---

## Caching Configuration

### Multi-tier Cache

```yaml
cache:
  enabled: true

  # L1 Cache - Tool Results
  l1:
    enabled: true
    max_size: 100MB
    ttl: 5m                     # Time to live
    eviction: lru               # lru, lfu, arc

  # L2 Cache - Resources
  l2:
    enabled: true
    max_size: 500MB
    ttl: 30m
    eviction: lfu

  # L3 Cache - Prompts
  l3:
    enabled: true
    max_size: 1GB
    ttl: 2h
    eviction: arc

  # Cache key configuration
  key:
    hash: blake3                # Hashing algorithm
    include_headers: false      # Include headers in key
    include_auth: false         # Include auth in key
```

---

## Security Configuration

### Authentication

```yaml
auth:
  # Authentication methods (multiple can be enabled)
  methods:
    - jwt
    - api_key
    - oauth2
    - mtls

  # JWT Configuration
  jwt:
    enabled: true
    algorithm: RS256
    public_key_file: /path/to/public.pem
    issuer: "https://auth.example.com"
    audience: ["only1mcp"]

  # API Key
  api_key:
    enabled: true
    header: "X-API-Key"        # Header name
    query_param: "api_key"     # Query parameter name

  # OAuth2
  oauth2:
    enabled: true
    providers:
      - id: google
        client_id: "${GOOGLE_CLIENT_ID}"
        client_secret: "${GOOGLE_CLIENT_SECRET}"
        auth_url: "https://accounts.google.com/o/oauth2/v2/auth"
        token_url: "https://oauth2.googleapis.com/token"
```

### Authorization (RBAC)

```yaml
authorization:
  enabled: true

  # Default policy
  default_policy: deny         # allow, deny

  # Roles
  roles:
    - name: admin
      permissions:
        - "*"                   # All permissions

    - name: developer
      permissions:
        - "tools:read"
        - "tools:execute"
        - "resources:read"

    - name: viewer
      permissions:
        - "tools:read"
        - "resources:read"
```

### Rate Limiting

```yaml
rate_limit:
  enabled: true

  # Global limits
  global:
    requests_per_second: 100
    burst: 200

  # Per-user limits
  per_user:
    requests_per_second: 10
    requests_per_minute: 500
    requests_per_hour: 10000

  # Per-IP limits
  per_ip:
    requests_per_second: 20
    requests_per_minute: 1000
```

---

## Monitoring Configuration

### Metrics

```yaml
metrics:
  enabled: true

  # Prometheus endpoint
  prometheus:
    enabled: true
    port: 9090
    path: /metrics

  # StatsD
  statsd:
    enabled: false
    host: localhost
    port: 8125
    prefix: "only1mcp"
```

### Tracing

```yaml
tracing:
  enabled: true

  # OpenTelemetry
  opentelemetry:
    enabled: true
    endpoint: "http://localhost:4317"
    service_name: "only1mcp"

    # Sampling
    sampling:
      strategy: adaptive        # always, never, adaptive, probabilistic
      rate: 0.1                # 10% for probabilistic
```

### Health Checks

```yaml
health:
  # Health check endpoints
  endpoints:
    liveness: /health/live
    readiness: /health/ready
    startup: /health/startup

  # Checks to perform
  checks:
    - database
    - cache
    - backends
```

---

## Advanced Configuration

### Plugin System

```yaml
plugins:
  enabled: true

  # Plugin directories
  directories:
    - /usr/local/lib/only1mcp/plugins
    - ~/.only1mcp/plugins

  # Auto-load plugins
  autoload:
    - auth-plugin
    - cache-plugin
    - logging-plugin
```

### Experimental Features

```yaml
experimental:
  # Feature flags
  features:
    ai_optimization: false
    quantum_crypto: false
    edge_computing: false

  # A/B testing
  ab_testing:
    enabled: false
    experiments:
      - name: "new_routing"
        percentage: 10         # 10% of traffic
```

---

## Environment Variables

All configuration values can be overridden using environment variables:

```bash
# Format: ONLY1MCP_<SECTION>_<KEY>
export ONLY1MCP_PROXY_PORT=8080
export ONLY1MCP_PROXY_HOST=0.0.0.0
export ONLY1MCP_LOG_LEVEL=debug

# Arrays use comma separation
export ONLY1MCP_AUTH_METHODS=jwt,api_key,oauth2

# Nested values use double underscore
export ONLY1MCP_CACHE_L1__TTL=5m
export ONLY1MCP_CACHE_L1__MAX_SIZE=100MB
```

---

## Configuration Templates

### Solo Developer

```yaml
# config/templates/solo.yaml
version: "1.0"
proxy:
  host: localhost
  port: 8080

servers:
  - id: local-mcp
    transport: stdio
    command: ["mcp-server"]

cache:
  enabled: true
  l1:
    max_size: 50MB
    ttl: 5m

logging:
  level: info
  format: pretty
```

### Small Team

```yaml
# config/templates/team.yaml
version: "1.0"
proxy:
  host: 0.0.0.0
  port: 8080

servers:
  - id: dev-server
    transport: http
    endpoint: http://mcp-dev:9001

  - id: prod-server
    transport: http
    endpoint: http://mcp-prod:9002

auth:
  methods: [api_key]

rate_limit:
  per_user:
    requests_per_minute: 100
```

### Enterprise

```yaml
# config/templates/enterprise.yaml
version: "1.0"
proxy:
  host: 0.0.0.0
  port: 443
  tls:
    enabled: true
    cert_file: /etc/ssl/certs/only1mcp.crt
    key_file: /etc/ssl/private/only1mcp.key

servers:
  - id: primary
    transport: http
    endpoint: https://mcp-primary:9001

  - id: secondary
    transport: http
    endpoint: https://mcp-secondary:9002

auth:
  methods: [jwt, mtls]

authorization:
  enabled: true

monitoring:
  metrics:
    prometheus:
      enabled: true
  tracing:
    opentelemetry:
      enabled: true
```

---

## Configuration Validation

### CLI Validation

```bash
# Validate configuration file
only1mcp validate config.yaml

# Validate with verbose output
only1mcp validate -v config.yaml

# Check specific section
only1mcp validate --section servers config.yaml
```

### Validation Rules

```yaml
# Built-in validation rules
validation:
  # Required fields
  required:
    - proxy.host
    - proxy.port
    - servers

  # Type checking
  types:
    proxy.port: integer
    proxy.worker_threads: integer
    cache.l1.ttl: duration

  # Range validation
  ranges:
    proxy.port: [1, 65535]
    proxy.worker_threads: [1, 256]

  # Pattern matching
  patterns:
    servers.*.id: "^[a-z0-9-]+$"
    auth.jwt.algorithm: "^(RS256|RS384|RS512|ES256|ES384)$"
```

---

## Hot-Reload Configuration

### Enabling Hot-Reload

```yaml
hot_reload:
  enabled: true
  watch_interval: 1s           # File system check interval
  debounce: 500ms             # Wait time after change detection

  # Reloadable sections
  reloadable:
    - servers
    - routing
    - cache
    - rate_limit

  # Non-reloadable (requires restart)
  static:
    - proxy.host
    - proxy.port
    - tls
```

### Reload Behavior

```yaml
reload_policy:
  # Validation before reload
  validate_before_reload: true

  # Rollback on error
  rollback_on_error: true
  keep_previous_versions: 3

  # Gradual rollout
  canary:
    enabled: false
    percentage: 10
    duration: 5m
```

---

## Best Practices

### 1. Use Environment Variables for Secrets
```yaml
auth:
  jwt:
    public_key: "${JWT_PUBLIC_KEY}"
  oauth2:
    client_secret: "${OAUTH_CLIENT_SECRET}"
```

### 2. Separate Environment Configs
```
config/
  ├── base.yaml          # Shared configuration
  ├── development.yaml   # Dev overrides
  ├── staging.yaml       # Staging overrides
  └── production.yaml    # Production overrides
```

### 3. Version Control
- Commit base configurations
- Don't commit secrets or environment-specific values
- Use `.gitignore` for local overrides

### 4. Configuration as Code
```yaml
# Include other files
include:
  - /etc/only1mcp/servers.yaml
  - /etc/only1mcp/security.yaml
```

### 5. Monitor Configuration Changes
```yaml
audit:
  log_config_changes: true
  notify_on_change: true
  webhook_url: "https://alerts.example.com/config-change"
```

---

## Troubleshooting

### Common Issues

1. **Configuration not loading**
   ```bash
   # Check configuration path
   only1mcp start --config-debug
   ```

2. **Validation errors**
   ```bash
   # Get detailed validation output
   only1mcp validate --explain config.yaml
   ```

3. **Hot-reload not working**
   ```bash
   # Check file system events
   only1mcp debug fs-watch config.yaml
   ```

4. **Environment variable override not working**
   ```bash
   # List all environment variables
   only1mcp config env-list
   ```

---

**Last Updated**: 2025-01-14
**Configuration Version**: 1.0
**Compatible With**: Only1MCP v0.4.0+