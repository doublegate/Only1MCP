# Only1MCP Plugin Marketplace

Community-driven marketplace for Only1MCP plugins and integrations.

## Overview

The Only1MCP Marketplace provides:
- **Plugin Registry**: Centralized catalog of community plugins
- **Discovery**: Search and browse available plugins
- **Installation**: One-command plugin installation
- **Rating & Reviews**: Community feedback system
- **Automated Testing**: CI/CD for plugin validation

## Quick Start

### Install a Plugin

```bash
# Using CLI
only1mcp plugin install rate-limiter

# Using configuration
only1mcp plugin install --from-registry authentication-provider

# From URL
only1mcp plugin install --url https://github.com/user/plugin.git
```

### Browse Marketplace

```bash
# List all plugins
only1mcp plugin list

# Search plugins
only1mcp plugin search authentication

# Show plugin details
only1mcp plugin info rate-limiter
```

### Publish a Plugin

```bash
# Initialize plugin template
only1mcp plugin init my-plugin

# Build and test
cd my-plugin
cargo build --release
cargo test

# Publish to marketplace
only1mcp plugin publish --name my-plugin --version 1.0.0
```

## Plugin Registry

### Official Plugins

**Authentication & Security**:
- `oauth2-provider` - OAuth2 authentication
- `jwt-validator` - JWT token validation
- `api-key-auth` - API key authentication
- `mtls-auth` - Mutual TLS authentication

**Rate Limiting & Throttling**:
- `rate-limiter` - Configurable rate limiting
- `adaptive-throttle` - AI-based throttling
- `quota-manager` - Usage quota management

**Monitoring & Analytics**:
- `metrics-collector` - Custom metrics collection
- `log-aggregator` - Log aggregation and analysis
- `apm-integration` - APM tool integration (Datadog, New Relic)

**Protocol Adapters**:
- `rest-adapter` - REST to MCP conversion
- `graphql-adapter` - GraphQL to MCP conversion
- `grpc-adapter` - gRPC to MCP conversion
- `websocket-adapter` - WebSocket protocol adapter

**Load Balancing**:
- `custom-lb` - Advanced load balancing algorithms
- `geo-router` - Geographic routing
- `cost-optimizer` - Cost-based routing

**Caching & Performance**:
- `redis-cache` - Redis caching integration
- `predictive-cache` - ML-based predictive caching
- `cdn-integration` - CDN integration for static content

**Data Transformation**:
- `json-transformer` - JSON data transformation
- `xml-converter` - XML/JSON conversion
- `schema-validator` - Schema validation and transformation

### Community Plugins

Browse community plugins at: https://marketplace.only1mcp.dev

## Plugin Structure

### Directory Layout

```
my-plugin/
├── Cargo.toml              # Rust package manifest
├── src/
│   ├── lib.rs             # Plugin implementation
│   └── tests.rs           # Unit tests
├── examples/
│   └── usage.rs           # Usage examples
├── README.md              # Plugin documentation
├── LICENSE                # Plugin license
└── plugin.yaml            # Marketplace metadata
```

### Plugin Manifest (plugin.yaml)

```yaml
name: my-plugin
version: 1.0.0
description: Short description of the plugin
author: Your Name <email@example.com>
homepage: https://github.com/user/my-plugin
repository: https://github.com/user/my-plugin
license: MIT

# Plugin capabilities
capabilities:
  - request_transform
  - response_transform
  - metrics_collector

# Dependencies
dependencies:
  only1mcp: ">=0.6.0"
  tokio: "1.0"

# Configuration schema
config_schema:
  type: object
  properties:
    enabled:
      type: boolean
      default: true
    threshold:
      type: integer
      minimum: 1
      maximum: 1000

# Keywords for search
keywords:
  - authentication
  - security
  - oauth

# Compatibility
compatibility:
  min_only1mcp_version: "0.6.0"
  platforms:
    - linux
    - macos
    - windows
```

### Plugin Implementation

```rust
use only1mcp::plugins::*;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct MyPluginConfig {
    pub enabled: bool,
    pub threshold: u32,
}

pub struct MyPlugin {
    metadata: PluginMetadata,
    state: PluginState,
    config: Option<MyPluginConfig>,
}

impl MyPlugin {
    pub fn new() -> Self {
        Self {
            metadata: PluginMetadata {
                name: "my-plugin".to_string(),
                version: "1.0.0".to_string(),
                author: "Your Name".to_string(),
                description: "Plugin description".to_string(),
                capabilities: vec![PluginCapability::RequestTransform],
                dependencies: vec![],
            },
            state: PluginState::Loaded,
            config: None,
        }
    }
}

#[async_trait]
impl Plugin for MyPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    async fn initialize(&mut self, config: HashMap<String, serde_json::Value>) -> Result<(), String> {
        let plugin_config: MyPluginConfig = serde_json::from_value(
            config.get("my_plugin").cloned().unwrap_or(serde_json::json!({}))
        ).map_err(|e| format!("Invalid config: {}", e))?;
        
        self.config = Some(plugin_config);
        self.state = PluginState::Ready;
        Ok(())
    }

    async fn start(&mut self) -> Result<(), String> {
        self.state = PluginState::Running;
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), String> {
        self.state = PluginState::Stopped;
        Ok(())
    }

    fn state(&self) -> PluginState {
        self.state
    }

    async fn health_check(&self) -> Result<PluginHealth, String> {
        Ok(PluginHealth {
            healthy: self.state == PluginState::Running,
            message: "Plugin is healthy".to_string(),
        })
    }
}

#[async_trait]
impl RequestTransformer for MyPlugin {
    async fn transform_request(
        &self,
        request: McpRequest,
        context: &PluginContext,
    ) -> Result<McpRequest, String> {
        // Your transformation logic here
        Ok(request)
    }
}
```

## Marketplace API

### Registry Endpoints

```http
# List all plugins
GET /api/plugins
Response: {
  "plugins": [
    {
      "name": "rate-limiter",
      "version": "1.0.0",
      "description": "...",
      "downloads": 1500,
      "rating": 4.5
    }
  ]
}

# Get plugin details
GET /api/plugins/{name}
Response: {
  "name": "rate-limiter",
  "version": "1.0.0",
  "author": "...",
  "repository": "...",
  "readme": "...",
  "versions": ["1.0.0", "0.9.0"],
  "dependencies": {...}
}

# Download plugin
GET /api/plugins/{name}/{version}/download
Response: Binary tarball

# Submit plugin
POST /api/plugins
Body: {
  "name": "my-plugin",
  "version": "1.0.0",
  "tarball_url": "..."
}

# Rate plugin
POST /api/plugins/{name}/rate
Body: {
  "rating": 5,
  "review": "Great plugin!"
}
```

## Plugin Testing

### Automated Tests

All marketplace plugins undergo automated testing:

```yaml
# .github/workflows/plugin-test.yml
name: Plugin Test
on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      
      - name: Run tests
        run: cargo test --all-features
      
      - name: Run clippy
        run: cargo clippy -- -D warnings
      
      - name: Check formatting
        run: cargo fmt -- --check
      
      - name: Security audit
        run: cargo audit
      
      - name: Integration test
        run: |
          only1mcp plugin install --local .
          only1mcp plugin test my-plugin
```

### Manual Testing

```bash
# Install locally
only1mcp plugin install --local /path/to/plugin

# Test with Only1MCP
only1mcp start --config test-config.yaml

# Run plugin-specific tests
only1mcp plugin test my-plugin --verbose
```

## Publishing Guidelines

### Requirements

1. **Code Quality**:
   - All tests passing
   - No clippy warnings
   - Formatted with rustfmt
   - Security audit clean

2. **Documentation**:
   - Comprehensive README
   - API documentation
   - Usage examples
   - Configuration guide

3. **Licensing**:
   - Clear license (MIT, Apache-2.0, GPL-3.0)
   - Compatible with Only1MCP license

4. **Versioning**:
   - Semantic versioning (MAJOR.MINOR.PATCH)
   - Changelog maintained

### Submission Process

1. **Prepare**:
   ```bash
   cargo build --release
   cargo test
   cargo doc
   ```

2. **Package**:
   ```bash
   only1mcp plugin package
   # Creates: my-plugin-1.0.0.tar.gz
   ```

3. **Publish**:
   ```bash
   only1mcp plugin publish \
     --name my-plugin \
     --version 1.0.0 \
     --tarball my-plugin-1.0.0.tar.gz \
     --token $MARKETPLACE_TOKEN
   ```

4. **Verification**:
   - Automated tests run
   - Security scan performed
   - Documentation validated
   - Review by maintainers (for first-time publishers)

## Best Practices

### Performance

- Minimize async overhead
- Use efficient data structures
- Cache where appropriate
- Profile with `cargo flamegraph`

### Security

- Validate all inputs
- Sanitize user data
- Use constant-time comparisons for secrets
- Follow OWASP guidelines

### Compatibility

- Test on multiple platforms
- Support Only1MCP version range
- Document breaking changes
- Provide migration guides

### Documentation

- Write clear README
- Provide configuration examples
- Include troubleshooting guide
- Add inline code comments

## Marketplace Statistics

View real-time statistics:
- Total plugins: https://marketplace.only1mcp.dev/stats
- Popular plugins: https://marketplace.only1mcp.dev/popular
- New releases: https://marketplace.only1mcp.dev/recent

## Support

- GitHub Issues: https://github.com/doublegate/Only1MCP/issues
- Discord: https://discord.gg/only1mcp
- Email: marketplace@only1mcp.dev

## License

Marketplace infrastructure: GPL-3.0
Individual plugins: See plugin LICENSE file
