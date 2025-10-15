# Only1MCP Configuration Schema & Examples
## Comprehensive YAML/TOML Configuration Reference & Common Use Case Templates

**Document Version:** 1.0  
**Configuration Formats:** YAML (primary), TOML (alternative)  
**Schema Version:** 1.0.0  
**Date:** October 14, 2025  
**Status:** Configuration Specification

---

## TABLE OF CONTENTS

1. [Configuration Overview](#configuration-overview)
2. [Configuration Loading Order](#configuration-loading-order)
3. [YAML Schema Reference](#yaml-schema-reference)
4. [TOML Schema Reference](#toml-schema-reference)
5. [Environment Variables](#environment-variables)
6. [Solo Developer Configuration](#solo-developer-configuration)
7. [Small Team Configuration](#small-team-configuration)
8. [Enterprise Configuration](#enterprise-configuration)
9. [Specialized Configurations](#specialized-configurations)
10. [Configuration Validation](#configuration-validation)
11. [Migration Guides](#migration-guides)
12. [Advanced Patterns](#advanced-patterns)

---

## CONFIGURATION OVERVIEW

### Key Features

Only1MCP's configuration system provides:
- **Hot-reload capability**: Change configuration without downtime
- **Multiple format support**: YAML (recommended) or TOML
- **Environment variable overrides**: For secrets and deployment-specific values
- **Schema validation**: Automatic validation with helpful error messages
- **Template system**: Pre-built configurations for common scenarios
- **Visual configuration**: Web UI alternative to manual editing

### Configuration Philosophy

1. **Convention over Configuration**: Sensible defaults that work out-of-box
2. **Progressive Disclosure**: Simple configs stay simple, complexity is optional
3. **Security by Default**: TLS enabled, auth configured, minimal exposure
4. **Performance Optimized**: Caching enabled, connection pooling configured
5. **Zero Surprises**: Clear naming, obvious behavior, comprehensive docs

---

## CONFIGURATION LOADING ORDER

Only1MCP loads configuration from multiple sources in priority order:

```yaml
# Priority order (highest to lowest):
1. Command-line flags          # --port 9000
2. Environment variables        # ONLY1MCP_PORT=9000
3. User config file            # ~/.only1mcp/config.yaml
4. Project config file         # ./only1mcp.yaml
5. System config file          # /etc/only1mcp/config.yaml
6. Built-in defaults           # Embedded in binary
```

### File Discovery Algorithm

```rust
// Pseudocode for config file discovery
fn find_config_file() -> Option<PathBuf> {
    // 1. Check --config flag
    if let Some(path) = args.config_path {
        return Some(path);
    }
    
    // 2. Check ONLY1MCP_CONFIG env var
    if let Ok(path) = env::var("ONLY1MCP_CONFIG") {
        return Some(PathBuf::from(path));
    }
    
    // 3. Check working directory
    if Path::new("only1mcp.yaml").exists() {
        return Some(PathBuf::from("only1mcp.yaml"));
    }
    
    // 4. Check user home directory
    if let Some(home) = dirs::home_dir() {
        let user_config = home.join(".only1mcp/config.yaml");
        if user_config.exists() {
            return Some(user_config);
        }
    }
    
    // 5. Check system directory
    if Path::new("/etc/only1mcp/config.yaml").exists() {
        return Some(PathBuf::from("/etc/only1mcp/config.yaml"));
    }
    
    None // Use built-in defaults
}
```

---

## YAML SCHEMA REFERENCE

### Complete YAML Schema with All Options

```yaml
# only1mcp.yaml - Complete configuration schema
# Version: 1.0.0
# All fields are optional - defaults shown

# Server configuration - how Only1MCP itself runs
server:
  # Network binding
  host: "0.0.0.0"              # Interface to bind (0.0.0.0 = all interfaces)
  port: 8080                   # Port to listen on
  
  # Performance tuning
  worker_threads: 0            # 0 = auto-detect CPU cores
  max_connections: 10000       # Maximum concurrent connections
  request_timeout_ms: 30000    # Global request timeout (30s)
  
  # TLS configuration
  tls:
    enabled: false             # Enable HTTPS
    cert_path: null            # Path to certificate file
    key_path: null             # Path to private key file
    ca_path: null              # Path to CA bundle (for mTLS)
    min_version: "1.3"         # Minimum TLS version (1.2 or 1.3)
    
  # Admin interface
  admin:
    enabled: true              # Enable web admin UI
    port: 8081                 # Admin UI port (null = same as main port)
    path: "/admin"             # URL path for admin UI
    auth_required: true        # Require authentication for admin

# MCP backend servers configuration
servers:
  # Each server has a unique identifier
  - id: "filesystem-server"
    name: "Filesystem MCP"    # Human-readable name
    enabled: true              # Enable/disable without removing
    
    # Transport configuration (REQUIRED)
    transport:
      type: "stdio"            # Options: stdio, http, sse (deprecated)
      
      # For STDIO transport
      command: "npx"           # Command to execute
      args:                    # Command arguments
        - "@modelcontextprotocol/server-filesystem"
        - "/home/user/data"
      env:                     # Environment variables
        NODE_ENV: "production"
      working_dir: null        # Working directory (null = current)
      
    # Health check configuration
    health_check:
      enabled: true            # Enable health monitoring
      interval_seconds: 10     # Check interval
      timeout_seconds: 5       # Health check timeout
      failure_threshold: 3     # Failures before marking unhealthy
      success_threshold: 2     # Successes before marking healthy
      
    # Request routing rules
    routing:
      # Tool-based routing
      tools:                   # Tools this server provides
        - "read_file"
        - "write_file"
        - "list_directory"
      tools_regex: null        # Regex pattern for tool matching
      
      # Priority and weight
      priority: 100            # Higher = preferred (default: 100)
      weight: 1                # Load balancing weight
      
    # Server-specific timeout
    timeout_ms: 5000          # Override global timeout for this server
    
    # Retry configuration
    retry:
      max_attempts: 3          # Maximum retry attempts
      initial_delay_ms: 100    # Initial retry delay
      max_delay_ms: 5000       # Maximum retry delay
      multiplier: 2            # Exponential backoff multiplier
      
  # HTTP transport example
  - id: "github-server"
    name: "GitHub MCP"
    transport:
      type: "http"
      url: "http://localhost:3000/mcp"
      headers:                 # HTTP headers
        Authorization: "Bearer ${GITHUB_TOKEN}"
      
  # Multiple servers can provide same tools (load balanced)
  - id: "filesystem-backup"
    name: "Filesystem Backup"
    transport:
      type: "stdio"
      command: "npx"
      args: ["@modelcontextprotocol/server-filesystem", "/backup"]
    routing:
      tools: ["read_file", "write_file"]
      priority: 50             # Lower priority = backup server

# Proxy behavior configuration
proxy:
  # Load balancing
  load_balancer:
    algorithm: "round_robin"   # Options: round_robin, least_connections, 
                              #          random, weighted, consistent_hash
    
    # For consistent_hash algorithm
    hash_key: "tool_name"      # What to hash on: tool_name, client_id
    virtual_nodes: 150         # Virtual nodes for consistent hash ring
    
  # Connection pooling
  connection_pool:
    max_per_backend: 100       # Max connections per backend
    min_idle: 10               # Minimum idle connections to maintain
    max_idle_time_ms: 300000   # Close idle connections after 5 min
    
  # Request handling
  request:
    max_body_size_mb: 100      # Maximum request body size
    compression: true          # Enable gzip compression
    
  # Response handling
  response:
    buffer_size_kb: 64         # Response buffer size
    streaming: true            # Enable response streaming
    
  # Hot-reload configuration
  hot_reload:
    enabled: true              # Enable configuration hot-reload
    watch_interval_ms: 1000    # File system watch interval
    debounce_ms: 500          # Wait before applying changes
    
# Context optimization features
context_optimization:
  # Response caching
  cache:
    enabled: true              # Enable response caching
    max_entries: 10000         # Maximum cache entries
    max_size_mb: 500          # Maximum cache size in MB
    ttl_seconds: 300          # Default cache TTL (5 minutes)
    
    # Per-tool cache configuration
    tool_overrides:
      read_file:
        ttl_seconds: 60        # Short TTL for file reads
      list_directory:
        ttl_seconds: 600       # Longer TTL for directory listings
        
  # Request batching
  batching:
    enabled: true              # Enable request batching
    max_batch_size: 50         # Maximum requests per batch
    batch_window_ms: 100       # Time window to collect batch
    
  # Lazy loading
  lazy_loading:
    enabled: true              # Load tool schemas on-demand
    preload_tools: []          # Tools to preload at startup
    
  # Compression
  compression:
    enabled: true              # Compress stored responses
    algorithm: "zstd"          # Options: gzip, zstd, lz4
    level: 3                   # Compression level (1-9)
    
  # Token estimation
  token_estimation:
    enabled: true              # Show token usage estimates
    model: "claude-3"          # Model for estimation

# Authentication configuration
auth:
  # Admin API authentication
  admin:
    enabled: true
    type: "api_key"            # Options: api_key, jwt, oauth2
    
    # For api_key auth
    api_key_env: "ONLY1MCP_ADMIN_KEY"  # Environment variable
    api_key_header: "X-API-Key"        # HTTP header name
    
    # For JWT auth
    jwt:
      secret_env: "ONLY1MCP_JWT_SECRET"
      algorithm: "HS256"       # Options: HS256, RS256
      expiry_hours: 24
      
    # For OAuth2 auth
    oauth2:
      provider: "google"       # Options: google, github, okta, custom
      client_id_env: "OAUTH_CLIENT_ID"
      client_secret_env: "OAUTH_CLIENT_SECRET"
      redirect_url: "http://localhost:8080/auth/callback"
      scopes: ["email", "profile"]
      
  # Client authentication (for MCP requests)
  client:
    enabled: false             # Most MCP clients don't support auth
    type: "bearer"             # Options: bearer, api_key, mtls
    
  # Rate limiting
  rate_limit:
    enabled: true
    type: "sliding_window"     # Options: fixed_window, sliding_window
    
    # Global limits
    requests_per_minute: 1000
    requests_per_hour: 10000
    
    # Per-client limits
    per_client:
      requests_per_minute: 60
      burst: 10                # Allow burst above limit
      
    # Exempt certain clients
    exemptions:
      - "127.0.0.1"           # Localhost
      - "10.0.0.0/8"          # Private network
      
# RBAC - Role-Based Access Control
rbac:
  enabled: false              # Enable RBAC
  
  # Define roles
  roles:
    - name: "admin"
      permissions:
        - "servers:*"         # All server operations
        - "config:*"          # Configuration changes
        - "metrics:read"      # View metrics
        
    - name: "developer"
      permissions:
        - "servers:read"      # View servers
        - "tools:*"           # Use all tools
        - "metrics:read"      # View metrics
        
    - name: "readonly"
      permissions:
        - "servers:read"      # View only
        - "tools:list"        # List tools only
        
  # User role assignments
  users:
    - id: "user@example.com"
      roles: ["admin"]
    - id: "dev@example.com"
      roles: ["developer"]
      
  # Default role for unauthenticated users
  default_role: "readonly"

# Observability configuration
observability:
  # Metrics
  metrics:
    enabled: true
    type: "prometheus"         # Options: prometheus, statsd
    port: 9090                # Metrics endpoint port
    path: "/metrics"          # Metrics endpoint path
    
    # Detailed metrics
    detailed:
      per_tool: true          # Metrics per tool
      per_backend: true       # Metrics per backend
      histograms: true        # Request duration histograms
      
  # Distributed tracing
  tracing:
    enabled: false            # Enable OpenTelemetry tracing
    type: "jaeger"           # Options: jaeger, zipkin, otlp
    
    # Jaeger configuration
    jaeger:
      agent_endpoint: "localhost:6831"
      sampling_rate: 0.01     # Sample 1% of requests
      
  # Logging
  logging:
    level: "info"             # Options: trace, debug, info, warn, error
    format: "json"            # Options: json, pretty, compact
    
    # Output destinations
    outputs:
      - type: "stdout"        # Console output
        level: "info"
        
      - type: "file"          # File output
        path: "/var/log/only1mcp/only1mcp.log"
        level: "debug"
        max_size_mb: 100
        max_backups: 7
        max_age_days: 30
        
      - type: "syslog"        # Syslog output
        level: "warn"
        facility: "local0"
        tag: "only1mcp"
        
  # Audit logging
  audit:
    enabled: false            # Enable audit logging
    log_requests: true        # Log all MCP requests
    log_admin: true          # Log admin operations
    
    # Sensitive data handling
    redact_sensitive: true    # Redact sensitive data
    redact_patterns:          # Patterns to redact
      - "password"
      - "token"
      - "secret"
      - "key"

# Advanced features
advanced:
  # Plugin system
  plugins:
    enabled: false            # Enable plugin system
    directory: "~/.only1mcp/plugins"  # Plugin directory
    auto_load: true          # Auto-load plugins on startup
    
  # Service discovery
  service_discovery:
    enabled: false
    type: "mdns"             # Options: mdns, consul, etcd
    
    # mDNS configuration
    mdns:
      service_name: "_mcp._tcp"
      domain: "local"
      
  # Circuit breaker
  circuit_breaker:
    enabled: true
    failure_threshold: 5      # Failures to open circuit
    success_threshold: 2      # Successes to close circuit
    timeout_seconds: 60       # Time before half-open
    
  # Chaos engineering (testing only)
  chaos:
    enabled: false            # NEVER enable in production
    failure_rate: 0.01        # Inject 1% failures
    latency_ms: 500          # Add random latency
    
# Development settings
development:
  # Debug mode
  debug: false                # Enable debug logging
  
  # Mock mode
  mock_servers: false         # Use mock servers for testing
  
  # Profiling
  profiling:
    enabled: false            # Enable CPU/memory profiling
    port: 6060               # Profiling endpoint port
```

---

## TOML SCHEMA REFERENCE

### Complete TOML Configuration (Equivalent to YAML)

```toml
# only1mcp.toml - TOML format configuration
# Equivalent to the YAML configuration above

[server]
host = "0.0.0.0"
port = 8080
worker_threads = 0
max_connections = 10000
request_timeout_ms = 30000

[server.tls]
enabled = false
cert_path = ""
key_path = ""
ca_path = ""
min_version = "1.3"

[server.admin]
enabled = true
port = 8081
path = "/admin"
auth_required = true

# Servers array in TOML
[[servers]]
id = "filesystem-server"
name = "Filesystem MCP"
enabled = true
timeout_ms = 5000

[servers.transport]
type = "stdio"
command = "npx"
args = ["@modelcontextprotocol/server-filesystem", "/home/user/data"]

[servers.transport.env]
NODE_ENV = "production"

[servers.health_check]
enabled = true
interval_seconds = 10
timeout_seconds = 5
failure_threshold = 3
success_threshold = 2

[servers.routing]
tools = ["read_file", "write_file", "list_directory"]
priority = 100
weight = 1

[servers.retry]
max_attempts = 3
initial_delay_ms = 100
max_delay_ms = 5000
multiplier = 2

# Second server
[[servers]]
id = "github-server"
name = "GitHub MCP"

[servers.transport]
type = "http"
url = "http://localhost:3000/mcp"

[servers.transport.headers]
Authorization = "Bearer ${GITHUB_TOKEN}"

# Proxy configuration
[proxy.load_balancer]
algorithm = "round_robin"
hash_key = "tool_name"
virtual_nodes = 150

[proxy.connection_pool]
max_per_backend = 100
min_idle = 10
max_idle_time_ms = 300000

[proxy.request]
max_body_size_mb = 100
compression = true

[proxy.response]
buffer_size_kb = 64
streaming = true

[proxy.hot_reload]
enabled = true
watch_interval_ms = 1000
debounce_ms = 500

# Context optimization
[context_optimization.cache]
enabled = true
max_entries = 10000
max_size_mb = 500
ttl_seconds = 300

[context_optimization.cache.tool_overrides.read_file]
ttl_seconds = 60

[context_optimization.cache.tool_overrides.list_directory]
ttl_seconds = 600

[context_optimization.batching]
enabled = true
max_batch_size = 50
batch_window_ms = 100

[context_optimization.lazy_loading]
enabled = true
preload_tools = []

[context_optimization.compression]
enabled = true
algorithm = "zstd"
level = 3

[context_optimization.token_estimation]
enabled = true
model = "claude-3"

# Authentication
[auth.admin]
enabled = true
type = "api_key"
api_key_env = "ONLY1MCP_ADMIN_KEY"
api_key_header = "X-API-Key"

[auth.client]
enabled = false
type = "bearer"

[auth.rate_limit]
enabled = true
type = "sliding_window"
requests_per_minute = 1000
requests_per_hour = 10000

[auth.rate_limit.per_client]
requests_per_minute = 60
burst = 10

[auth.rate_limit.exemptions]
addresses = ["127.0.0.1", "10.0.0.0/8"]

# Observability
[observability.metrics]
enabled = true
type = "prometheus"
port = 9090
path = "/metrics"

[observability.metrics.detailed]
per_tool = true
per_backend = true
histograms = true

[observability.logging]
level = "info"
format = "json"

[[observability.logging.outputs]]
type = "stdout"
level = "info"

[[observability.logging.outputs]]
type = "file"
path = "/var/log/only1mcp/only1mcp.log"
level = "debug"
max_size_mb = 100
max_backups = 7
max_age_days = 30

[observability.audit]
enabled = false
log_requests = true
log_admin = true
redact_sensitive = true
redact_patterns = ["password", "token", "secret", "key"]
```

---

## ENVIRONMENT VARIABLES

### Environment Variable Reference

All configuration values can be overridden via environment variables using the `ONLY1MCP_` prefix:

```bash
# Format: ONLY1MCP_<SECTION>_<FIELD>
# Nested fields use double underscores

# Server configuration
ONLY1MCP_SERVER__HOST=0.0.0.0
ONLY1MCP_SERVER__PORT=8080
ONLY1MCP_SERVER__WORKER_THREADS=8
ONLY1MCP_SERVER__TLS__ENABLED=true
ONLY1MCP_SERVER__TLS__CERT_PATH=/etc/ssl/cert.pem
ONLY1MCP_SERVER__TLS__KEY_PATH=/etc/ssl/key.pem

# Authentication secrets (common pattern)
ONLY1MCP_ADMIN_KEY=secret-admin-key-here
ONLY1MCP_JWT_SECRET=jwt-signing-secret-here
OAUTH_CLIENT_ID=google-oauth-client-id
OAUTH_CLIENT_SECRET=google-oauth-client-secret

# Backend server tokens (for HTTP transports)
GITHUB_TOKEN=ghp_xxxxxxxxxxxxxxxxxxxx
OPENAI_API_KEY=sk-xxxxxxxxxxxxxxxxxxxx

# Logging
ONLY1MCP_LOG_LEVEL=debug
ONLY1MCP_LOG_FORMAT=json

# Feature flags
ONLY1MCP_CACHE_ENABLED=true
ONLY1MCP_BATCHING_ENABLED=true
ONLY1MCP_HOT_RELOAD_ENABLED=false  # Disable in production

# Performance tuning
ONLY1MCP_MAX_CONNECTIONS=50000
ONLY1MCP_REQUEST_TIMEOUT_MS=60000

# Database connection (if using persistent cache)
DATABASE_URL=postgres://user:pass@localhost/only1mcp
REDIS_URL=redis://localhost:6379
```

### Docker Environment File Example

```bash
# .env file for Docker deployment
# docker run --env-file .env only1mcp

# Core configuration
ONLY1MCP_SERVER__HOST=0.0.0.0
ONLY1MCP_SERVER__PORT=8080
ONLY1MCP_LOG_LEVEL=info

# TLS certificates (mounted as volumes)
ONLY1MCP_SERVER__TLS__ENABLED=true
ONLY1MCP_SERVER__TLS__CERT_PATH=/certs/fullchain.pem
ONLY1MCP_SERVER__TLS__KEY_PATH=/certs/privkey.pem

# Authentication
ONLY1MCP_ADMIN_KEY=${ADMIN_KEY}
ONLY1MCP_JWT_SECRET=${JWT_SECRET}

# Backend tokens
GITHUB_TOKEN=${GITHUB_TOKEN}
ANTHROPIC_API_KEY=${ANTHROPIC_API_KEY}
OPENAI_API_KEY=${OPENAI_API_KEY}

# Monitoring
ONLY1MCP_METRICS_ENABLED=true
ONLY1MCP_METRICS_PORT=9090

# Resource limits
ONLY1MCP_CACHE__MAX_SIZE_MB=1000
ONLY1MCP_MAX_CONNECTIONS=10000
```

---

## SOLO DEVELOPER CONFIGURATION

### Minimal Configuration for Individual Use

```yaml
# ~/.only1mcp/config.yaml
# Solo developer - local development focus

# Just the essentials - everything else uses smart defaults
servers:
  # File system access
  - id: "fs"
    name: "Files"
    transport:
      type: "stdio"
      command: "npx"
      args: ["@modelcontextprotocol/server-filesystem", "${HOME}/projects"]
    
  # GitHub repositories
  - id: "github"
    name: "GitHub"
    transport:
      type: "stdio"
      command: "npx"
      args: ["@modelcontextprotocol/server-github"]
      env:
        GITHUB_TOKEN: "${GITHUB_TOKEN}"
    
  # Local SQLite database
  - id: "db"
    name: "Database"
    transport:
      type: "stdio"
      command: "npx"
      args: ["@modelcontextprotocol/server-sqlite", "${HOME}/data/app.db"]
    
  # Web browser automation
  - id: "browser"
    name: "Browser"
    transport:
      type: "stdio"
      command: "npx"
      args: ["@modelcontextprotocol/server-puppeteer"]

# Enable all optimizations for token savings
context_optimization:
  cache:
    enabled: true
    ttl_seconds: 600         # Cache for 10 minutes
  batching:
    enabled: true
  lazy_loading:
    enabled: true            # Critical for token savings

# Simple logging for debugging
observability:
  logging:
    level: "info"
    format: "pretty"         # Human-readable logs
```

### VS Code / Cursor Integration

```json
// .vscode/settings.json
{
  "mcp.servers": {
    "only1mcp": {
      "command": "only1mcp",
      "args": ["proxy", "--config", "${workspaceFolder}/.only1mcp/config.yaml"],
      "env": {
        "GITHUB_TOKEN": "${env:GITHUB_TOKEN}",
        "ONLY1MCP_LOG_LEVEL": "debug"
      }
    }
  }
}
```

---

## SMALL TEAM CONFIGURATION

### Configuration for 5-20 Person Teams

```yaml
# /etc/only1mcp/team-config.yaml
# Small team - shared development resources

server:
  host: "0.0.0.0"
  port: 8080
  # Enable TLS for network access
  tls:
    enabled: true
    cert_path: "/etc/letsencrypt/live/mcp.team.com/fullchain.pem"
    key_path: "/etc/letsencrypt/live/mcp.team.com/privkey.pem"

servers:
  # Shared code repository
  - id: "team-github"
    name: "Team GitHub"
    transport:
      type: "http"
      url: "https://github-mcp.team.internal:3000/mcp"
      headers:
        Authorization: "Bearer ${TEAM_GITHUB_TOKEN}"
    routing:
      priority: 100
    
  # Shared documentation
  - id: "confluence"
    name: "Confluence Docs"
    transport:
      type: "http"
      url: "https://confluence-mcp.team.internal:3001/mcp"
    
  # Team database (read-only for most)
  - id: "staging-db"
    name: "Staging Database"
    transport:
      type: "stdio"
      command: "npx"
      args: ["@modelcontextprotocol/server-postgres"]
      env:
        DATABASE_URL: "${STAGING_DB_URL}"
    health_check:
      enabled: true
      interval_seconds: 30
    
  # Slack integration
  - id: "slack"
    name: "Team Slack"
    transport:
      type: "http"
      url: "https://slack-mcp.team.internal:3002/mcp"

# Team authentication - simple API keys
auth:
  client:
    enabled: true
    type: "api_key"
    api_key_header: "X-Team-Key"
  
  # Rate limiting per developer
  rate_limit:
    enabled: true
    per_client:
      requests_per_minute: 100
      requests_per_hour: 5000

# Basic RBAC for team members
rbac:
  enabled: true
  roles:
    - name: "developer"
      permissions:
        - "tools:*"           # Use all tools
        - "servers:read"      # View server status
    
    - name: "admin"
      permissions:
        - "*"                # Full access
  
  # Map team members to roles
  users:
    - id: "alice@team.com"
      roles: ["admin"]
    - id: "bob@team.com"
      roles: ["developer"]
    - id: "carol@team.com"
      roles: ["developer"]

# Shared cache for team efficiency
context_optimization:
  cache:
    enabled: true
    max_entries: 50000       # Larger cache for team
    max_size_mb: 2000       # 2GB cache
    ttl_seconds: 1800       # 30 minute TTL
  
  # Batch requests from multiple developers
  batching:
    enabled: true
    max_batch_size: 100
    batch_window_ms: 200

# Team monitoring
observability:
  metrics:
    enabled: true
    type: "prometheus"
    port: 9090
  
  # Centralized logging
  logging:
    level: "info"
    format: "json"
    outputs:
      - type: "syslog"       # Send to central log server
        facility: "local0"
        tag: "only1mcp"
```

### Docker Compose for Team Deployment

```yaml
# docker-compose.team.yaml
version: '3.8'

services:
  only1mcp:
    image: ghcr.io/only1mcp/only1mcp:latest
    ports:
      - "8080:8080"          # MCP proxy
      - "8081:8081"          # Admin UI
      - "9090:9090"          # Metrics
    volumes:
      - ./team-config.yaml:/etc/only1mcp/config.yaml:ro
      - /etc/letsencrypt:/etc/letsencrypt:ro
    environment:
      TEAM_GITHUB_TOKEN: ${TEAM_GITHUB_TOKEN}
      STAGING_DB_URL: ${STAGING_DB_URL}
      ONLY1MCP_LOG_LEVEL: info
    restart: unless-stopped
    networks:
      - mcp-network
    
  # Optional: Prometheus for metrics
  prometheus:
    image: prom/prometheus:latest
    volumes:
      - ./prometheus.yml:/etc/prometheus/prometheus.yml:ro
      - prometheus-data:/prometheus
    ports:
      - "9091:9090"
    networks:
      - mcp-network
    
  # Optional: Grafana for dashboards
  grafana:
    image: grafana/grafana:latest
    ports:
      - "3000:3000"
    volumes:
      - grafana-data:/var/lib/grafana
    networks:
      - mcp-network

networks:
  mcp-network:
    driver: bridge

volumes:
  prometheus-data:
  grafana-data:
```

---

## ENTERPRISE CONFIGURATION

### Production-Grade Configuration for Large Organizations

```yaml
# /etc/only1mcp/enterprise.yaml
# Enterprise deployment - 100+ developers, compliance requirements

server:
  host: "0.0.0.0"
  port: 443                  # Standard HTTPS port
  worker_threads: 32         # Dedicated threads for high load
  max_connections: 50000     # Support many concurrent users
  
  # Enterprise TLS with mTLS
  tls:
    enabled: true
    cert_path: "/certs/server.crt"
    key_path: "/certs/server.key"
    ca_path: "/certs/ca-bundle.crt"  # Client certificate validation
    min_version: "1.3"       # TLS 1.3 only for security
    
  # Separate admin interface
  admin:
    enabled: true
    port: 8443              # Separate port for admin
    path: "/admin"
    auth_required: true

# Enterprise MCP servers with high availability
servers:
  # Primary GitHub Enterprise
  - id: "ghe-primary"
    name: "GitHub Enterprise (Primary)"
    transport:
      type: "http"
      url: "https://ghe-mcp-1.corp.internal/mcp"
      headers:
        Authorization: "Bearer ${GHE_TOKEN}"
        X-Correlation-ID: "${REQUEST_ID}"
    health_check:
      enabled: true
      interval_seconds: 5    # Aggressive health checking
      timeout_seconds: 2
      failure_threshold: 2
    routing:
      priority: 100
      weight: 2             # Higher capacity server
    retry:
      max_attempts: 3
      initial_delay_ms: 50
      max_delay_ms: 1000
    
  # Secondary GitHub Enterprise (failover)
  - id: "ghe-secondary"
    name: "GitHub Enterprise (Secondary)"
    transport:
      type: "http"
      url: "https://ghe-mcp-2.corp.internal/mcp"
      headers:
        Authorization: "Bearer ${GHE_TOKEN}"
    health_check:
      enabled: true
      interval_seconds: 5
    routing:
      priority: 50          # Lower priority for failover
      weight: 1
      
  # Production database cluster
  - id: "prod-db-read"
    name: "Production DB (Read Replica)"
    transport:
      type: "http"
      url: "https://db-mcp.corp.internal/mcp"
    routing:
      tools: ["query", "analyze"]  # Read-only operations
      
  # Confluence Enterprise
  - id: "confluence"
    name: "Confluence Documentation"
    transport:
      type: "http"
      url: "https://confluence-mcp.corp.internal/mcp"
    routing:
      tools: ["search_docs", "read_page"]
      
  # JIRA integration
  - id: "jira"
    name: "JIRA Issue Tracker"
    transport:
      type: "http"
      url: "https://jira-mcp.corp.internal/mcp"
      
  # ServiceNow for ITSM
  - id: "servicenow"
    name: "ServiceNow ITSM"
    transport:
      type: "http"
      url: "https://servicenow-mcp.corp.internal/mcp"

# Enterprise proxy configuration
proxy:
  load_balancer:
    algorithm: "consistent_hash"  # Session affinity
    hash_key: "client_id"
    virtual_nodes: 200
    
  connection_pool:
    max_per_backend: 500    # High connection limits
    min_idle: 50
    max_idle_time_ms: 600000  # 10 minute idle timeout
    
  request:
    max_body_size_mb: 500   # Large file support
    compression: true
    
  hot_reload:
    enabled: false          # Disabled for stability
    
  # Circuit breaker for resilience
  circuit_breaker:
    enabled: true
    failure_threshold: 10
    success_threshold: 5
    timeout_seconds: 30
    half_open_requests: 5

# Advanced context optimization
context_optimization:
  cache:
    enabled: true
    max_entries: 100000     # Large cache for enterprise
    max_size_mb: 10000     # 10GB cache
    ttl_seconds: 3600      # 1 hour default TTL
    
    # Use Redis for distributed cache
    backend: "redis"
    redis:
      url: "redis://redis-cluster.corp.internal:6379"
      password_env: "REDIS_PASSWORD"
      cluster: true
      
  batching:
    enabled: true
    max_batch_size: 200
    batch_window_ms: 50
    
  compression:
    enabled: true
    algorithm: "zstd"
    level: 6               # Higher compression for storage

# Enterprise authentication with SSO
auth:
  admin:
    enabled: true
    type: "oauth2"
    oauth2:
      provider: "okta"     # Enterprise SSO
      client_id_env: "OKTA_CLIENT_ID"
      client_secret_env: "OKTA_CLIENT_SECRET"
      issuer: "https://corp.okta.com/oauth2/default"
      redirect_url: "https://mcp.corp.com/auth/callback"
      scopes: ["openid", "profile", "email", "groups"]
      
  client:
    enabled: true
    type: "mtls"          # Mutual TLS for clients
    
  rate_limit:
    enabled: true
    type: "sliding_window"
    # Department-based limits
    groups:
      - name: "engineering"
        requests_per_minute: 500
        requests_per_hour: 20000
      - name: "data-science"
        requests_per_minute: 1000
        requests_per_hour: 50000
      - name: "default"
        requests_per_minute: 100
        requests_per_hour: 5000

# Enterprise RBAC with AD/LDAP groups
rbac:
  enabled: true
  
  # Sync with Active Directory groups
  ldap:
    enabled: true
    url: "ldaps://ad.corp.com"
    bind_dn: "CN=svc-only1mcp,OU=Service Accounts,DC=corp,DC=com"
    bind_password_env: "LDAP_PASSWORD"
    user_base_dn: "OU=Users,DC=corp,DC=com"
    group_base_dn: "OU=Groups,DC=corp,DC=com"
    sync_interval_minutes: 60
    
  roles:
    - name: "admin"
      permissions: ["*"]
      ldap_groups: ["CN=MCP-Admins,OU=Groups,DC=corp,DC=com"]
      
    - name: "developer"
      permissions:
        - "tools:*"
        - "servers:read"
        - "metrics:read"
      ldap_groups: ["CN=Engineering,OU=Groups,DC=corp,DC=com"]
      
    - name: "data-scientist"
      permissions:
        - "tools:query"
        - "tools:analyze"
        - "servers:read"
      ldap_groups: ["CN=DataScience,OU=Groups,DC=corp,DC=com"]
      
    - name: "auditor"
      permissions:
        - "audit:read"
        - "metrics:read"
        - "servers:read"
      ldap_groups: ["CN=Compliance,OU=Groups,DC=corp,DC=com"]

# Enterprise observability
observability:
  metrics:
    enabled: true
    type: "prometheus"
    port: 9090
    
    # Detailed metrics for capacity planning
    detailed:
      per_tool: true
      per_backend: true
      per_user: true       # Track per-user metrics
      histograms: true
      
  # Distributed tracing for debugging
  tracing:
    enabled: true
    type: "jaeger"
    jaeger:
      agent_endpoint: "jaeger-agent.corp.internal:6831"
      sampling_rate: 0.001  # Sample 0.1% in production
      
  # Comprehensive logging
  logging:
    level: "info"
    format: "json"
    
    outputs:
      # ELK stack integration
      - type: "elasticsearch"
        urls: ["https://es-cluster.corp.internal:9200"]
        index: "only1mcp"
        
      # Backup to S3
      - type: "s3"
        bucket: "corp-logs"
        prefix: "only1mcp/"
        region: "us-east-1"
        
  # Audit logging for compliance
  audit:
    enabled: true
    log_requests: true
    log_admin: true
    log_auth: true
    
    # Compliance requirements
    retention_days: 2555   # 7 years for compliance
    
    # PII redaction
    redact_sensitive: true
    redact_patterns:
      - "\\b[A-Z]{2}[0-9]{2}\\s?[A-Z]{3}\\b"  # UK National Insurance
      - "\\b[0-9]{3}-[0-9]{2}-[0-9]{4}\\b"     # US SSN
      - "\\b[A-Z][0-9]{7}\\b"                  # UK Driving License
      
    # Immutable audit trail
    backend: "blockchain"  # Or use append-only database
    
# High availability configuration
high_availability:
  enabled: true
  
  # Clustering for multiple Only1MCP instances
  clustering:
    enabled: true
    node_id: "only1mcp-prod-1"
    peers:
      - "only1mcp-prod-2.corp.internal:8080"
      - "only1mcp-prod-3.corp.internal:8080"
    
  # Session replication
  session_replication:
    enabled: true
    backend: "redis"
    
  # State synchronization
  state_sync:
    enabled: true
    interval_ms: 1000
    
# Compliance and security
compliance:
  # Data residency
  data_residency:
    enabled: true
    allowed_regions: ["us-east-1", "eu-west-1"]
    
  # Encryption at rest
  encryption:
    enabled: true
    algorithm: "AES-256-GCM"
    key_management: "vault"  # HashiCorp Vault
    vault:
      url: "https://vault.corp.internal"
      auth_method: "kubernetes"
      
  # HIPAA compliance
  hipaa:
    enabled: true
    phi_detection: true
    audit_trail: true
    encryption_required: true
    
  # SOC2 compliance
  soc2:
    enabled: true
    change_control: true
    access_reviews: true
    vulnerability_scanning: true
```

---

## SPECIALIZED CONFIGURATIONS

### AI Research Team Configuration

```yaml
# ML/AI research team with GPU servers and notebooks

servers:
  # Jupyter notebooks
  - id: "jupyter"
    name: "Jupyter Notebooks"
    transport:
      type: "http"
      url: "http://jupyter-hub.research.internal:8000/mcp"
    routing:
      tools: ["execute_cell", "read_notebook", "list_notebooks"]
      
  # MLflow experiment tracking
  - id: "mlflow"
    name: "MLflow Experiments"
    transport:
      type: "http"
      url: "http://mlflow.research.internal:5000/mcp"
      
  # GPU cluster management
  - id: "slurm"
    name: "SLURM GPU Cluster"
    transport:
      type: "stdio"
      command: "python"
      args: ["mcp_servers/slurm_mcp.py"]
      
  # Vector database for RAG
  - id: "pinecone"
    name: "Pinecone Vector DB"
    transport:
      type: "http"
      url: "https://pinecone-mcp.internal:8090/mcp"
      headers:
        Api-Key: "${PINECONE_API_KEY}"

# Optimize for large model outputs
proxy:
  response:
    buffer_size_kb: 512    # Large buffers for model outputs
    streaming: true
    
context_optimization:
  # Different caching for experiments
  cache:
    enabled: true
    tool_overrides:
      execute_cell:
        ttl_seconds: 0     # Never cache execution
      read_notebook:
        ttl_seconds: 3600  # Cache notebooks for 1 hour
```

### DevOps Team Configuration

```yaml
# DevOps team with infrastructure automation

servers:
  # Kubernetes cluster
  - id: "k8s"
    name: "Kubernetes"
    transport:
      type: "stdio"
      command: "kubectl"
      args: ["mcp", "serve"]
    routing:
      tools: ["get_pods", "describe_service", "logs", "exec"]
      
  # Terraform state
  - id: "terraform"
    name: "Terraform"
    transport:
      type: "http"
      url: "https://terraform-cloud.internal/mcp"
      
  # AWS resources
  - id: "aws"
    name: "AWS"
    transport:
      type: "stdio"
      command: "aws-mcp"
      env:
        AWS_PROFILE: "production"
        
  # Monitoring stack
  - id: "prometheus"
    name: "Prometheus Metrics"
    transport:
      type: "http"
      url: "http://prometheus.internal:9090/mcp"

# DevOps specific optimizations
proxy:
  request:
    timeout_ms: 60000      # Long timeout for infra operations
    
context_optimization:
  cache:
    tool_overrides:
      get_pods:
        ttl_seconds: 10    # Very short cache for live data
      describe_service:
        ttl_seconds: 300   # 5 min cache for service info
```

---

## CONFIGURATION VALIDATION

### Schema Validation Rules

```yaml
# Validation performed on configuration load

validation_rules:
  # Required fields
  required:
    - servers                # At least one server must be configured
    
  # Type validations
  types:
    server.port:
      type: integer
      min: 1
      max: 65535
    
    server.worker_threads:
      type: integer
      min: 0              # 0 means auto-detect
      max: 256
      
    servers[].transport.type:
      type: string
      enum: ["stdio", "http", "sse"]
      
    proxy.load_balancer.algorithm:
      type: string
      enum: ["round_robin", "least_connections", "random", 
             "weighted", "consistent_hash"]
      
  # Conditional validations
  conditionals:
    - if: server.tls.enabled
      then_required: [server.tls.cert_path, server.tls.key_path]
      
    - if: proxy.load_balancer.algorithm == "consistent_hash"
      then_required: [proxy.load_balancer.hash_key]
      
    - if: auth.admin.type == "oauth2"
      then_required: [auth.admin.oauth2.provider, 
                     auth.admin.oauth2.client_id_env]
      
  # Cross-field validations
  cross_field:
    - name: "cache_size_check"
      condition: "cache.max_size_mb >= cache.max_entries * 0.001"
      error: "Cache size too small for max_entries"
      
    - name: "port_conflict"
      condition: "server.port != server.admin.port"
      error: "Main and admin ports must be different"
```

### Validation CLI Tool

```bash
# Validate configuration before deployment
only1mcp validate --config /path/to/config.yaml

# Example output for valid config
✓ Configuration valid
  - 4 servers configured
  - TLS enabled with valid certificates
  - Authentication configured
  - All health checks properly configured

# Example output for invalid config
✗ Configuration errors found:
  - Line 23: server.tls.cert_path: file not found: /etc/ssl/cert.pem
  - Line 45: servers[2].transport.type: invalid value "websocket", must be one of: stdio, http, sse
  - Line 67: proxy.load_balancer.algorithm: "ip_hash" is not supported
  
Fix these errors and run validation again.
```

### Runtime Validation

```rust
// Rust code for runtime configuration validation

use serde::{Deserialize, Serialize};
use validator::{Validate, ValidationError};

#[derive(Debug, Deserialize, Serialize, Validate)]
struct ServerConfig {
    #[validate(length(min = 1, max = 255))]
    host: String,
    
    #[validate(range(min = 1, max = 65535))]
    port: u16,
    
    #[validate(range(min = 0, max = 256))]
    worker_threads: usize,
    
    #[validate]
    tls: Option<TlsConfig>,
}

#[derive(Debug, Deserialize, Serialize, Validate)]
struct TlsConfig {
    enabled: bool,
    
    #[validate(custom = "validate_file_exists")]
    cert_path: Option<String>,
    
    #[validate(custom = "validate_file_exists")]
    key_path: Option<String>,
}

fn validate_file_exists(path: &str) -> Result<(), ValidationError> {
    if !std::path::Path::new(path).exists() {
        return Err(ValidationError::new("file_not_found"));
    }
    Ok(())
}

// Custom validation for complex rules
impl ServerConfig {
    fn validate_config(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        
        // Check TLS configuration consistency
        if let Some(tls) = &self.tls {
            if tls.enabled && (tls.cert_path.is_none() || tls.key_path.is_none()) {
                errors.push("TLS enabled but certificates not provided".to_string());
            }
        }
        
        // Check port conflicts
        if self.port < 1024 && !is_root() {
            errors.push(format!("Port {} requires root privileges", self.port));
        }
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}
```

---

## MIGRATION GUIDES

### From Docker MCP Toolkit

```yaml
# Migration from Docker MCP Toolkit to Only1MCP

# Step 1: Export Docker configuration
# docker-mcp-config.json -> only1mcp.yaml

# Original Docker MCP config
{
  "mcpServers": {
    "filesystem": {
      "command": "docker",
      "args": ["run", "-i", "--rm", "mcp/filesystem", "/data"]
    }
  }
}

# Converted Only1MCP config
servers:
  - id: "filesystem"
    name: "Filesystem"
    transport:
      type: "stdio"
      command: "npx"  # Native execution instead of Docker
      args: ["@modelcontextprotocol/server-filesystem", "/data"]
    
# Benefits after migration:
# - 70% reduction in resource usage (no Docker overhead)
# - 10x faster startup time
# - Native performance
# - Simpler debugging
```

### From TBXark mcp-proxy (Go)

```yaml
# Migration from TBXark/mcp-proxy

# Original mcp-proxy config.json
{
  "servers": [
    {
      "name": "server1",
      "url": "http://localhost:3000"
    }
  ],
  "port": 8080
}

# Converted Only1MCP config with enhancements
server:
  port: 8080

servers:
  - id: "server1"
    name: "Server 1"
    transport:
      type: "http"
      url: "http://localhost:3000"
    
    # Additional features not in mcp-proxy:
    health_check:
      enabled: true           # Automatic health monitoring
    retry:
      max_attempts: 3         # Automatic retry on failure

# New capabilities gained:
context_optimization:
  cache:
    enabled: true            # Response caching (not in mcp-proxy)
  batching:
    enabled: true            # Request batching (not in mcp-proxy)
```

### From Direct MCP Connections

```yaml
# Before: Multiple MCP servers configured in Claude/Cursor
# After: Single Only1MCP endpoint

# Step 1: List current MCP servers in AI client
# Step 2: Add all to Only1MCP configuration

servers:
  # Move all individual server configs here
  - id: "github"
    transport:
      type: "stdio"
      command: "npx"
      args: ["@modelcontextprotocol/server-github"]
      
  - id: "postgres"
    transport:
      type: "stdio"  
      command: "npx"
      args: ["@modelcontextprotocol/server-postgres"]
      
  # ... add all other servers

# Step 3: Update AI client to use Only1MCP
# Old: 5 separate MCP endpoints
# New: 1 Only1MCP endpoint (http://localhost:8080)

# Step 4: Enable optimizations
context_optimization:
  cache:
    enabled: true
  lazy_loading:
    enabled: true            # Huge token savings
    
# Result: 60-70% reduction in context tokens
```

---

## ADVANCED PATTERNS

### Dynamic Server Discovery

```yaml
# Automatic discovery of MCP servers on network

advanced:
  service_discovery:
    enabled: true
    type: "mdns"             # Multicast DNS for local network
    
    mdns:
      service_name: "_mcp._tcp"
      domain: "local"
      auto_add: true         # Automatically add discovered servers
      
    # Filter discovered servers
    filters:
      - pattern: "^test-"    # Ignore test servers
        action: "exclude"
      - pattern: "^prod-"    # Only production servers
        action: "include"
```

### Multi-Region Deployment

```yaml
# Global deployment with regional routing

# Region detection
proxy:
  routing:
    mode: "geo"              # Geographic routing
    
    regions:
      us-east:
        servers: ["us-east-1", "us-east-2"]
        fallback: "us-west"
        
      eu-central:
        servers: ["eu-central-1", "eu-central-2"]
        fallback: "us-east"
        
      asia-pacific:
        servers: ["ap-southeast-1", "ap-northeast-1"]
        fallback: "us-west"
```

### Canary Deployments

```yaml
# Gradual rollout of new server versions

servers:
  - id: "api-stable"
    name: "Stable API"
    transport:
      type: "http"
      url: "http://api-v1.internal/mcp"
    routing:
      weight: 90             # 90% of traffic
      
  - id: "api-canary"
    name: "Canary API"
    transport:
      type: "http"
      url: "http://api-v2.internal/mcp"
    routing:
      weight: 10             # 10% of traffic
      tags: ["canary", "beta"]
      
# Monitor canary performance
observability:
  metrics:
    detailed:
      per_backend: true      # Compare stable vs canary
```

### Custom Authentication Providers

```yaml
# Integration with custom auth systems

auth:
  admin:
    type: "custom"
    custom:
      # Custom auth endpoint
      validate_url: "https://auth.company.com/validate"
      
      # Headers to forward
      forward_headers:
        - "Authorization"
        - "X-API-Key"
        - "X-Request-ID"
        
      # Response parsing
      user_id_field: "user.id"
      roles_field: "user.roles"
      
      # Cache auth results
      cache_ttl_seconds: 300
```

### Intelligent Request Routing

```yaml
# ML-based routing (future feature)

proxy:
  routing:
    mode: "intelligent"
    
    ml_routing:
      enabled: true
      model: "routing_v1"    # Pre-trained routing model
      
      # Features for ML routing
      features:
        - "tool_name"
        - "request_size"
        - "time_of_day"
        - "client_history"
        
      # Online learning
      online_learning:
        enabled: true
        feedback_endpoint: "/feedback"
        update_interval_minutes: 60
```

---

## CONFIGURATION BEST PRACTICES

### Security Checklist

```yaml
# ✅ Security best practices

# 1. Always use TLS in production
server:
  tls:
    enabled: true            # ✅ REQUIRED
    min_version: "1.3"       # ✅ TLS 1.3 only
    
# 2. Never store secrets in config files
auth:
  api_key_env: "ONLY1MCP_KEY"  # ✅ Use environment variable
  # api_key: "secret123"        # ❌ NEVER do this
  
# 3. Enable authentication
auth:
  client:
    enabled: true            # ✅ Always authenticate clients
    
# 4. Set up rate limiting
auth:
  rate_limit:
    enabled: true            # ✅ Prevent abuse
    
# 5. Enable audit logging
observability:
  audit:
    enabled: true            # ✅ Track all operations
    
# 6. Use least privilege
rbac:
  enabled: true              # ✅ Role-based access
  default_role: "readonly"   # ✅ Minimal default permissions
```

### Performance Optimization

```yaml
# ⚡ Performance best practices

# 1. Enable all caching
context_optimization:
  cache:
    enabled: true            # ⚡ Reduces backend load
    max_size_mb: 1000       # ⚡ Size based on available RAM
    
# 2. Use connection pooling
proxy:
  connection_pool:
    max_per_backend: 100     # ⚡ Reuse connections
    min_idle: 10            # ⚡ Keep warm connections
    
# 3. Enable compression
proxy:
  request:
    compression: true        # ⚡ Reduce network traffic
    
# 4. Use consistent hashing for session affinity
proxy:
  load_balancer:
    algorithm: "consistent_hash"  # ⚡ Better cache hits
    
# 5. Optimize health checks
servers:
  - health_check:
      interval_seconds: 30   # ⚡ Not too frequent
      timeout_seconds: 5     # ⚡ Quick timeout
```

### Monitoring Setup

```yaml
# 📊 Monitoring best practices

# 1. Enable comprehensive metrics
observability:
  metrics:
    enabled: true
    detailed:
      per_tool: true         # 📊 Track tool usage
      per_backend: true      # 📊 Monitor backend health
      histograms: true       # 📊 Latency distributions
      
# 2. Structured logging
observability:
  logging:
    format: "json"           # 📊 Machine-readable
    level: "info"           # 📊 Appropriate verbosity
    
# 3. Distributed tracing (large deployments)
observability:
  tracing:
    enabled: true            # 📊 Debug request flow
    sampling_rate: 0.01      # 📊 1% sampling in prod
    
# 4. Alerting thresholds
monitoring:
  alerts:
    error_rate: 0.01         # 📊 Alert if >1% errors
    p99_latency_ms: 1000     # 📊 Alert if p99 >1s
    cache_hit_rate: 0.5      # 📊 Alert if <50% cache hits
```

---

## APPENDIX: Quick Reference

### Common Configuration Snippets

```yaml
# Enable everything for development
development:
  all_features: true
  
# Minimal production config
production:
  only_essentials: true
  
# Maximum performance mode
performance:
  optimize_everything: true
  
# Maximum security mode
security:
  paranoid_mode: true
```

### Environment Variable Quick Reference

```bash
# Essential environment variables
export ONLY1MCP_CONFIG=/path/to/config.yaml
export ONLY1MCP_LOG_LEVEL=info
export ONLY1MCP_ADMIN_KEY=your-secret-key
export GITHUB_TOKEN=ghp_xxxxxxxxxxxx
export DATABASE_URL=postgres://localhost/db
```

### Common CLI Commands

```bash
# Validate configuration
only1mcp validate

# Test configuration (dry run)
only1mcp test --config config.yaml

# Start with specific config
only1mcp start --config /etc/only1mcp/config.yaml

# Export configuration template
only1mcp config generate --type enterprise > config.yaml

# Convert between formats
only1mcp config convert --from config.yaml --to config.toml

# Validate and fix common issues
only1mcp config doctor
```

---

**Document Status:** ✓ COMPLETE  
**Schema Version:** 1.0.0  
**Next Review:** January 2026  
**Feedback:** config@only1mcp.dev

*This configuration reference represents the complete schema for Only1MCP v1.0. For latest updates and configuration migrations, visit https://docs.only1mcp.dev/configuration*
