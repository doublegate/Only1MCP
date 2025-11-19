# Only1MCP Desktop Application (Tauri)

Native cross-platform desktop application for managing Only1MCP proxy servers.

## Overview

The Only1MCP Desktop App provides a modern, native GUI for:
- Server management and configuration
- Real-time metrics and monitoring
- Log viewing and debugging
- System tray integration
- Auto-updates

## Architecture

### Technology Stack
- **Frontend**: React + TypeScript + Tailwind CSS
- **Backend**: Rust (Tauri)
- **State Management**: Zustand
- **Charts**: Recharts / Chart.js
- **Icons**: Lucide React

### Project Structure

```
tauri-app/
├── src-tauri/          # Rust backend
│   ├── src/
│   │   ├── main.rs     # Tauri app entry point
│   │   ├── commands.rs # Tauri commands
│   │   ├── state.rs    # Application state
│   │   └── tray.rs     # System tray integration
│   ├── Cargo.toml
│   └── tauri.conf.json
├── src/                # React frontend
│   ├── components/
│   │   ├── Dashboard.tsx
│   │   ├── ServerList.tsx
│   │   ├── MetricsChart.tsx
│   │   ├── LogViewer.tsx
│   │   └── Settings.tsx
│   ├── hooks/
│   │   ├── useProxyState.ts
│   │   └── useMetrics.ts
│   ├── App.tsx
│   └── main.tsx
├── package.json
└── README.md
```

## Quick Start

### Prerequisites

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install Node.js 18+
# Download from https://nodejs.org/

# Install Tauri CLI
cargo install tauri-cli
```

### Setup

```bash
cd tauri-app

# Install dependencies
npm install

# Run in development mode
npm run tauri dev

# Build for production
npm run tauri build
```

## Features

### 1. Server Management

**Dashboard View**:
- List of all configured MCP servers
- Real-time status indicators
- Quick enable/disable toggle
- Server health metrics

**Configuration Editor**:
- Visual configuration builder
- YAML preview
- Validation and error checking
- Template library

### 2. Monitoring & Metrics

**Real-Time Metrics**:
- Request rate (req/s)
- Error rate (%)
- Average latency (ms)
- Active connections
- Cache hit rate

**Charts & Graphs**:
- Time-series line charts
- Request distribution by server
- Error types breakdown
- Performance trends

### 3. Log Viewer

**Features**:
- Real-time log streaming
- Log level filtering
- Search and highlighting
- Export logs

### 4. System Tray Integration

**Tray Menu**:
- Quick status view
- Start/Stop/Restart proxy
- Open dashboard
- Settings
- Quit application

**Notifications**:
- Error alerts
- Server down notifications
- Performance warnings

### 5. Auto-Update

**Features**:
- Automatic update checking
- Download and install updates
- Release notes display
- Rollback capability

## Tauri Commands

Backend commands exposed to frontend:

```rust
#[tauri::command]
async fn start_proxy(config_path: String) -> Result<(), String>

#[tauri::command]
async fn stop_proxy() -> Result<(), String>

#[tauri::command]
async fn get_status() -> Result<ProxyStatus, String>

#[tauri::command]
async fn get_metrics() -> Result<MetricsSnapshot, String>

#[tauri::command]
async fn get_servers() -> Result<Vec<ServerInfo>, String>

#[tauri::command]
async fn update_server(server: ServerConfig) -> Result<(), String>

#[tauri::command]
async fn get_logs(count: usize) -> Result<Vec<LogEntry>, String>

#[tauri::command]
async fn validate_config(config: String) -> Result<ValidationResult, String>
```

## Frontend Components

### Dashboard Component

```typescript
import { useProxyState } from './hooks/useProxyState';
import { MetricsChart } from './components/MetricsChart';
import { ServerList } from './components/ServerList';

export function Dashboard() {
  const { status, metrics, servers } = useProxyState();

  return (
    <div className="dashboard">
      <StatusBar status={status} />
      <MetricsChart metrics={metrics} />
      <ServerList servers={servers} />
    </div>
  );
}
```

### Server List Component

```typescript
export function ServerList({ servers }: { servers: ServerInfo[] }) {
  return (
    <div className="server-list">
      {servers.map(server => (
        <ServerCard
          key={server.id}
          server={server}
          onToggle={() => toggleServer(server.id)}
          onEdit={() => editServer(server.id)}
        />
      ))}
    </div>
  );
}
```

## Building for Distribution

### Windows

```bash
npm run tauri build -- --target x86_64-pc-windows-msvc

# Output: src-tauri/target/release/bundle/msi/Only1MCP_0.6.0_x64_en-US.msi
```

### macOS

```bash
npm run tauri build -- --target x86_64-apple-darwin
npm run tauri build -- --target aarch64-apple-darwin

# Output: src-tauri/target/release/bundle/dmg/Only1MCP_0.6.0_x64.dmg
```

### Linux

```bash
npm run tauri build -- --target x86_64-unknown-linux-gnu

# Output: src-tauri/target/release/bundle/deb/only1mcp_0.6.0_amd64.deb
#         src-tauri/target/release/bundle/appimage/only1mcp_0.6.0_amd64.AppImage
```

## Configuration

### tauri.conf.json

Key settings:

```json
{
  "build": {
    "distDir": "../dist",
    "devPath": "http://localhost:5173"
  },
  "package": {
    "productName": "Only1MCP",
    "version": "0.6.0"
  },
  "tauri": {
    "allowlist": {
      "all": false,
      "shell": {
        "all": false,
        "execute": true,
        "open": true
      },
      "fs": {
        "all": false,
        "readFile": true,
        "writeFile": true,
        "readDir": true
      },
      "dialog": {
        "all": true
      },
      "notification": {
        "all": true
      }
    },
    "bundle": {
      "active": true,
      "icon": [
        "icons/32x32.png",
        "icons/128x128.png",
        "icons/icon.icns",
        "icons/icon.ico"
      ],
      "identifier": "com.only1mcp.app"
    },
    "security": {
      "csp": "default-src 'self'; script-src 'self' 'unsafe-inline'"
    },
    "systemTray": {
      "iconPath": "icons/tray-icon.png"
    },
    "updater": {
      "active": true,
      "endpoints": [
        "https://releases.only1mcp.dev/{{target}}/{{current_version}}"
      ],
      "dialog": true,
      "pubkey": "YOUR_PUBLIC_KEY_HERE"
    },
    "windows": [
      {
        "title": "Only1MCP",
        "width": 1200,
        "height": 800,
        "minWidth": 800,
        "minHeight": 600,
        "resizable": true,
        "fullscreen": false
      }
    ]
  }
}
```

## Security

### CSP (Content Security Policy)
- Strict CSP to prevent XSS
- Only allow self-hosted resources
- No inline scripts (except necessary)

### File System Access
- Restricted to configuration directories
- User must explicitly grant permissions
- Path validation and sanitization

### Process Execution
- Only allow execution of Only1MCP binary
- Command validation
- Sandboxed execution

## Testing

```bash
# Unit tests
npm run test

# E2E tests
npm run test:e2e

# Rust tests
cd src-tauri && cargo test
```

## Deployment

### Auto-Update Setup

1. Generate signing keys:
```bash
tauri signer generate -- -w ~/.tauri/myapp.key
```

2. Configure updater in `tauri.conf.json`

3. Set up release server with JSON endpoint:
```json
{
  "version": "0.6.0",
  "notes": "New features and bug fixes",
  "pub_date": "2025-01-19T00:00:00Z",
  "platforms": {
    "darwin-x86_64": {
      "signature": "...",
      "url": "https://releases.only1mcp.dev/Only1MCP-0.6.0-x86_64.app.tar.gz"
    },
    "windows-x86_64": {
      "signature": "...",
      "url": "https://releases.only1mcp.dev/Only1MCP-0.6.0-x64-setup.exe"
    }
  }
}
```

## Development Roadmap

### Phase 1: Core Features (Current)
- [x] Basic dashboard
- [x] Server management
- [x] Real-time metrics
- [x] Log viewer

### Phase 2: Enhanced UX
- [ ] Configuration wizard
- [ ] Template library
- [ ] Drag-and-drop configuration
- [ ] Theme customization

### Phase 3: Advanced Features
- [ ] Multi-proxy management
- [ ] Remote proxy monitoring
- [ ] Alerting and notifications
- [ ] Performance profiling

### Phase 4: Enterprise Features
- [ ] SSO integration
- [ ] Audit logging
- [ ] Role-based access
- [ ] Multi-user support

## Troubleshooting

### Common Issues

**App won't start**:
- Check Rust and Node.js versions
- Ensure all dependencies are installed
- Check for port conflicts

**Build fails**:
- Update Rust toolchain: `rustup update`
- Clear build cache: `cargo clean`
- Reinstall node modules: `rm -rf node_modules && npm install`

**Auto-update not working**:
- Verify signing key configuration
- Check update endpoint URL
- Ensure internet connectivity

## Contributing

See main project [CONTRIBUTING.md](../CONTRIBUTING.md)

## License

GPL-3.0 - See [LICENSE](../LICENSE) for details
