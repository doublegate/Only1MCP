# Only1MCP User Documentation Strategy
## Comprehensive Documentation Framework for End Users, Operators, and Integrators

**Document Version:** 1.0  
**Documentation Scope:** Installation, Configuration, Operation, Integration, Troubleshooting  
**Target Audiences:** End Users, System Administrators, AI Developers, Enterprise Teams  
**Date:** October 14, 2025  
**Status:** Documentation Strategy Specification

---

## TABLE OF CONTENTS

1. [Documentation Philosophy](#documentation-philosophy)
2. [Audience Segmentation](#audience-segmentation)
3. [Documentation Architecture](#documentation-architecture)
4. [Quick Start Guides](#quick-start-guides)
5. [Configuration Documentation](#configuration-documentation)
6. [API Reference Documentation](#api-reference-documentation)
7. [Tutorial Framework](#tutorial-framework)
8. [Interactive Examples](#interactive-examples)
9. [Troubleshooting Guides](#troubleshooting-guides)
10. [Migration Documentation](#migration-documentation)
11. [Video Documentation Strategy](#video-documentation-strategy)
12. [Documentation Toolchain](#documentation-toolchain)
13. [Localization Strategy](#localization-strategy)
14. [Documentation Maintenance](#documentation-maintenance)
15. [Community Contributions](#community-contributions)

---

## DOCUMENTATION PHILOSOPHY

### Core Principles

**1. Zero-to-Hero in 5 Minutes**
```markdown
Our primary documentation metric: A new user should achieve their first 
successful MCP aggregation within 5 minutes of finding our documentation.
```

**Research Context:** Current MCP server setup takes 30+ minutes with frequent JSON editing errors. Users report spending hours debugging configuration issues【Document 02, Section 3】.

**2. Progressive Disclosure**
- **Level 1**: Quick start - get running immediately
- **Level 2**: Common configurations - solve 80% of use cases
- **Level 3**: Advanced features - power users and enterprise
- **Level 4**: Internals - contributors and extenders

**3. Example-Driven Learning**
```rust
// Every concept introduced with a working example
// Bad documentation:
"Configure the load balancer algorithm in the config file."

// Good documentation:
"Configure load balancing by setting the algorithm in config.yaml:
```yaml
proxy:
  load_balancer:
    algorithm: consistent_hash  # Options: round_robin, least_connections, consistent_hash
    virtual_nodes: 200          # For consistent_hash only (default: 150)
```
This distributes requests evenly across backend servers."
```

**4. Problem-Solution Format**
Address the documented pain points directly:
- **Problem**: "All my MCP servers active = 30k+ tokens"
- **Solution**: Step-by-step guide to context optimization
- **Result**: "Reduced to 12k tokens (60% savings)"

**5. Visual-First for Complex Concepts**
- Architecture diagrams for system overview
- Flowcharts for decision trees
- Screenshots for UI features
- GIF recordings for multi-step processes

---

## AUDIENCE SEGMENTATION

### Primary Audiences

#### 1. Solo Developers (60% of users)
**Profile:**
- 5-8 MCP servers (filesystem, GitHub, database, browser)
- Local development focus
- Pain: Context bloat, configuration complexity

**Documentation Needs:**
- Ultra-simple installation (one-liner)
- Pre-built configurations for common stacks
- VS Code / Cursor integration guides
- Personal productivity tips

#### 2. Small Teams (25% of users)
**Profile:**
- 10-20 MCP servers across team
- Mixed local/cloud deployment
- Pain: Consistency across environments

**Documentation Needs:**
- Team configuration templates
- Docker Compose examples
- CI/CD integration guides
- Collaboration workflows

#### 3. Enterprise Teams (10% of users)
**Profile:**
- 100+ developers, 50+ MCP servers
- Compliance requirements (HIPAA, SOC2)
- Pain: Centralized management, audit trails

**Documentation Needs:**
- Enterprise deployment guides
- Security hardening checklists
- RBAC configuration
- Integration with enterprise tools (LDAP, SAML)

#### 4. MCP Server Developers (5% of users)
**Profile:**
- Building custom MCP servers
- Need aggregation for testing
- Pain: Development workflow efficiency

**Documentation Needs:**
- MCP protocol implementation guides
- Testing harness documentation
- Plugin development guides
- Performance optimization tips

---

## DOCUMENTATION ARCHITECTURE

### Documentation Site Structure

```
docs.only1mcp.dev/
├── getting-started/
│   ├── installation/          # Platform-specific installers
│   │   ├── linux/
│   │   ├── macos/
│   │   ├── windows/
│   │   └── docker/
│   ├── quickstart/            # 5-minute guide
│   ├── first-aggregation/     # Hello world example
│   └── basic-concepts/        # MCP primer
│
├── configuration/
│   ├── file-reference/        # config.yaml schema
│   ├── visual-configurator/   # UI guide
│   ├── cli-configuration/     # CLI commands
│   ├── hot-reload/           # Zero-downtime updates
│   └── templates/            # Pre-built configs
│       ├── solo-developer/
│       ├── small-team/
│       └── enterprise/
│
├── features/
│   ├── context-optimization/  # Token reduction techniques
│   ├── load-balancing/       # Distribution strategies
│   ├── health-checks/        # Monitoring backends
│   ├── caching/              # Response caching
│   ├── security/             # Auth, TLS, RBAC
│   └── plugins/              # Extension system
│
├── integrations/
│   ├── ai-clients/           # Claude, GPT, Cursor
│   ├── mcp-servers/          # Server catalog
│   ├── monitoring/           # Prometheus, Grafana
│   ├── cloud-providers/      # AWS, GCP, Azure
│   └── ci-cd/               # GitHub Actions, GitLab
│
├── api-reference/
│   ├── mcp-endpoints/        # Standard MCP API
│   ├── admin-api/            # Management endpoints
│   ├── metrics-api/          # Monitoring endpoints
│   └── sdk/                  # Client libraries
│
├── tutorials/
│   ├── video-tutorials/      # Embedded YouTube
│   ├── interactive/          # Try in browser
│   ├── use-cases/           # Real-world scenarios
│   └── workshops/           # Self-paced learning
│
├── troubleshooting/
│   ├── common-issues/        # FAQ-style
│   ├── error-reference/      # Error codes
│   ├── debugging/            # Debug techniques
│   └── support/             # Getting help
│
├── migration/
│   ├── from-docker-toolkit/  # Docker MCP migration
│   ├── from-mcp-proxy/       # TBXark migration
│   ├── from-direct/          # No aggregator → Only1MCP
│   └── version-upgrades/     # Only1MCP v1 → v2
│
└── reference/
    ├── architecture/         # System design
    ├── benchmarks/          # Performance data
    ├── changelog/           # Release notes
    ├── glossary/           # Term definitions
    └── research/           # Background studies
```

### Documentation Versions

```yaml
# Version strategy
versions:
  current: v1.2.0          # Latest stable
  lts: v1.0.0             # Long-term support
  next: v2.0.0-beta       # Preview/upcoming
  legacy:                 # Maintained versions
    - v0.9.x
    - v0.8.x
```

---

## QUICK START GUIDES

### Installation One-Liners

```bash
# Linux/macOS - Shell installer with automatic architecture detection
curl -sSfL https://only1mcp.dev/install.sh | sh

# Windows - PowerShell installer
irm https://only1mcp.dev/install.ps1 | iex

# macOS - Homebrew
brew install only1mcp

# Docker (optional containerized deployment)
docker run -d -p 8080:8080 ghcr.io/only1mcp/only1mcp:latest

# Cargo (for Rust developers)
cargo install only1mcp
```

### 5-Minute Quick Start Template

```markdown
# Only1MCP Quick Start

Get your first MCP aggregation running in 5 minutes!

## 1. Install (30 seconds)

\`\`\`bash
# macOS/Linux
curl -sSfL https://only1mcp.dev/install.sh | sh

# Verify installation
only1mcp --version
\`\`\`

## 2. Generate Config (1 minute)

\`\`\`bash
# Interactive configuration wizard
only1mcp init

# Questions:
# - How many MCP servers? (e.g., 3)
# - Server 1 type? (filesystem/github/web)
# - Server 1 path? (/usr/local/bin/mcp-filesystem)
# ...

# Creates: ~/.only1mcp/config.yaml
\`\`\`

## 3. Start Proxy (30 seconds)

\`\`\`bash
# Start Only1MCP
only1mcp start

# Output:
# ✅ Started Only1MCP on http://localhost:8080
# ✅ Proxying 3 MCP servers
# ✅ Context reduction: 60% (20k → 8k tokens)
\`\`\`

## 4. Configure AI Client (2 minutes)

**Claude Desktop:**
\`\`\`json
{
  "mcpServers": {
    "only1mcp": {
      "command": "curl",
      "args": ["-X", "POST", "http://localhost:8080/mcp"]
    }
  }
}
\`\`\`

**Cursor:**
\`\`\`json
{
  "mcp.servers": ["http://localhost:8080/mcp"]
}
\`\`\`

## 5. Test It! (1 minute)

Ask your AI: "What tools are available?"

You should see all tools from all your MCP servers, aggregated through Only1MCP!

## Next Steps

- [Optimize context usage](../features/context-optimization) (-70% tokens)
- [Add more servers](../configuration/cli-configuration#add-server)
- [Enable caching](../features/caching) (instant responses)
- [Set up monitoring](../integrations/monitoring) (Grafana dashboards)
```

### Platform-Specific Guides

**Windows-Specific Quick Start:**
```markdown
# Windows Installation Guide

## Prerequisites
- Windows 10 version 1903+ or Windows 11
- PowerShell 5.0+ (pre-installed)

## Installation Methods

### Method 1: PowerShell Script (Recommended)
\`\`\`powershell
# Run as Administrator
Set-ExecutionPolicy Bypass -Scope Process -Force
irm https://only1mcp.dev/install.ps1 | iex
\`\`\`

### Method 2: Manual Installation
1. Download: https://github.com/doublegate/Only1MCP/releases/latest/download/only1mcp-windows-amd64.exe
2. Add to PATH: 
   - Press Win+X, select "System"
   - Click "Advanced system settings"
   - Click "Environment Variables"
   - Add C:\Program Files\Only1MCP to PATH
3. Verify: Open new PowerShell, run `only1mcp --version`

### Method 3: Chocolatey
\`\`\`powershell
choco install only1mcp
\`\`\`

## Windows-Specific Configuration

### Firewall Rules
Only1MCP needs to accept incoming connections on port 8080:
\`\`\`powershell
New-NetFirewallRule -DisplayName "Only1MCP" -Direction Inbound -Protocol TCP -LocalPort 8080 -Action Allow
\`\`\`

### Windows Service Installation
Run Only1MCP as a Windows service for automatic startup:
\`\`\`powershell
only1mcp service install
only1mcp service start
\`\`\`

### WSL2 Integration
If using WSL2, expose Only1MCP to WSL:
\`\`\`bash
# In WSL2
export ONLY1MCP_URL="http://$(cat /etc/resolv.conf | grep nameserver | awk '{print $2}'):8080"
\`\`\`
```

---

## CONFIGURATION DOCUMENTATION

### Visual Configuration Guide

```markdown
# Visual Configuration Interface

Only1MCP eliminates manual JSON editing with an intuitive web UI.

## Accessing the Configuration UI

1. Start Only1MCP: `only1mcp start`
2. Open browser: http://localhost:8080/admin
3. Default credentials: admin/changeme (first-time setup)

## UI Sections

### Dashboard Overview
![Dashboard Screenshot](images/dashboard.png)

The dashboard shows:
- Active MCP servers (green = healthy, red = unhealthy)
- Real-time metrics (requests/sec, latency, cache hits)
- Context savings meter (showing token reduction %)
- Quick actions toolbar

### Adding a Server

1. Click "Add Server" button
2. Choose server type:
   - **Filesystem**: Local file access
   - **GitHub**: Repository access
   - **Database**: SQL queries
   - **Web Search**: Internet access
   - **Custom**: Any MCP server

3. Configure server:
   ![Add Server Dialog](images/add-server.png)
   
   - **Name**: Human-readable identifier
   - **Transport**: STDIO (local) or HTTP (remote)
   - **Command**: For STDIO servers (e.g., npx)
   - **Arguments**: Command arguments
   - **Health Check**: Enable/disable monitoring

4. Test connection (automatic)
5. Save configuration

### Hot-Swapping Servers

Change servers without restarting:

1. Toggle server on/off with switch
2. Drag to reorder priority
3. Click "Apply Changes"
4. Zero downtime - existing connections preserved!

### Context Optimization Settings

![Context Settings](images/context-settings.png)

Reduce token usage:
- **Dynamic Loading**: Load tools on-demand ✅
- **Response Caching**: Cache for 5 minutes ✅
- **Batch Requests**: Combine multiple calls ✅
- **Compression**: Gzip responses ✅

Estimated savings: 12,450 tokens → 4,980 tokens (60% reduction)
```

### Configuration File Reference

```yaml
# ~/.only1mcp/config.yaml - Complete Reference
# Full configuration with all options documented

# Configuration version (required)
version: "1.0"

# Proxy settings - core aggregator configuration
proxy:
  # Network binding
  host: "0.0.0.0"              # Interface to bind (0.0.0.0 = all interfaces)
  port: 8080                   # Port number (default: 8080)
  
  # TLS configuration (optional, recommended for production)
  tls:
    enabled: false             # Enable HTTPS
    cert: "/path/to/cert.pem"  # TLS certificate path
    key: "/path/to/key.pem"    # TLS private key path
    
  # Context optimization - reduce AI token consumption
  context_optimization:
    # Caching - store responses for idempotent requests
    cache:
      enabled: true            # Enable response caching
      ttl_seconds: 300         # Cache time-to-live (5 minutes)
      max_size_mb: 1024        # Maximum cache size
      
    # Batching - combine multiple requests
    batching:
      enabled: true            # Enable request batching
      max_batch_size: 10       # Maximum requests per batch
      max_wait_ms: 100         # Maximum wait time for batch
      
    # Dynamic loading - load tool schemas on-demand
    dynamic_loading:
      enabled: true            # Enable lazy tool loading
      preload: ["web_search"]  # Tools to always preload
      
    # Compression - reduce payload size
    compression:
      enabled: true            # Enable response compression
      algorithm: "gzip"        # Options: gzip, brotli, none
      level: 6                 # Compression level (1-9)
  
  # Load balancing - distribute requests across backends
  load_balancer:
    algorithm: "consistent_hash"  # Options: round_robin, least_connections, consistent_hash
    virtual_nodes: 200            # Virtual nodes for consistent hash (default: 150)
    
  # Health checking - monitor backend availability
  health:
    enabled: true              # Enable health checks
    interval_seconds: 10       # Check interval
    timeout_seconds: 5         # Health check timeout
    failure_threshold: 3       # Failures before marking unhealthy
    success_threshold: 2       # Successes before marking healthy

# Authentication - secure access to proxy and admin API
auth:
  # Admin API authentication
  admin:
    enabled: true              # Require auth for admin endpoints
    type: "api_key"           # Options: api_key, jwt, oauth2
    api_key_env: "ONLY1MCP_ADMIN_KEY"  # Environment variable with key
    
  # Client authentication (AI agents)
  client:
    enabled: false            # Require auth for MCP endpoints
    type: "bearer"           # Options: bearer, api_key
    
  # JWT configuration (if using JWT auth)
  jwt:
    secret_env: "ONLY1MCP_JWT_SECRET"  # JWT signing secret
    expiry_hours: 24                   # Token expiration time
    
  # OAuth2 configuration (enterprise)
  oauth2:
    provider: "google"        # OAuth provider
    client_id_env: "OAUTH_CLIENT_ID"
    client_secret_env: "OAUTH_CLIENT_SECRET"
    redirect_url: "http://localhost:8080/auth/callback"
    
  # Rate limiting - prevent abuse
  rate_limit:
    enabled: true             # Enable rate limiting
    requests_per_minute: 60   # Request limit per client
    burst: 10                 # Burst allowance

# Logging - application logs and audit trail
logging:
  # Log level: trace, debug, info, warn, error
  level: "info"
  
  # Output format: pretty (human), json (structured)
  format: "pretty"
  
  # Log outputs
  outputs:
    - type: "stdout"          # Console output
      
    - type: "file"            # File output
      path: "/var/log/only1mcp/only1mcp.log"
      max_size_mb: 100        # Rotate at size
      max_backups: 7          # Keep N old files
      
    - type: "syslog"          # Syslog output (Linux/macOS)
      facility: "local0"
      
  # Audit logging - track all actions
  audit:
    enabled: true             # Enable audit logs
    path: "/var/log/only1mcp/audit.log"
    include_requests: true    # Log full requests
    include_responses: false  # Log full responses (privacy)

# Metrics - Prometheus-compatible metrics
metrics:
  enabled: true               # Enable metrics endpoint
  port: 9090                  # Metrics port
  path: "/metrics"            # Metrics path
  
  # Custom labels for all metrics
  labels:
    environment: "production"
    region: "us-east-1"

# MCP Backend Servers - the servers to aggregate
servers:
  # Filesystem MCP server (STDIO transport)
  - id: "filesystem-1"                    # Unique identifier
    name: "Local Filesystem"              # Display name
    enabled: true                          # Enable/disable without removing
    transport: "stdio"                     # Transport type: stdio, http, sse
    
    # STDIO-specific settings
    command: "npx"                         # Command to execute
    args:                                  # Command arguments
      - "@modelcontextprotocol/server-filesystem"
      - "/home/user/projects"
    
    # Environment variables for server
    env:
      MCP_LOG_LEVEL: "debug"
      
    # Working directory
    working_dir: "/home/user"
    
    # Process limits (Linux/macOS)
    limits:
      memory_mb: 512                      # Memory limit
      cpu_percent: 50                      # CPU limit (% of one core)
      timeout_seconds: 30                  # Max execution time per request
    
    # Tool filtering - only expose specific tools
    tool_filter:
      mode: "allow"                        # allow or deny
      tools: ["file_read", "file_write"]  # Tool names
      
    # Custom health check
    health_check:
      enabled: true
      method: "ping"                       # Health check method
      interval_seconds: 30                 # Override global interval
  
  # GitHub MCP server (HTTP transport)
  - id: "github-1"
    name: "GitHub API"
    enabled: true
    transport: "http"
    
    # HTTP-specific settings
    url: "https://github-mcp.example.com/mcp"  # Server URL
    
    # Authentication for backend
    auth:
      type: "bearer"                       # Auth type: bearer, basic, api_key
      token_env: "GITHUB_TOKEN"           # Token from environment
      
    # HTTP client settings
    timeout_ms: 5000                      # Request timeout
    retries: 3                             # Retry attempts
    retry_delay_ms: 1000                   # Delay between retries
    
    # Custom headers
    headers:
      X-Custom-Header: "value"
      
    # TLS settings for backend
    tls:
      verify: true                         # Verify certificates
      ca_cert: "/path/to/ca.pem"          # Custom CA certificate
      
  # Database MCP server (Streamable HTTP)
  - id: "postgres-1"
    name: "PostgreSQL Database"
    enabled: true
    transport: "streamable_http"
    
    url: "https://db-mcp.internal:9443/mcp"
    
    # Connection pooling
    pool:
      min_connections: 2
      max_connections: 10
      idle_timeout_seconds: 300
      
    # Query limits
    limits:
      max_query_time_seconds: 60
      max_result_size_mb: 100
      
    # Failover configuration
    failover:
      enabled: true
      backup_servers:
        - "https://db-mcp-replica1.internal:9443/mcp"
        - "https://db-mcp-replica2.internal:9443/mcp"
      strategy: "round_robin"             # or "failover"

# Advanced features
advanced:
  # Plugin system
  plugins:
    enabled: false
    directory: "/etc/only1mcp/plugins"
    
  # Service discovery
  discovery:
    enabled: false
    type: "consul"                        # consul, mdns, kubernetes
    consul:
      address: "consul.internal:8500"
      service_prefix: "mcp-"
      
  # Distributed tracing
  tracing:
    enabled: false
    type: "opentelemetry"
    endpoint: "http://jaeger.internal:4317"
    
  # Feature flags
  features:
    experimental_wasm_plugins: false
    ai_routing_optimization: false
    zero_copy_streaming: true
```

---

## API REFERENCE DOCUMENTATION

### OpenAPI/Swagger Documentation

```yaml
# openapi.yaml - Auto-generated from code
openapi: 3.0.0
info:
  title: Only1MCP API
  description: |
    Only1MCP provides three API surfaces:
    1. **MCP API** - Standard Model Context Protocol endpoints for AI clients
    2. **Admin API** - Management endpoints for configuration and monitoring
    3. **Metrics API** - Prometheus-compatible metrics endpoint
  version: 1.2.0
  contact:
    email: api@only1mcp.dev
  license:
    name: Apache 2.0
    url: https://www.apache.org/licenses/LICENSE-2.0

servers:
  - url: http://localhost:8080
    description: Local development
  - url: https://only1mcp.example.com
    description: Production

paths:
  /mcp:
    post:
      summary: MCP JSON-RPC Endpoint
      description: Main endpoint for MCP protocol communication
      operationId: mcpRequest
      tags:
        - MCP
      requestBody:
        required: true
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/JsonRpcRequest'
            examples:
              listTools:
                summary: List available tools
                value:
                  jsonrpc: "2.0"
                  id: 1
                  method: "tools/list"
              callTool:
                summary: Execute a tool
                value:
                  jsonrpc: "2.0"
                  id: 2
                  method: "tools/call"
                  params:
                    name: "web_search"
                    arguments:
                      query: "rust async patterns"
      responses:
        '200':
          description: Successful response
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/JsonRpcResponse'
```

### SDK Documentation

```markdown
# Only1MCP Client Libraries

## Official SDKs

### Rust Client
\`\`\`rust
use only1mcp_client::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create client
    let client = Client::new("http://localhost:8080")?;
    
    // List tools
    let tools = client.list_tools().await?;
    println!("Available tools: {:?}", tools);
    
    // Call a tool
    let result = client.call_tool("web_search", json!({
        "query": "rust async patterns"
    })).await?;
    
    Ok(())
}
\`\`\`

### Python Client
\`\`\`python
from only1mcp import Client

# Create client
client = Client("http://localhost:8080")

# List tools
tools = await client.list_tools()
print(f"Available tools: {tools}")

# Call a tool
result = await client.call_tool("web_search", {
    "query": "rust async patterns"
})
\`\`\`

### TypeScript/JavaScript Client
\`\`\`typescript
import { Client } from '@only1mcp/client';

// Create client
const client = new Client('http://localhost:8080');

// List tools
const tools = await client.listTools();
console.log('Available tools:', tools);

// Call a tool
const result = await client.callTool('web_search', {
  query: 'rust async patterns'
});
\`\`\`
```

---

## TUTORIAL FRAMEWORK

### Interactive Tutorial System

```markdown
# Interactive Tutorials

Learn Only1MCP with hands-on, interactive tutorials that run in your browser.

## Tutorial Categories

### 🚀 Getting Started (30 minutes)
1. **Installation & Setup** (5 min)
   - Install Only1MCP
   - Verify installation
   - Explore CLI commands
   
2. **Your First Aggregation** (10 min)
   - Add filesystem MCP server
   - Start proxy
   - Connect AI client
   - Test tool execution
   
3. **Adding Multiple Servers** (15 min)
   - Add GitHub server
   - Add web search server
   - Observe unified tool list
   - Compare token usage

### 🎯 Context Optimization (45 minutes)
1. **Measuring Context Usage** (10 min)
   - Use metrics endpoint
   - Calculate token consumption
   - Identify optimization opportunities
   
2. **Enabling Caching** (15 min)
   - Configure cache settings
   - Test with repeated queries
   - Monitor cache hit rate
   - Measure performance improvement
   
3. **Dynamic Tool Loading** (20 min)
   - Enable lazy loading
   - Configure preloaded tools
   - Test on-demand loading
   - Verify token savings

### 🏢 Enterprise Features (60 minutes)
1. **Setting Up RBAC** (20 min)
   - Create user roles
   - Assign permissions
   - Test access controls
   - Audit log review
   
2. **Implementing OAuth2** (20 min)
   - Configure OAuth provider
   - Set up redirects
   - Test authentication flow
   - Token management
   
3. **Production Deployment** (20 min)
   - TLS configuration
   - Health checks setup
   - Monitoring integration
   - Backup procedures

## Interactive Playground

Try Only1MCP in your browser without installation:

<iframe src="https://play.only1mcp.dev" width="100%" height="600px"></iframe>

Features:
- Pre-configured MCP servers
- Live configuration editing
- Real-time metrics
- Sample AI client
- Export configuration
```

### Use Case Tutorials

```markdown
# Real-World Use Cases

## Use Case 1: AI-Powered Development Environment

### The Challenge
Sarah is a full-stack developer using Cursor with 8 MCP servers:
- Filesystem (2 instances for different projects)
- GitHub (for code repository)
- PostgreSQL (development database)
- Redis (cache)
- Docker (container management)
- Jira (task tracking)
- Slack (team communication)

**Problem**: Each Cursor session starts with 45,000 tokens of MCP context!

### The Solution

#### Step 1: Install and Configure Only1MCP
\`\`\`yaml
# sarah-dev-config.yaml
version: "1.0"

proxy:
  context_optimization:
    cache:
      enabled: true
      ttl_seconds: 600  # 10 minutes for dev
    dynamic_loading:
      enabled: true
      preload: ["file_read", "github_pull"]  # Most used

servers:
  - id: "project-a-files"
    name: "Project A"
    transport: "stdio"
    command: "npx"
    args: ["@modelcontextprotocol/server-filesystem", "/home/sarah/project-a"]
    tool_filter:
      mode: "allow"
      tools: ["file_read", "file_write", "file_search"]
    
  - id: "project-b-files"
    name: "Project B"
    transport: "stdio"
    command: "npx"
    args: ["@modelcontextprotocol/server-filesystem", "/home/sarah/project-b"]
    enabled: false  # Toggle based on current project
    
  # ... other servers
\`\`\`

#### Step 2: Project-Based Profiles
\`\`\`bash
# Create profiles for different projects
only1mcp profile create project-a --from sarah-dev-config.yaml
only1mcp profile set-active project-a

# Switch profiles when changing projects
only1mcp profile switch project-b
\`\`\`

#### Step 3: Cursor Integration
\`\`\`json
// .cursor/settings.json
{
  "mcp.server": "http://localhost:8080/mcp",
  "mcp.optimization": {
    "lazy": true,
    "cache": true
  }
}
\`\`\`

### The Result
- **Before**: 45,000 tokens at session start
- **After**: 12,000 tokens (73% reduction!)
- **Performance**: 50ms average tool response (was 300ms)
- **Productivity**: No more "context limit reached" errors

### Key Learnings
1. Use project profiles to toggle servers
2. Cache frequently accessed data (10 min TTL for dev)
3. Preload only essential tools
4. Monitor token usage via metrics endpoint
```

---

## INTERACTIVE EXAMPLES

### Code Sandbox Integration

```html
<!-- Embedded CodeSandbox for live examples -->
<div class="example-container">
  <h3>Try It Live: Basic Configuration</h3>
  <iframe
    src="https://codesandbox.io/embed/only1mcp-basic-config-x7j4k?fontsize=14&hidenavigation=1&theme=dark"
    style="width:100%; height:500px; border:0; border-radius: 4px; overflow:hidden;"
    title="Only1MCP Basic Configuration"
    allow="accelerometer; ambient-light-sensor; camera; encrypted-media; geolocation; gyroscope; hid; microphone; midi; payment; usb; vr; xr-spatial-tracking"
    sandbox="allow-forms allow-modals allow-popups allow-presentation allow-same-origin allow-scripts"
  ></iframe>
</div>

<script>
// Interactive configuration builder
function buildConfig() {
  const config = {
    version: "1.0",
    proxy: {
      port: document.getElementById('port').value,
      context_optimization: {
        cache: {
          enabled: document.getElementById('cache').checked,
          ttl_seconds: parseInt(document.getElementById('ttl').value)
        }
      }
    },
    servers: []
  };
  
  // Add servers from form
  document.querySelectorAll('.server-config').forEach(server => {
    config.servers.push({
      id: server.querySelector('.server-id').value,
      name: server.querySelector('.server-name').value,
      transport: server.querySelector('.transport').value,
      // ... more fields
    });
  });
  
  // Display YAML
  document.getElementById('config-output').textContent = 
    jsyaml.dump(config);
  
  // Estimate token savings
  const tokens = estimateTokens(config);
  document.getElementById('token-estimate').textContent = 
    `Estimated tokens: ${tokens.optimized} (saved ${tokens.saved}%)`;
}
</script>
```

### CLI Playground

```markdown
# Interactive CLI Tutorial

<div id="terminal"></div>

<script src="https://cdn.jsdelivr.net/npm/xterm@4.19.0/lib/xterm.min.js"></script>
<script>
// Create interactive terminal
const term = new Terminal();
term.open(document.getElementById('terminal'));

// Simulated Only1MCP CLI
const commands = {
  'only1mcp --version': 'only1mcp 1.2.0',
  'only1mcp list': `
┌──────────────┬─────────────────┬───────────┬──────────┐
│ ID           │ Name            │ Transport │ Status   │
├──────────────┼─────────────────┼───────────┼──────────┤
│ filesystem-1 │ Local Files     │ STDIO     │ ✅ Healthy│
│ github-1     │ GitHub API      │ HTTP      │ ✅ Healthy│
│ web-search-1 │ Web Search      │ HTTP      │ ⚠️ Degraded│
└──────────────┴─────────────────┴───────────┴──────────┘
  `,
  'only1mcp add': 'Interactive mode: Use only1mcp add <name> <url>',
  'only1mcp metrics': `
Requests Total: 1,234
Error Rate: 0.2%
Cache Hit Rate: 78%
Active Connections: 3
Token Savings: 62%
  `
};

// Handle commands
term.onData(data => {
  // Process input and show output
  if (data === '\r') {
    const cmd = term.buffer.active.cursorX;
    const output = commands[cmd] || 'Command not found';
    term.write('\r\n' + output + '\r\n$ ');
  } else {
    term.write(data);
  }
});

term.write('$ ');
</script>
```

---

## TROUBLESHOOTING GUIDES

### Common Issues Matrix

```markdown
# Troubleshooting Guide

## Quick Diagnosis

### Connection Issues

| Symptom | Possible Cause | Solution |
|---------|---------------|----------|
| "Connection refused" on port 8080 | Only1MCP not running | Run `only1mcp start` |
| "Address already in use" | Port conflict | Change port in config or kill conflicting process |
| "TLS handshake failed" | Certificate issues | Check cert paths and expiry |
| "Backend timeout" | Slow MCP server | Increase timeout_seconds in config |

### Context/Token Issues

| Symptom | Possible Cause | Solution |
|---------|---------------|----------|
| Token count not reducing | Caching disabled | Enable cache in config |
| "Context window exceeded" | Too many servers active | Use dynamic loading or disable unused servers |
| Slow tool discovery | No caching | Enable cache with 5-minute TTL |
| Duplicate tools | Name collision | Use namespacing or tool_filter |

### Performance Issues

| Symptom | Possible Cause | Solution |
|---------|---------------|----------|
| High latency (>100ms) | No connection pooling | Enable connection pooling |
| Memory usage growing | Cache unbounded | Set max_size_mb for cache |
| CPU spike on startup | Health checks | Stagger health check intervals |
| Degraded after hours | Memory leak | Update to latest version |

## Detailed Troubleshooting

### Issue: "All MCP servers showing unhealthy"

**Diagnosis Steps:**

1. **Check Only1MCP logs:**
\`\`\`bash
only1mcp logs --level debug --tail 50
\`\`\`

2. **Test backend directly:**
\`\`\`bash
# For STDIO server
npx @modelcontextprotocol/server-filesystem --test

# For HTTP server
curl -X POST http://backend-server:9000/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"ping"}'
\`\`\`

3. **Verify health check configuration:**
\`\`\`yaml
health:
  interval_seconds: 10  # Not too aggressive
  timeout_seconds: 5    # Sufficient for slow servers
  failure_threshold: 3  # Allow transient failures
\`\`\`

4. **Check network connectivity:**
\`\`\`bash
# Test DNS resolution
nslookup backend-server.example.com

# Test port connectivity
telnet backend-server.example.com 9000

# Test with curl
curl -v http://backend-server.example.com:9000/health
\`\`\`

**Common Solutions:**

- **Firewall blocking**: Add firewall rules for MCP ports
- **DNS issues**: Use IP addresses instead of hostnames
- **Process died**: Restart backend MCP servers
- **Resource limits**: Increase memory/CPU limits

### Issue: "Configuration changes not taking effect"

**Diagnosis Steps:**

1. **Verify hot-reload is enabled:**
\`\`\`bash
only1mcp config get proxy.hot_reload
# Should return: true
\`\`\`

2. **Check for config errors:**
\`\`\`bash
only1mcp validate --config ~/.only1mcp/config.yaml
\`\`\`

3. **Monitor reload events:**
\`\`\`bash
only1mcp logs --filter hot-reload --follow
\`\`\`

**Solution:**
\`\`\`bash
# Force reload if hot-reload fails
only1mcp reload

# Or restart completely
only1mcp restart
\`\`\`

### Error Code Reference

| Error Code | Meaning | Resolution |
|------------|---------|------------|
| -32001 | Backend Timeout | Increase timeout_seconds or check backend health |
| -32002 | No Backend Available | All backends unhealthy, check health checks |
| -32003 | Authentication Failed | Verify API keys/tokens in environment |
| -32004 | Rate Limit Exceeded | Reduce request rate or increase limits |
| -32005 | Tool Not Found | Tool doesn't exist or is filtered out |
| -32006 | Backend Error | Check backend server logs |
| -32007 | Invalid Configuration | Run `only1mcp validate` to find issues |
| -32008 | Cache Error | Clear cache with `only1mcp cache clear` |
```

### Debug Mode Guide

```markdown
# Debug Mode Guide

## Enabling Debug Mode

### Method 1: Environment Variable
\`\`\`bash
RUST_LOG=debug only1mcp start
\`\`\`

### Method 2: Configuration
\`\`\`yaml
logging:
  level: debug
\`\`\`

### Method 3: CLI Flag
\`\`\`bash
only1mcp start --debug
\`\`\`

## Debug Output Interpretation

### Understanding Log Levels

\`\`\`
TRACE: Every function call and data flow
DEBUG: Detailed operational information
INFO:  High-level operational messages
WARN:  Warning conditions
ERROR: Error conditions
\`\`\`

### Sample Debug Output
\`\`\`
[2025-10-14T10:30:45Z DEBUG only1mcp::proxy::router] Incoming request: method=tools/list
[2025-10-14T10:30:45Z TRACE only1mcp::proxy::cache] Cache lookup: key=0xDEADBEEF
[2025-10-14T10:30:45Z DEBUG only1mcp::proxy::cache] Cache miss, forwarding to backend
[2025-10-14T10:30:45Z TRACE only1mcp::transport::http] Sending request to http://backend:9000
[2025-10-14T10:30:45Z DEBUG only1mcp::transport::http] Response received: 200 OK, 1.2KB
[2025-10-14T10:30:45Z TRACE only1mcp::proxy::cache] Caching response: ttl=300s
[2025-10-14T10:30:45Z DEBUG only1mcp::proxy::router] Request completed: duration=23ms
\`\`\`

### Key Debug Patterns to Watch

1. **Slow Requests**: Look for high duration values
2. **Cache Misses**: Frequent misses indicate poor cache config
3. **Retries**: Multiple attempts suggest unstable backends
4. **Connection Pool**: "No available connections" indicates exhaustion
```

---

## MIGRATION DOCUMENTATION

### From Docker MCP Toolkit

```markdown
# Migrating from Docker MCP Toolkit to Only1MCP

## Why Migrate?

| Feature | Docker MCP Toolkit | Only1MCP |
|---------|-------------------|----------|
| Setup Time | 30+ minutes | 5 minutes |
| Docker Desktop Required | ✅ Yes ($) | ❌ No |
| Context Overhead | High (all tools loaded) | Low (60% reduction) |
| Configuration | JSON editing | Visual UI |
| Hot-reload | ❌ No | ✅ Yes |
| Performance | ~300ms latency | <50ms latency |

## Migration Steps

### Step 1: Export Docker MCP Configuration

\`\`\`bash
# List current MCP containers
docker ps --filter label=mcp.server

# Export configuration
docker inspect mcp-filesystem | jq '.[0].Config.Env' > docker-config.json
\`\`\`

### Step 2: Install Only1MCP

\`\`\`bash
# One-line installer
curl -sSfL https://only1mcp.dev/install.sh | sh
\`\`\`

### Step 3: Import Configuration

\`\`\`bash
# Automatic import tool
only1mcp import docker --config docker-config.json

# Creates: ~/.only1mcp/config.yaml with all your servers
\`\`\`

### Step 4: Optimize Configuration

The import tool creates a basic config. Optimize it:

\`\`\`yaml
# Before (direct import)
servers:
  - id: "filesystem"
    transport: "stdio"
    command: "docker"
    args: ["run", "mcp-filesystem"]  # Docker overhead!

# After (native execution)
servers:
  - id: "filesystem"
    transport: "stdio"
    command: "npx"
    args: ["@modelcontextprotocol/server-filesystem", "/path"]  # Direct execution!
\`\`\`

### Step 5: Update AI Client

**Before (Docker):**
\`\`\`json
{
  "mcp": {
    "servers": {
      "filesystem": {
        "command": "docker",
        "args": ["run", "-i", "mcp-filesystem"]
      },
      "github": {
        "command": "docker",
        "args": ["run", "-i", "mcp-github"]
      }
    }
  }
}
\`\`\`

**After (Only1MCP):**
\`\`\`json
{
  "mcp": {
    "server": "http://localhost:8080/mcp"  // Just one endpoint!
  }
}
\`\`\`

### Step 6: Verify Migration

\`\`\`bash
# Test all servers
only1mcp test all

# Compare token usage
only1mcp metrics --format comparison

# Output:
# Docker MCP: 45,000 tokens
# Only1MCP:   18,000 tokens (60% reduction!)
\`\`\`

## Migration Checklist

- [ ] Docker configuration exported
- [ ] Only1MCP installed
- [ ] Configuration imported and optimized
- [ ] Native MCP servers installed (npm packages)
- [ ] AI client updated to use Only1MCP
- [ ] All tools tested and working
- [ ] Performance improvement verified
- [ ] Docker containers stopped (save resources)
```

---

## VIDEO DOCUMENTATION STRATEGY

### Video Content Plan

```markdown
# Video Documentation Series

## 🎥 YouTube Channel Structure

### Playlists

1. **Getting Started (5 videos)**
   - Installation in 60 Seconds
   - Your First MCP Aggregation
   - Understanding Context Optimization
   - Visual Configuration Tour
   - Troubleshooting Basics

2. **Feature Deep Dives (10 videos)**
   - Load Balancing Strategies
   - Caching for Performance
   - Security & Authentication
   - Hot-Reload Magic
   - Plugin Development

3. **Use Case Walkthroughs (8 videos)**
   - Solo Developer Setup
   - Small Team Configuration
   - Enterprise Deployment
   - CI/CD Integration
   - Multi-Cloud Setup

4. **Live Coding Sessions (Monthly)**
   - Building Custom Plugins
   - Performance Optimization
   - Security Hardening
   - Community Q&A

## Video Templates

### Standard Tutorial Format (5-10 minutes)

\`\`\`
00:00 - Hook (problem statement)
00:30 - Introduction (what you'll learn)
01:00 - Prerequisites
01:30 - Main content (step-by-step)
08:00 - Results demonstration
09:00 - Next steps
09:30 - Call to action (subscribe, documentation)
\`\`\`

### Quick Tips Format (60 seconds)

\`\`\`
00:00 - Problem
00:10 - Solution
00:40 - Result
00:50 - More info link
\`\`\`

## Production Guidelines

### Recording Setup
- Resolution: 1920x1080 minimum
- Frame rate: 30fps
- Audio: Clear narration (noise reduction)
- Screen: High contrast terminal theme
- Annotations: Highlight key areas

### Tools
- Recording: OBS Studio
- Editing: DaVinci Resolve
- Animations: Manim (for diagrams)
- Thumbnails: Figma templates
```

### Interactive Video Tutorials

```html
<!-- Embedded interactive video with chapters -->
<div class="video-tutorial">
  <video id="tutorial-video" controls>
    <source src="https://cdn.only1mcp.dev/videos/getting-started.mp4" type="video/mp4">
    <!-- Chapters -->
    <track kind="chapters" src="chapters.vtt" srclang="en">
    <!-- Captions -->
    <track kind="captions" src="captions.vtt" srclang="en" label="English" default>
  </video>
  
  <!-- Interactive overlay -->
  <div id="video-overlay">
    <!-- Command overlay at 1:30 -->
    <div class="command-overlay" data-time="90">
      <pre>curl -sSfL https://only1mcp.dev/install.sh | sh</pre>
      <button onclick="copyCommand(this)">Copy</button>
    </div>
    
    <!-- Quiz overlay at 5:00 -->
    <div class="quiz-overlay" data-time="300">
      <h3>Quick Check</h3>
      <p>What's the default port for Only1MCP?</p>
      <button onclick="answer(8080)">8080</button>
      <button onclick="answer(3000)">3000</button>
      <button onclick="answer(9000)">9000</button>
    </div>
  </div>
</div>

<script>
// Sync overlays with video playback
const video = document.getElementById('tutorial-video');
const overlays = document.querySelectorAll('[data-time]');

video.addEventListener('timeupdate', () => {
  overlays.forEach(overlay => {
    const showTime = parseInt(overlay.dataset.time);
    if (Math.abs(video.currentTime - showTime) < 0.5) {
      overlay.classList.add('visible');
    }
  });
});
</script>
```

---

## DOCUMENTATION TOOLCHAIN

### Documentation Generation Pipeline

```yaml
# .github/workflows/docs.yml
name: Documentation Pipeline

on:
  push:
    paths:
      - 'docs/**'
      - 'src/**/*.rs'  # For rustdoc
      - 'examples/**'

jobs:
  build-docs:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      # Generate API documentation from source
      - name: Generate rustdoc
        run: |
          cargo doc --no-deps --all-features
          cp -r target/doc docs/api/rust
      
      # Generate OpenAPI spec
      - name: Generate OpenAPI
        run: |
          cargo run --bin generate-openapi > docs/api/openapi.yaml
          npx @redocly/openapi-cli bundle docs/api/openapi.yaml -o docs/api/openapi.json
      
      # Build mdBook documentation
      - name: Build mdBook
        run: |
          cargo install mdbook mdbook-mermaid mdbook-toc
          mdbook build docs/
      
      # Generate CLI documentation
      - name: Generate CLI docs
        run: |
          cargo run --bin only1mcp -- generate-docs --format markdown > docs/cli-reference.md
      
      # Deploy to GitHub Pages
      - name: Deploy
        uses: peaceiris/actions-gh-pages@v3
        with:
          github_token: ${{ secrets.GITHUB_TOKEN }}
          publish_dir: ./docs/book
```

### Documentation Tools Stack

```toml
# Cargo.toml - Documentation dependencies
[dev-dependencies]
# API documentation
utoipa = "4.0"           # OpenAPI generation
utoipa-swagger-ui = "4.0" # Swagger UI

# CLI documentation
clap_mangen = "4.0"      # Man page generation
clap_complete = "4.0"    # Shell completion

# Markdown processing
pulldown-cmark = "0.9"   # Markdown parsing
syntect = "5.0"         # Syntax highlighting
```

### Documentation Testing

```rust
// src/lib.rs - Doctest examples that are actually tested

/// Routes an MCP request to the appropriate backend server.
///
/// # Examples
///
/// ```
/// use only1mcp::{Router, McpRequest};
/// 
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let router = Router::new(Default::default());
/// 
/// let request = McpRequest {
///     jsonrpc: "2.0".to_string(),
///     id: 1,
///     method: "tools/list".to_string(),
///     params: None,
/// };
/// 
/// let response = router.route(request).await?;
/// assert_eq!(response.jsonrpc, "2.0");
/// # Ok(())
/// # }
/// ```
pub async fn route_request(&self, request: McpRequest) -> Result<McpResponse, Error> {
    // Implementation
}
```

---

## LOCALIZATION STRATEGY

### Supported Languages

```yaml
# i18n/languages.yaml
languages:
  en:
    name: English
    native: English
    complete: true
    maintainers: ["core-team"]
    
  zh-CN:
    name: Chinese (Simplified)
    native: 简体中文
    complete: true
    maintainers: ["community-zh"]
    
  es:
    name: Spanish
    native: Español
    complete: 90%
    maintainers: ["community-es"]
    
  ja:
    name: Japanese
    native: 日本語
    complete: 85%
    maintainers: ["community-ja"]
    
  de:
    name: German
    native: Deutsch
    complete: 75%
    maintainers: ["community-de"]
```

### Translation Workflow

```markdown
# Translation Guide

## For Translators

### Getting Started

1. Fork the repository
2. Create branch: `i18n-{lang}-{component}`
3. Translate files in `docs/i18n/{lang}/`
4. Submit PR with checklist

### Translation Guidelines

1. **Maintain Technical Accuracy**
   - Don't translate code, commands, or API names
   - Keep parameter names in English
   - Translate descriptions and explanations

2. **Consistency**
   - Use glossary for common terms
   - Follow style guide for your language
   - Maintain formatting and structure

3. **Examples**
   ```yaml
   # English
   error_no_backend: "No backend server available"
   
   # Spanish
   error_no_backend: "No hay servidor backend disponible"
   
   # Chinese
   error_no_backend: "没有可用的后端服务器"
   ```

### Tools

- **Crowdin Integration**: Automated sync with main branch
- **Translation Memory**: Reuse previous translations
- **Glossary**: Consistent terminology
- **Review System**: Native speaker verification
```

---

## DOCUMENTATION MAINTENANCE

### Version Control Strategy

```markdown
# Documentation Versioning

## Branch Strategy

- `main`: Current stable release docs
- `next`: Upcoming release documentation
- `archive/v1.0`: Historical documentation

## Version Banner

All documentation pages include version banner:

\`\`\`html
<div class="version-banner">
  ⚠️ You're viewing documentation for Only1MCP v1.2.0
  <a href="/docs/latest">View latest (v1.3.0)</a>
</div>
\`\`\`

## Deprecation Notices

\`\`\`markdown
> **Deprecated in v1.2.0**  
> This feature will be removed in v2.0.0. Use [new feature] instead.
> 
> Migration guide: [Migrating from X to Y](/migration/x-to-y)
\`\`\`
```

### Documentation Review Process

```yaml
# .github/CODEOWNERS - Documentation ownership
/docs/                    @only1mcp/docs-team
/docs/api/                @only1mcp/api-team
/docs/getting-started/    @only1mcp/dx-team
/docs/enterprise/         @only1mcp/enterprise-team
/docs/i18n/zh-CN/        @only1mcp/i18n-zh
/docs/i18n/es/           @only1mcp/i18n-es
```

### Metrics & Analytics

```javascript
// docs/assets/analytics.js - Documentation analytics

// Track documentation effectiveness
window.docAnalytics = {
  // Page metrics
  trackPageView: (page) => {
    gtag('event', 'page_view', {
      page_path: page,
      page_title: document.title
    });
  },
  
  // Search metrics
  trackSearch: (query, results) => {
    gtag('event', 'search', {
      search_term: query,
      results_count: results.length
    });
  },
  
  // Feedback metrics
  trackFeedback: (helpful, page) => {
    gtag('event', 'feedback', {
      helpful: helpful,
      page: page
    });
  },
  
  // Time on page
  trackReadTime: () => {
    const startTime = Date.now();
    window.addEventListener('beforeunload', () => {
      const readTime = Math.round((Date.now() - startTime) / 1000);
      gtag('event', 'read_time', {
        seconds: readTime,
        page: window.location.pathname
      });
    });
  }
};

// Initialize tracking
docAnalytics.trackPageView(window.location.pathname);
docAnalytics.trackReadTime();
```

---

## COMMUNITY CONTRIBUTIONS

### Documentation Contribution Guide

```markdown
# Contributing to Documentation

## Ways to Contribute

### 1. Fix Typos and Errors
- Spot a typo? Fix it directly on GitHub
- Click "Edit this page" on any doc page
- Submit PR with fix

### 2. Improve Explanations
- Clarify confusing sections
- Add missing context
- Provide better examples

### 3. Add Examples
- Real-world use cases
- Configuration examples
- Integration guides

### 4. Create Tutorials
- Video tutorials
- Interactive examples
- Workshop materials

### 5. Translate Documentation
- Join translation team
- Review translations
- Maintain language versions

## Contribution Process

1. **Find an Issue**
   - Check [documentation issues](https://github.com/doublegate/Only1MCP/labels/documentation)
   - Look for "good first issue" label
   - Comment to claim issue

2. **Make Changes**
   ```bash
   # Fork and clone
   git clone https://github.com/YOUR_USERNAME/only1mcp.git
   cd only1mcp/docs
   
   # Create branch
   git checkout -b docs/improve-quickstart
   
   # Make changes
   edit getting-started/quickstart.md
   
   # Preview locally
   mdbook serve
   ```

3. **Submit PR**
   - Clear description of changes
   - Link to related issue
   - Screenshots if applicable
   - Request review from @only1mcp/docs-team

## Style Guide

### Writing Style
- **Active voice**: "Configure the server" not "The server is configured"
- **Present tense**: "Only1MCP aggregates" not "Only1MCP will aggregate"
- **Direct address**: "You can configure" not "One can configure"
- **Concise**: Avoid unnecessary words

### Formatting
- **Code blocks**: Use language hints (\`\`\`bash, \`\`\`yaml)
- **Inline code**: Use backticks for `commands`, `filenames`, `values`
- **Bold**: Use for **important points** and **UI elements**
- **Lists**: Use for steps, options, features
- **Tables**: Use for comparisons, references

### Examples
Every feature should have:
1. What it does (description)
2. Why you'd use it (motivation)
3. How to use it (example)
4. What to expect (output)

## Recognition

Contributors are recognized in:
- [CONTRIBUTORS.md](https://github.com/doublegate/Only1MCP/blob/main/CONTRIBUTORS.md)
- Release notes
- Documentation footer
- Annual contributor report

### Contributor Levels
- 🌱 **Seedling**: First contribution
- 🌿 **Sprout**: 5+ contributions  
- 🌳 **Tree**: 20+ contributions
- 🌲 **Forest**: 50+ contributions
- 🏔️ **Mountain**: 100+ contributions
```

### Documentation Feedback System

```html
<!-- Feedback widget on every page -->
<div class="doc-feedback">
  <h4>Was this page helpful?</h4>
  <button onclick="sendFeedback(true)">👍 Yes</button>
  <button onclick="sendFeedback(false)">👎 No</button>
  
  <div id="feedback-form" style="display:none">
    <textarea placeholder="How can we improve this page?"></textarea>
    <button onclick="submitFeedback()">Submit</button>
  </div>
</div>

<script>
function sendFeedback(helpful) {
  // Track feedback
  docAnalytics.trackFeedback(helpful, window.location.pathname);
  
  // Show form for negative feedback
  if (!helpful) {
    document.getElementById('feedback-form').style.display = 'block';
  } else {
    showMessage('Thanks for your feedback!');
  }
}

function submitFeedback() {
  const feedback = document.querySelector('textarea').value;
  
  // Submit to GitHub Issues via API
  fetch('https://api.only1mcp.dev/feedback', {
    method: 'POST',
    headers: {'Content-Type': 'application/json'},
    body: JSON.stringify({
      page: window.location.pathname,
      feedback: feedback,
      type: 'documentation'
    })
  });
  
  showMessage('Feedback submitted. Thank you!');
}
</script>
```

---

## APPENDIX A: Documentation Templates

### Feature Documentation Template

```markdown
# [Feature Name]

## Overview

Brief description of what this feature does and why it's valuable.

## Use Cases

- **Scenario 1**: Description
- **Scenario 2**: Description
- **Scenario 3**: Description

## Configuration

### Basic Configuration

\`\`\`yaml
# Minimal configuration
feature:
  enabled: true
\`\`\`

### Advanced Configuration

\`\`\`yaml
# Full configuration with all options
feature:
  enabled: true
  option1: value1
  option2: value2
  advanced:
    setting1: value
    setting2: value
\`\`\`

### Configuration Reference

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| enabled | bool | false | Enable/disable feature |
| option1 | string | "default" | Description of option1 |
| option2 | number | 100 | Description of option2 |

## Examples

### Example 1: [Common Use Case]

\`\`\`bash
# Commands to demonstrate
only1mcp feature enable
only1mcp feature configure --option1 value
\`\`\`

Expected output:
\`\`\`
Feature enabled successfully
Configuration updated
\`\`\`

### Example 2: [Advanced Use Case]

[More complex example with explanation]

## Troubleshooting

### Common Issues

**Issue**: [Description]
**Solution**: [Step-by-step fix]

**Issue**: [Description]
**Solution**: [Step-by-step fix]

## Performance Impact

- **Memory**: +X MB when enabled
- **CPU**: Negligible impact
- **Network**: Y additional requests per minute

## Security Considerations

- Consideration 1
- Consideration 2

## Related Documentation

- [Related Feature 1](/docs/features/feature1)
- [Related Feature 2](/docs/features/feature2)
- [API Reference](/docs/api/feature)

## Changelog

- **v1.2.0**: Feature introduced
- **v1.2.1**: Added option2
- **v1.3.0**: Performance improvements
```

---

## APPENDIX B: Documentation Metrics Dashboard

```yaml
# metrics/documentation-kpis.yaml
# Key metrics for documentation effectiveness

metrics:
  engagement:
    - page_views_per_month: 50000
    - unique_visitors: 15000
    - average_time_on_page: "3:45"
    - bounce_rate: 32%
    
  effectiveness:
    - quickstart_completion_rate: 78%
    - search_success_rate: 85%
    - feedback_positive_rate: 92%
    - support_ticket_reduction: 40%
    
  content:
    - total_pages: 245
    - code_examples: 180
    - video_tutorials: 24
    - interactive_examples: 15
    
  community:
    - documentation_prs: 120
    - contributors: 45
    - translations: 5
    - feedback_submissions: 890
    
  search:
    top_searches:
      - "configuration"
      - "context optimization"
      - "docker migration"
      - "troubleshooting"
      - "enterprise setup"
    
  problem_areas:
    high_bounce_pages:
      - "/docs/advanced/plugins" (65% bounce)
      - "/docs/api/streaming" (58% bounce)
    
    low_feedback_pages:
      - "/docs/security/rbac" (45% positive)
      - "/docs/deployment/kubernetes" (52% positive)
```

---

**Document Status:** ✅ COMPLETE  
**Next Review:** Post-MVP launch feedback (Week 4)  
**Maintained By:** Only1MCP Documentation Team  
**Questions:** docs@only1mcp.dev

**Key Success Metrics:**
- **5-minute quickstart completion**: Target 80% success rate
- **Documentation-driven support reduction**: Target 40% fewer support tickets
- **Community contribution**: Target 50+ documentation PRs in first 3 months
- **Search success rate**: Target 85% finding answer without support
