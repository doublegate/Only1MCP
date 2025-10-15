# Only1MCP CLI Reference

Complete command-line interface reference for Only1MCP proxy server.

## Table of Contents
- [Installation](#installation)
- [Global Options](#global-options)
- [Commands](#commands)
  - [start](#start)
  - [validate](#validate)
  - [config](#config)
  - [server](#server)
  - [health](#health)
  - [tools](#tools)
  - [test](#test)
  - [version](#version)
- [Environment Variables](#environment-variables)
- [Configuration Files](#configuration-files)
- [Examples](#examples)

## Installation

### From Binary Release
```bash
# Download latest release
curl -L https://github.com/doublegate/Only1MCP/releases/latest/download/only1mcp-linux-amd64.tar.gz | tar xz
sudo mv only1mcp /usr/local/bin/

# Verify installation
only1mcp --version
```

### From Cargo
```bash
cargo install only1mcp
```

### From Source
```bash
git clone https://github.com/doublegate/Only1MCP
cd only1mcp
cargo build --release
sudo cp target/release/only1mcp /usr/local/bin/
```

## Global Options

These options are available for all commands:

```
OPTIONS:
    -h, --help                 Print help information
    -V, --version             Print version information
    -v, --verbose...          Increase logging verbosity (can be repeated)
    -q, --quiet               Suppress all output except errors
    --log-level <LEVEL>       Set log level [default: info]
                             [possible values: trace, debug, info, warn, error]
    --log-format <FORMAT>     Log output format [default: pretty]
                             [possible values: pretty, json, compact]
    --no-color               Disable colored output
```

## Commands

### start

Start the Only1MCP proxy server.

```bash
only1mcp start [OPTIONS]
```

#### Options
```
OPTIONS:
    -c, --config <FILE>           Configuration file path [default: only1mcp.yaml]
    -H, --host <HOST>            Bind host address [default: 127.0.0.1]
    -p, --port <PORT>            Bind port [default: 8080]
    --tls-cert <FILE>            TLS certificate file
    --tls-key <FILE>             TLS private key file
    --workers <N>                Number of worker threads [default: auto]
    --max-connections <N>        Maximum concurrent connections [default: 10000]
    --admin-port <PORT>          Admin API port [default: 9090]
    --admin-auth                 Enable admin API authentication
    --metrics-port <PORT>        Prometheus metrics port [default: 9091]
    --hot-reload                 Enable configuration hot-reload
    --daemon                     Run as daemon (background process)
    --pid-file <FILE>           Write PID to file
    --tui                        Enable terminal UI dashboard
```

#### Examples
```bash
# Start with default configuration
only1mcp start

# Start with specific config file
only1mcp start --config /etc/only1mcp/config.yaml

# Start with TLS enabled
only1mcp start --tls-cert cert.pem --tls-key key.pem

# Start with verbose logging
only1mcp start -vv --log-level debug

# Start as daemon with PID file
only1mcp start --daemon --pid-file /var/run/only1mcp.pid

# Start with TUI dashboard
only1mcp start --tui
```

### validate

Validate configuration file syntax and settings.

```bash
only1mcp validate [OPTIONS] <CONFIG>
```

#### Arguments
```
ARGS:
    <CONFIG>    Configuration file to validate
```

#### Options
```
OPTIONS:
    --strict              Enable strict validation (check external resources)
    --test-connections    Test connections to all configured servers
    --schema <FILE>       Custom schema file for validation
    --format <FORMAT>     Config format [default: auto]
                         [possible values: auto, yaml, toml, json]
```

#### Examples
```bash
# Validate configuration file
only1mcp validate config.yaml

# Strict validation with connection tests
only1mcp validate --strict --test-connections config.yaml

# Validate TOML configuration
only1mcp validate --format toml config.toml
```

### config

Configuration management commands.

```bash
only1mcp config <SUBCOMMAND>
```

#### Subcommands

##### generate

Generate a new configuration file from template.

```bash
only1mcp config generate [OPTIONS]
```

Options:
```
OPTIONS:
    --template <NAME>         Template to use [default: basic]
                             [possible values: basic, solo, team, enterprise]
    --format <FORMAT>        Output format [default: yaml]
                             [possible values: yaml, toml, json]
    --output <FILE>          Output file (stdout if not specified)
    --interactive            Interactive configuration wizard
```

Examples:
```bash
# Generate basic configuration
only1mcp config generate > config.yaml

# Generate enterprise template
only1mcp config generate --template enterprise > enterprise.yaml

# Interactive configuration wizard
only1mcp config generate --interactive
```

##### show

Display current configuration (with secrets redacted).

```bash
only1mcp config show [OPTIONS]
```

Options:
```
OPTIONS:
    --config <FILE>          Configuration file [default: only1mcp.yaml]
    --include-defaults       Show default values
    --format <FORMAT>        Output format [default: yaml]
    --section <SECTION>      Show only specific section
```

##### migrate

Migrate configuration from older version.

```bash
only1mcp config migrate <OLD_CONFIG> <NEW_CONFIG>
```

Options:
```
OPTIONS:
    --from-version <VER>     Source configuration version
    --to-version <VER>       Target configuration version [default: latest]
    --backup                 Create backup of old config
```

### server

Manage MCP backend servers.

```bash
only1mcp server <SUBCOMMAND>
```

#### Subcommands

##### list

List configured MCP servers.

```bash
only1mcp server list [OPTIONS]
```

Options:
```
OPTIONS:
    --config <FILE>          Configuration file
    --format <FORMAT>        Output format [default: table]
                            [possible values: table, json, csv]
    --filter <FILTER>        Filter expression (e.g., "status=healthy")
    --sort <FIELD>          Sort by field [default: id]
```

##### add

Add a new MCP server.

```bash
only1mcp server add [OPTIONS] <ID> <URL>
```

Options:
```
OPTIONS:
    --name <NAME>            Server display name
    --transport <TYPE>       Transport type [default: http]
                            [possible values: stdio, http, sse, websocket]
    --auth-type <TYPE>      Authentication type
    --health-check          Enable health checking
    --weight <N>            Server weight for load balancing
```

##### remove

Remove an MCP server.

```bash
only1mcp server remove <ID>
```

##### enable/disable

Enable or disable an MCP server.

```bash
only1mcp server enable <ID>
only1mcp server disable <ID>
```

### health

Health check and status commands.

```bash
only1mcp health <SUBCOMMAND>
```

#### Subcommands

##### check

Perform health check on proxy or specific server.

```bash
only1mcp health check [OPTIONS] [SERVER_ID]
```

Options:
```
OPTIONS:
    --config <FILE>          Configuration file
    --timeout <SECONDS>      Health check timeout [default: 5]
    --retries <N>           Number of retries [default: 3]
    --all                   Check all servers
```

##### status

Show proxy and server status.

```bash
only1mcp health status [OPTIONS]
```

Options:
```
OPTIONS:
    --config <FILE>          Configuration file
    --format <FORMAT>        Output format [default: table]
    --watch                 Continuously update (like top)
    --interval <SECONDS>     Update interval for watch mode [default: 2]
```

### tools

MCP tools management.

```bash
only1mcp tools <SUBCOMMAND>
```

#### Subcommands

##### list

List available MCP tools.

```bash
only1mcp tools list [OPTIONS]
```

Options:
```
OPTIONS:
    --server <ID>           List tools from specific server
    --all                   List from all servers
    --format <FORMAT>       Output format [default: table]
    --filter <FILTER>       Filter tools by name/description
```

##### call

Call an MCP tool directly.

```bash
only1mcp tools call [OPTIONS] <TOOL> [ARGS]
```

Options:
```
OPTIONS:
    --server <ID>           Target specific server
    --input <FILE>          Read input from file
    --output <FILE>         Write output to file
    --timeout <SECONDS>     Request timeout
```

Examples:
```bash
# List all tools
only1mcp tools list --all

# Call a specific tool
only1mcp tools call fs_read --server filesystem '{"path": "/etc/hosts"}'

# Call tool with input from file
only1mcp tools call analyze --input data.json
```

### test

Run diagnostic tests.

```bash
only1mcp test [OPTIONS]
```

Options:
```
OPTIONS:
    --config <FILE>          Configuration file
    --suite <SUITE>          Test suite to run [default: all]
                            [possible values: all, connectivity, performance, security]
    --server <ID>           Test specific server
    --output <FILE>         Save test report to file
    --benchmark             Run performance benchmarks
    --load-test             Run load testing
    --concurrent <N>         Number of concurrent test clients [default: 10]
    --duration <SECONDS>     Test duration for load tests [default: 60]
```

Examples:
```bash
# Run all diagnostic tests
only1mcp test

# Run connectivity tests only
only1mcp test --suite connectivity

# Run load test with 100 concurrent clients
only1mcp test --load-test --concurrent 100 --duration 300

# Benchmark specific server
only1mcp test --benchmark --server github
```

### version

Display version information.

```bash
only1mcp version [OPTIONS]
```

Options:
```
OPTIONS:
    --json                  Output as JSON
    --build-info           Include build information
```

## Environment Variables

Only1MCP respects the following environment variables:

| Variable | Description | Default |
|----------|-------------|---------|
| `ONLY1MCP_CONFIG` | Configuration file path | `only1mcp.yaml` |
| `ONLY1MCP_HOST` | Bind host address | `127.0.0.1` |
| `ONLY1MCP_PORT` | Bind port | `8080` |
| `ONLY1MCP_LOG_LEVEL` | Log level | `info` |
| `ONLY1MCP_LOG_FORMAT` | Log format | `pretty` |
| `ONLY1MCP_DATA_DIR` | Data directory | `~/.only1mcp` |
| `ONLY1MCP_CACHE_DIR` | Cache directory | `~/.only1mcp/cache` |
| `ONLY1MCP_NO_COLOR` | Disable colored output | `false` |
| `RUST_LOG` | Rust log configuration | - |
| `RUST_BACKTRACE` | Enable backtraces | `0` |

### Authentication Variables
| Variable | Description |
|----------|-------------|
| `ONLY1MCP_API_KEY` | Default API key for authentication |
| `ONLY1MCP_OAUTH_CLIENT_ID` | OAuth client ID |
| `ONLY1MCP_OAUTH_CLIENT_SECRET` | OAuth client secret |
| `ONLY1MCP_JWT_SECRET` | JWT signing secret |

### Server-Specific Variables
| Variable | Description |
|----------|-------------|
| `GITHUB_TOKEN` | GitHub API token |
| `OPENAI_API_KEY` | OpenAI API key |
| `ANTHROPIC_API_KEY` | Anthropic API key |

## Configuration Files

Only1MCP looks for configuration files in the following order:

1. Command-line specified (`--config`)
2. Current directory (`./only1mcp.yaml`, `./only1mcp.toml`)
3. User config directory (`~/.config/only1mcp/config.yaml`)
4. System config directory (`/etc/only1mcp/config.yaml`)

### Configuration Formats

Only1MCP supports multiple configuration formats:
- YAML (`.yaml`, `.yml`)
- TOML (`.toml`)
- JSON (`.json`)

Format is auto-detected by file extension.

## Examples

### Basic Usage

```bash
# Start proxy with default settings
only1mcp start

# Start with custom configuration
only1mcp start --config my-config.yaml

# Validate configuration before starting
only1mcp validate my-config.yaml && only1mcp start --config my-config.yaml
```

### Production Deployment

```bash
# Generate enterprise configuration
only1mcp config generate --template enterprise > /etc/only1mcp/config.yaml

# Validate with connection tests
only1mcp validate --strict --test-connections /etc/only1mcp/config.yaml

# Start as daemon with monitoring
only1mcp start \
    --config /etc/only1mcp/config.yaml \
    --daemon \
    --pid-file /var/run/only1mcp.pid \
    --log-level info \
    --log-format json \
    --hot-reload
```

### Development Setup

```bash
# Generate solo developer config
only1mcp config generate --template solo > dev-config.yaml

# Start with debug logging and TUI
only1mcp start \
    --config dev-config.yaml \
    --log-level debug \
    --tui

# In another terminal, monitor health
watch only1mcp health status --config dev-config.yaml
```

### Testing and Debugging

```bash
# Run diagnostic tests
only1mcp test --config production.yaml

# Check specific server health
only1mcp health check github --config production.yaml

# List all available tools
only1mcp tools list --all --format json | jq '.'

# Run load test
only1mcp test \
    --load-test \
    --concurrent 50 \
    --duration 120 \
    --output load-test-report.json
```

### Container Usage

```bash
# Run in Docker
docker run -d \
    -p 8080:8080 \
    -v $(pwd)/config.yaml:/config.yaml \
    -e ONLY1MCP_CONFIG=/config.yaml \
    only1mcp/only1mcp:latest

# Run in Kubernetes
kubectl apply -f only1mcp-deployment.yaml
kubectl port-forward svc/only1mcp 8080:8080
```

### Systemd Service

```ini
# /etc/systemd/system/only1mcp.service
[Unit]
Description=Only1MCP Proxy Server
After=network.target

[Service]
Type=simple
User=only1mcp
Group=only1mcp
ExecStart=/usr/local/bin/only1mcp start --config /etc/only1mcp/config.yaml
ExecReload=/bin/kill -HUP $MAINPID
Restart=on-failure
RestartSec=5
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
```

```bash
# Enable and start service
sudo systemctl enable only1mcp
sudo systemctl start only1mcp
sudo systemctl status only1mcp
```

## Exit Codes

| Code | Description |
|------|-------------|
| 0 | Success |
| 1 | General error |
| 2 | Configuration error |
| 3 | Connection error |
| 4 | Authentication error |
| 5 | Permission denied |
| 10 | Invalid arguments |
| 11 | File not found |
| 12 | Invalid configuration |
| 20 | Server error |
| 21 | Backend unavailable |
| 22 | Timeout |
| 127 | Command not found |

## See Also

- [Configuration Guide](CONFIGURATION_GUIDE.md)
- [Deployment Guide](DEPLOYMENT_GUIDE.md)
- [API Reference](API_REFERENCE.md)
- [Troubleshooting Guide](TROUBLESHOOTING.md)