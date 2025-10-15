# Only1MCP Deployment & Operations Strategy
## Production-Ready Deployment, Monitoring, and Incident Management

**Document Version:** 1.0  
**Target Environments:** Development, Staging, Production  
**Deployment Platforms:** Linux, macOS, Windows, Docker/Podman (optional)  
**Date:** October 14, 2025  
**Status:** Operational Strategy Specification

---

## TABLE OF CONTENTS

1. [Deployment Philosophy](#deployment-philosophy)
2. [CI/CD Pipeline Architecture](#cicd-pipeline-architecture)
3. [Release Management](#release-management)
4. [Environment Configuration](#environment-configuration)
5. [Monitoring & Alerting](#monitoring--alerting)
6. [Incident Response](#incident-response)
7. [Backup & Recovery](#backup--recovery)
8. [Scaling Strategy](#scaling-strategy)
9. [Security Operations](#security-operations)
10. [Runbooks & Procedures](#runbooks--procedures)

---

## DEPLOYMENT PHILOSOPHY

### Core Principles

**1. Zero-Downtime Deployments**
- Blue-green deployment pattern for production
- Rolling updates with health checks
- Automatic rollback on failure detection
- Connection draining (30s grace period)

**2. Immutable Infrastructure**
- Each release produces new binaries
- No in-place modifications
- Clear versioning and traceability
- Easy rollback to previous versions

**3. Environment Parity**
- Development mirrors production architecture
- Staging is production-identical
- Infrastructure as Code (IaC) for consistency
- Automated environment provisioning

**4. Progressive Delivery**
- Alpha → Beta → Release Candidate → GA
- Feature flags for gradual rollout
- Canary deployments (5% → 25% → 100%)
- Real-time monitoring during rollout

---

## CI/CD PIPELINE ARCHITECTURE

### Pipeline Stages

```yaml
# .github/workflows/ci-cd.yml
name: Only1MCP CI/CD Pipeline

on:
  push:
    branches: [main, develop]
    tags: ['v*']
  pull_request:
    branches: [main]

jobs:
  # Stage 1: Code Quality
  quality:
    name: Code Quality Checks
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy
      
      - name: Cache dependencies
        uses: Swatinem/rust-cache@v2
      
      - name: Check formatting
        run: cargo fmt --all -- --check
      
      - name: Clippy lints
        run: cargo clippy --all-targets --all-features -- -D warnings
      
      - name: Check documentation
        run: cargo doc --no-deps --document-private-items
        env:
          RUSTDOCFLAGS: -D warnings

  # Stage 2: Build & Test
  build-test:
    name: Build and Test
    needs: quality
    strategy:
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]
        rust: [stable, nightly]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      
      - name: Install Rust
        uses: dtolnay/rust-toolchain@master
        with:
          toolchain: ${{ matrix.rust }}
      
      - uses: Swatinem/rust-cache@v2
      
      - name: Build
        run: cargo build --release --verbose
      
      - name: Run unit tests
        run: cargo test --lib --release
      
      - name: Run integration tests
        run: cargo test --test '*' --release
      
      - name: Upload artifacts
        if: matrix.os == 'ubuntu-latest' && matrix.rust == 'stable'
        uses: actions/upload-artifact@v4
        with:
          name: only1mcp-linux-x86_64
          path: target/release/only1mcp

  # Stage 3: Security Scanning
  security:
    name: Security Audit
    needs: build-test
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Dependency audit
        uses: actions-rust-lang/audit@v1
      
      - name: Vulnerability scan
        run: |
          cargo install cargo-audit
          cargo audit --deny warnings
      
      - name: License compliance
        run: |
          cargo install cargo-license
          cargo license --json > licenses.json
          # Verify no GPL/AGPL licenses
          ! grep -E '"GPL|AGPL"' licenses.json

  # Stage 4: Performance Benchmarks
  benchmark:
    name: Performance Benchmarks
    needs: build-test
    runs-on: ubuntu-latest
    if: github.event_name == 'pull_request'
    steps:
      - uses: actions/checkout@v4
      
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      
      - name: Run benchmarks
        run: cargo bench --bench '*' -- --save-baseline pr-${{ github.event.number }}
      
      - name: Compare with main
        run: |
          git fetch origin main:main
          git checkout main
          cargo bench --bench '*' -- --save-baseline main
          git checkout -
          cargo bench --bench '*' -- --baseline main --load-baseline pr-${{ github.event.number }}
      
      - name: Upload benchmark results
        uses: actions/upload-artifact@v4
        with:
          name: benchmark-results
          path: target/criterion/

  # Stage 5: Build Release Artifacts
  release-build:
    name: Build Release Artifacts
    needs: [quality, build-test, security]
    if: startsWith(github.ref, 'refs/tags/v')
    strategy:
      matrix:
        include:
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
            artifact: only1mcp-linux-amd64
          - os: ubuntu-latest
            target: x86_64-unknown-linux-musl
            artifact: only1mcp-linux-amd64-static
          - os: ubuntu-latest
            target: aarch64-unknown-linux-gnu
            artifact: only1mcp-linux-arm64
          - os: windows-latest
            target: x86_64-pc-windows-msvc
            artifact: only1mcp-windows-amd64.exe
          - os: macos-latest
            target: x86_64-apple-darwin
            artifact: only1mcp-darwin-amd64
          - os: macos-latest
            target: aarch64-apple-darwin
            artifact: only1mcp-darwin-arm64
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      
      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}
      
      - name: Install cross-compilation tools
        if: matrix.target == 'aarch64-unknown-linux-gnu'
        run: |
          sudo apt-get update
          sudo apt-get install -y gcc-aarch64-linux-gnu
      
      - name: Build release binary
        run: cargo build --release --target ${{ matrix.target }}
      
      - name: Strip binary (Unix)
        if: matrix.os != 'windows-latest'
        run: strip target/${{ matrix.target }}/release/only1mcp
      
      - name: Create archive
        run: |
          cd target/${{ matrix.target }}/release
          tar czf ${{ matrix.artifact }}.tar.gz only1mcp${{ matrix.os == 'windows-latest' && '.exe' || '' }}
      
      - name: Upload release artifact
        uses: actions/upload-artifact@v4
        with:
          name: ${{ matrix.artifact }}
          path: target/${{ matrix.target }}/release/${{ matrix.artifact }}.tar.gz

  # Stage 6: Docker Image Build
  docker-build:
    name: Build Docker Images
    needs: [quality, build-test, security]
    if: github.event_name == 'push' && (github.ref == 'refs/heads/main' || startsWith(github.ref, 'refs/tags/v'))
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Set up Docker Buildx
        uses: docker/setup-buildx-action@v3
      
      - name: Login to GitHub Container Registry
        uses: docker/login-action@v3
        with:
          registry: ghcr.io
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}
      
      - name: Extract metadata
        id: meta
        uses: docker/metadata-action@v5
        with:
          images: ghcr.io/${{ github.repository }}
          tags: |
            type=ref,event=branch
            type=semver,pattern={{version}}
            type=semver,pattern={{major}}.{{minor}}
            type=sha
      
      - name: Build and push
        uses: docker/build-push-action@v5
        with:
          context: .
          push: true
          tags: ${{ steps.meta.outputs.tags }}
          labels: ${{ steps.meta.outputs.labels }}
          cache-from: type=gha
          cache-to: type=gha,mode=max
          platforms: linux/amd64,linux/arm64

  # Stage 7: Create GitHub Release
  github-release:
    name: Create GitHub Release
    needs: [release-build, docker-build]
    if: startsWith(github.ref, 'refs/tags/v')
    runs-on: ubuntu-latest
    permissions:
      contents: write
    steps:
      - uses: actions/checkout@v4
      
      - name: Download all artifacts
        uses: actions/download-artifact@v4
        with:
          path: artifacts/
      
      - name: Generate checksums
        run: |
          cd artifacts
          find . -type f -name "*.tar.gz" -exec sha256sum {} \; > SHA256SUMS
      
      - name: Create Release
        uses: softprops/action-gh-release@v1
        with:
          files: artifacts/**/*
          generate_release_notes: true
          draft: false
          prerelease: ${{ contains(github.ref, '-alpha') || contains(github.ref, '-beta') || contains(github.ref, '-rc') }}
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}

  # Stage 8: Deploy to Staging
  deploy-staging:
    name: Deploy to Staging
    needs: github-release
    if: startsWith(github.ref, 'refs/tags/v') && !contains(github.ref, '-alpha')
    runs-on: ubuntu-latest
    environment:
      name: staging
      url: https://staging.only1mcp.dev
    steps:
      - name: Deploy via SSH
        uses: appleboy/ssh-action@v1.0.0
        with:
          host: ${{ secrets.STAGING_HOST }}
          username: ${{ secrets.STAGING_USER }}
          key: ${{ secrets.STAGING_SSH_KEY }}
          script: |
            cd /opt/only1mcp
            ./scripts/deploy.sh ${{ github.ref_name }} staging
      
      - name: Health check
        run: |
          sleep 10
          curl -f https://staging.only1mcp.dev/health || exit 1
      
      - name: Run smoke tests
        run: |
          curl -f https://staging.only1mcp.dev/api/v1/admin/servers || exit 1

  # Stage 9: Deploy to Production
  deploy-production:
    name: Deploy to Production
    needs: deploy-staging
    if: startsWith(github.ref, 'refs/tags/v') && !contains(github.ref, '-alpha') && !contains(github.ref, '-beta') && !contains(github.ref, '-rc')
    runs-on: ubuntu-latest
    environment:
      name: production
      url: https://only1mcp.dev
    steps:
      - name: Create deployment issue
        uses: actions/github-script@v7
        with:
          script: |
            await github.rest.issues.create({
              owner: context.repo.owner,
              repo: context.repo.repo,
              title: `Production Deployment: ${context.ref}`,
              body: `Deploying version ${context.ref} to production.\n\nMonitor: https://grafana.only1mcp.dev`,
              labels: ['deployment', 'production']
            });
      
      - name: Blue-Green Deployment
        uses: appleboy/ssh-action@v1.0.0
        with:
          host: ${{ secrets.PROD_HOST }}
          username: ${{ secrets.PROD_USER }}
          key: ${{ secrets.PROD_SSH_KEY }}
          script: |
            cd /opt/only1mcp
            ./scripts/blue-green-deploy.sh ${{ github.ref_name }}
      
      - name: Health check
        run: |
          for i in {1..30}; do
            if curl -f https://only1mcp.dev/health; then
              echo "Health check passed"
              exit 0
            fi
            echo "Attempt $i failed, retrying..."
            sleep 10
          done
          exit 1
      
      - name: Notify Slack
        uses: slackapi/slack-github-action@v1.25.0
        with:
          webhook-url: ${{ secrets.SLACK_WEBHOOK }}
          payload: |
            {
              "text": "✅ Only1MCP ${{ github.ref_name }} deployed to production",
              "blocks": [
                {
                  "type": "section",
                  "text": {
                    "type": "mrkdwn",
                    "text": "*Deployment Successful*\nVersion: ${{ github.ref_name }}\nEnvironment: Production\nStatus: https://status.only1mcp.dev"
                  }
                }
              ]
            }
```

### Deployment Script

```bash
#!/bin/bash
# scripts/deploy.sh - Production deployment script

set -euo pipefail

VERSION=$1
ENVIRONMENT=${2:-production}

echo "🚀 Deploying Only1MCP $VERSION to $ENVIRONMENT"

# 1. Download release binary
echo "📦 Downloading release artifacts..."
curl -fsSL "https://github.com/doublegate/Only1MCP/releases/download/$VERSION/only1mcp-linux-amd64.tar.gz" -o only1mcp.tar.gz
tar xzf only1mcp.tar.gz
chmod +x only1mcp

# 2. Verify binary integrity
echo "🔐 Verifying checksums..."
curl -fsSL "https://github.com/doublegate/Only1MCP/releases/download/$VERSION/SHA256SUMS" | sha256sum -c -

# 3. Stop old process (gracefully)
echo "🛑 Stopping old process..."
if systemctl is-active --quiet only1mcp; then
    systemctl stop only1mcp
    sleep 5  # Allow graceful shutdown
fi

# 4. Backup current binary
echo "💾 Backing up current binary..."
if [ -f /usr/local/bin/only1mcp ]; then
    cp /usr/local/bin/only1mcp /usr/local/bin/only1mcp.backup
fi

# 5. Install new binary
echo "📥 Installing new binary..."
mv only1mcp /usr/local/bin/only1mcp

# 6. Validate configuration
echo "✅ Validating configuration..."
/usr/local/bin/only1mcp validate --config /etc/only1mcp/config.yaml

# 7. Start new process
echo "▶️  Starting new process..."
systemctl start only1mcp

# 8. Health check
echo "🏥 Running health checks..."
for i in {1..30}; do
    if curl -f http://localhost:8080/health; then
        echo "✅ Deployment successful!"
        exit 0
    fi
    echo "Attempt $i failed, retrying..."
    sleep 2
done

# Rollback on failure
echo "❌ Health checks failed - rolling back..."
systemctl stop only1mcp
mv /usr/local/bin/only1mcp.backup /usr/local/bin/only1mcp
systemctl start only1mcp
exit 1
```

### Blue-Green Deployment

```bash
#!/bin/bash
# scripts/blue-green-deploy.sh - Zero-downtime deployment

VERSION=$1
CURRENT_COLOR=$(cat /etc/only1mcp/active-color)  # "blue" or "green"
NEW_COLOR=$([[ "$CURRENT_COLOR" == "blue" ]] && echo "green" || echo "blue")

echo "🎨 Blue-Green Deployment: $CURRENT_COLOR → $NEW_COLOR"

# 1. Deploy to inactive environment
echo "📦 Deploying to $NEW_COLOR environment..."
ssh ${NEW_COLOR}@only1mcp-internal "cd /opt/only1mcp && ./deploy.sh $VERSION production"

# 2. Run smoke tests
echo "🧪 Running smoke tests on $NEW_COLOR..."
for endpoint in /health /api/v1/admin/servers; do
    curl -f "http://${NEW_COLOR}.only1mcp.internal:8080$endpoint" || {
        echo "❌ Smoke test failed on $endpoint"
        exit 1
    }
done

# 3. Gradually shift traffic (5% → 25% → 50% → 100%)
echo "🔄 Shifting traffic to $NEW_COLOR..."
for weight in 5 25 50 100; do
    echo "  Setting $NEW_COLOR weight to $weight%..."
    curl -X POST "http://loadbalancer.only1mcp.internal/api/v1/backends/${NEW_COLOR}/weight" \
         -d "{\"weight\": $weight}" \
         -H "Content-Type: application/json"
    
    sleep 30  # Monitor for 30 seconds
    
    # Check error rates
    ERROR_RATE=$(curl -s "http://prometheus.only1mcp.internal/api/v1/query?query=rate(only1mcp_errors_total[1m])" | jq -r '.data.result[0].value[1]')
    if (( $(echo "$ERROR_RATE > 0.01" | bc -l) )); then
        echo "❌ Error rate too high ($ERROR_RATE), rolling back..."
        curl -X POST "http://loadbalancer.only1mcp.internal/api/v1/backends/${NEW_COLOR}/weight" -d '{"weight": 0}'
        exit 1
    fi
done

# 4. Mark new environment as active
echo "$NEW_COLOR" > /etc/only1mcp/active-color

# 5. Drain old environment
echo "🚰 Draining $CURRENT_COLOR environment..."
curl -X POST "http://loadbalancer.only1mcp.internal/api/v1/backends/${CURRENT_COLOR}/weight" -d '{"weight": 0}'
sleep 30  # Allow in-flight requests to complete

# 6. Stop old environment
echo "🛑 Stopping $CURRENT_COLOR environment..."
ssh ${CURRENT_COLOR}@only1mcp-internal "systemctl stop only1mcp"

echo "✅ Blue-Green deployment complete! Active: $NEW_COLOR"
```

---

## RELEASE MANAGEMENT

### Versioning Strategy (SemVer)

**Format:** `MAJOR.MINOR.PATCH[-PRERELEASE][+BUILD]`

**Examples:**
- `1.0.0` - Initial GA release
- `1.1.0` - New features (backward compatible)
- `1.1.1` - Bug fixes
- `2.0.0` - Breaking changes
- `1.2.0-beta.1` - Pre-release
- `1.2.0+build.123` - Build metadata

**Version Bumping Rules:**
- **MAJOR**: Breaking API changes, incompatible config format
- **MINOR**: New features, new server discovery protocols
- **PATCH**: Bug fixes, security patches, performance improvements

### Release Cadence

- **Major releases**: Every 6-12 months
- **Minor releases**: Every 4-6 weeks
- **Patch releases**: As needed (critical bugs/security)
- **Pre-releases**: Weekly during development phases

### Changelog Management

```markdown
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.2.0] - 2025-10-14

### Added
- Context optimization with dynamic tool loading (60% token reduction)
- Plugin system for custom routers and integrations
- Multi-environment profile support
- CLI commands: `only1mcp test`, `only1mcp benchmark`

### Changed
- Improved error messages with troubleshooting hints
- Updated Axum to 0.7.5 (security fixes)
- Consistent hashing now uses 200 virtual nodes (was 150)

### Fixed
- Hot-reload race condition when adding/removing servers quickly
- Memory leak in STDIO transport after 1000+ requests
- CORS headers not sent for preflight requests

### Security
- CVE-2025-12345: Fixed command injection in STDIO server spawning
- Updated Rustls to 0.23 (TLS 1.3 security improvements)

### Deprecated
- SSE transport (use Streamable HTTP instead)
- Legacy config format v0.x (migrate to v1.0 schema)

## [1.1.0] - 2025-09-15
...
```

### Release Checklist

**Pre-Release (T-7 days):**
- [ ] All tests passing on CI
- [ ] Security audit completed (`cargo audit`)
- [ ] Performance benchmarks reviewed (no regressions >5%)
- [ ] Documentation updated (README, API docs, migration guides)
- [ ] CHANGELOG.md updated with all changes
- [ ] Version bumped in Cargo.toml
- [ ] Translation updates (if i18n enabled)

**Release Day (T-0):**
- [ ] Create Git tag: `git tag -a v1.2.0 -m "Release 1.2.0"`
- [ ] Push tag: `git push origin v1.2.0`
- [ ] CI builds and publishes artifacts automatically
- [ ] GitHub Release created with release notes
- [ ] Docker images published to ghcr.io
- [ ] Publish to crates.io: `cargo publish`
- [ ] Deploy to staging environment
- [ ] Run smoke tests on staging

**Post-Release (T+1):**
- [ ] Deploy to production (if GA release)
- [ ] Monitor error rates and performance metrics for 24 hours
- [ ] Announce on GitHub Discussions, Reddit (r/rust, r/mcp), Twitter
- [ ] Update documentation website
- [ ] Close milestone in GitHub
- [ ] Review and triage new issues

**Post-Release (T+7):**
- [ ] Retrospective: What went well? What to improve?
- [ ] Update release process documentation
- [ ] Plan next release

---

## ENVIRONMENT CONFIGURATION

### Environment Tiers

| Environment | Purpose | Deployment Trigger | Data | Monitoring |
|-------------|---------|-------------------|------|------------|
| **Development** | Local dev | Manual | Mock/synthetic | Minimal |
| **Staging** | Pre-prod testing | Tag v*-beta, v*-rc | Anonymized prod copy | Full |
| **Production** | Live users | Tag v* (GA only) | Real | Full + alerting |

### Configuration Management

**Environment Variables:**
```bash
# Production environment variables
ONLY1MCP_ENV=production
ONLY1MCP_LOG_LEVEL=info
ONLY1MCP_CONFIG_PATH=/etc/only1mcp/config.yaml
ONLY1MCP_TLS_CERT=/etc/only1mcp/certs/fullchain.pem
ONLY1MCP_TLS_KEY=/etc/only1mcp/certs/privkey.pem
ONLY1MCP_JWT_SECRET=$(cat /run/secrets/jwt_secret)
ONLY1MCP_METRICS_PORT=9090
ONLY1MCP_ADMIN_API_KEY=$(cat /run/secrets/admin_api_key)
```

**Secrets Management:**
- **Development**: `.env` file (gitignored)
- **Staging/Production**: HashiCorp Vault, AWS Secrets Manager, or systemd credentials
- **Never** commit secrets to Git
- Rotate secrets quarterly

**Configuration File (YAML):**
```yaml
# /etc/only1mcp/config.production.yaml
version: "1.0"

proxy:
  host: "0.0.0.0"
  port: 8080
  tls:
    enabled: true
    cert: "/etc/only1mcp/certs/fullchain.pem"
    key: "/etc/only1mcp/certs/privkey.pem"
  
  # Context optimization
  cache:
    enabled: true
    ttl_seconds: 300
    max_size_mb: 1024
  
  # Load balancing
  load_balancer:
    algorithm: "consistent_hash"
    virtual_nodes: 200
  
  # Health checks
  health:
    interval_seconds: 10
    timeout_seconds: 5
    failure_threshold: 3
    success_threshold: 2

auth:
  jwt:
    secret_env: "ONLY1MCP_JWT_SECRET"
    expiry_hours: 24
  
  rate_limit:
    requests_per_minute: 60
    burst: 10

logging:
  level: "info"
  format: "json"
  outputs:
    - type: "stdout"
    - type: "file"
      path: "/var/log/only1mcp/only1mcp.log"
      max_size_mb: 100
      max_backups: 7

metrics:
  enabled: true
  port: 9090
  path: "/metrics"

servers:
  - id: "filesystem-prod"
    name: "Production Filesystem"
    transport: "stdio"
    command: "npx"
    args: ["@modelcontextprotocol/server-filesystem", "/data"]
    health_check_url: "internal://ping"
  
  - id: "web-search-prod"
    name: "Production Web Search"
    transport: "http"
    url: "https://search.internal.only1mcp.dev/mcp"
    auth:
      type: "bearer"
      token_env: "SEARCH_API_KEY"
    timeout_ms: 5000
    retries: 3
```

---

## MONITORING & ALERTING

### Metrics Collection (Prometheus)

**Key Metrics:**
```rust
// Instrumented in src/metrics.rs
use prometheus::{Counter, Histogram, IntGauge, Registry};

lazy_static! {
    // Request metrics
    pub static ref REQUESTS_TOTAL: Counter = 
        register_counter!("only1mcp_requests_total", "Total requests").unwrap();
    
    pub static ref REQUEST_DURATION: Histogram =
        register_histogram!(
            "only1mcp_request_duration_seconds",
            "Request duration",
            vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0]
        ).unwrap();
    
    // Backend metrics
    pub static ref BACKEND_REQUESTS: Counter =
        register_counter!("only1mcp_backend_requests_total", "Backend requests by server").unwrap();
    
    pub static ref BACKEND_ERRORS: Counter =
        register_counter!("only1mcp_backend_errors_total", "Backend errors").unwrap();
    
    pub static ref BACKEND_DURATION: Histogram =
        register_histogram!("only1mcp_backend_duration_seconds", "Backend response time").unwrap();
    
    // Cache metrics
    pub static ref CACHE_HITS: Counter =
        register_counter!("only1mcp_cache_hits_total", "Cache hits").unwrap();
    
    pub static ref CACHE_MISSES: Counter =
        register_counter!("only1mcp_cache_misses_total", "Cache misses").unwrap();
    
    pub static ref CACHE_SIZE: IntGauge =
        register_int_gauge!("only1mcp_cache_entries", "Number of cached entries").unwrap();
    
    // System metrics
    pub static ref CONNECTIONS_ACTIVE: IntGauge =
        register_int_gauge!("only1mcp_connections_active", "Active connections").unwrap();
    
    pub static ref MEMORY_USAGE: IntGauge =
        register_int_gauge!("only1mcp_memory_bytes", "Memory usage in bytes").unwrap();
}
```

### Grafana Dashboards

**Dashboard 1: Overview**
```json
{
  "dashboard": {
    "title": "Only1MCP Overview",
    "panels": [
      {
        "title": "Requests per Second",
        "targets": [{
          "expr": "rate(only1mcp_requests_total[5m])"
        }]
      },
      {
        "title": "Response Time (p50, p95, p99)",
        "targets": [
          {"expr": "histogram_quantile(0.50, rate(only1mcp_request_duration_seconds_bucket[5m]))"},
          {"expr": "histogram_quantile(0.95, rate(only1mcp_request_duration_seconds_bucket[5m]))"},
          {"expr": "histogram_quantile(0.99, rate(only1mcp_request_duration_seconds_bucket[5m]))"}
        ]
      },
      {
        "title": "Error Rate",
        "targets": [{
          "expr": "rate(only1mcp_backend_errors_total[5m]) / rate(only1mcp_requests_total[5m])"
        }]
      },
      {
        "title": "Cache Hit Rate",
        "targets": [{
          "expr": "rate(only1mcp_cache_hits_total[5m]) / (rate(only1mcp_cache_hits_total[5m]) + rate(only1mcp_cache_misses_total[5m]))"
        }]
      }
    ]
  }
}
```

### Alert Rules (Prometheus)

```yaml
# /etc/prometheus/rules/only1mcp.yml
groups:
  - name: only1mcp_alerts
    interval: 30s
    rules:
      # High error rate
      - alert: HighErrorRate
        expr: rate(only1mcp_backend_errors_total[5m]) / rate(only1mcp_requests_total[5m]) > 0.05
        for: 2m
        labels:
          severity: critical
        annotations:
          summary: "Only1MCP error rate above 5%"
          description: "Error rate is {{ $value | humanizePercentage }} (threshold: 5%)"
      
      # High latency
      - alert: HighLatency
        expr: histogram_quantile(0.99, rate(only1mcp_request_duration_seconds_bucket[5m])) > 0.1
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Only1MCP p99 latency above 100ms"
          description: "p99 latency is {{ $value }}s (threshold: 0.1s)"
      
      # Backend unhealthy
      - alert: BackendUnhealthy
        expr: up{job="only1mcp-backends"} == 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "Backend {{ $labels.instance }} is down"
          description: "Backend has been unreachable for 1 minute"
      
      # Low cache hit rate
      - alert: LowCacheHitRate
        expr: rate(only1mcp_cache_hits_total[10m]) / (rate(only1mcp_cache_hits_total[10m]) + rate(only1mcp_cache_misses_total[10m])) < 0.5
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "Cache hit rate below 50%"
          description: "Cache efficiency is low ({{ $value | humanizePercentage }}), consider tuning TTL"
      
      # Memory usage high
      - alert: HighMemoryUsage
        expr: only1mcp_memory_bytes > 500000000  # 500MB
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Memory usage above 500MB"
          description: "Current usage: {{ $value | humanize1024 }}B"
```

### Alerting Channels

```yaml
# /etc/alertmanager/config.yml
route:
  group_by: ['alertname', 'severity']
  group_wait: 30s
  group_interval: 5m
  repeat_interval: 4h
  receiver: 'team-channel'
  
  routes:
    - match:
        severity: critical
      receiver: 'pagerduty'
      continue: true
    
    - match:
        severity: warning
      receiver: 'slack-warnings'

receivers:
  - name: 'team-channel'
    slack_configs:
      - api_url: 'https://hooks.slack.com/services/...'
        channel: '#only1mcp-alerts'
        title: '{{ .GroupLabels.alertname }}'
        text: '{{ range .Alerts }}{{ .Annotations.description }}{{ end }}'
  
  - name: 'pagerduty'
    pagerduty_configs:
      - service_key: '<pagerduty_key>'
        description: '{{ .GroupLabels.alertname }}: {{ .Annotations.summary }}'
  
  - name: 'slack-warnings'
    slack_configs:
      - api_url: 'https://hooks.slack.com/services/...'
        channel: '#only1mcp-warnings'
```

---

## INCIDENT RESPONSE

### Incident Severity Levels

| Severity | Definition | Response Time | On-Call |
|----------|-----------|---------------|---------|
| **SEV1 - Critical** | Service down, major data loss, security breach | <15 minutes | All hands |
| **SEV2 - High** | Degraded performance, affecting >50% users | <1 hour | Primary + Backup |
| **SEV3 - Medium** | Minor issues, workarounds available | <4 hours | Primary |
| **SEV4 - Low** | Cosmetic issues, no user impact | Next business day | Queue |

### Incident Response Playbook

**Step 1: Detect & Alert (0-5 min)**
- Alert fired from monitoring system
- On-call engineer paged via PagerDuty
- Automated health checks identify affected components

**Step 2: Acknowledge & Assess (5-15 min)**
```bash
# Quick diagnostics
systemctl status only1mcp
journalctl -u only1mcp -n 100 --no-pager
curl http://localhost:8080/health
curl http://localhost:9090/metrics | grep error

# Check recent deployments
git log --oneline -n 5
ls -lt /usr/local/bin/only1mcp*

# Review Grafana dashboards
open https://grafana.only1mcp.dev/d/overview
```

**Step 3: Contain & Mitigate (15-30 min)**
- If recent deployment: Rollback immediately
  ```bash
  ./scripts/rollback.sh
  ```
- If backend issue: Remove unhealthy backend from pool
  ```bash
  only1mcp remove <backend_id>
  ```
- If load spike: Scale horizontally or enable rate limiting
  ```bash
  # Emergency rate limiting
  curl -X POST http://localhost:8080/api/v1/admin/config \
       -d '{"rate_limit": {"requests_per_minute": 30}}'
  ```

**Step 4: Resolve (30-120 min)**
- Root cause analysis
- Apply fix (code change, config update, infrastructure scaling)
- Validate fix in staging first if possible
- Deploy fix to production
- Monitor for 30 minutes post-fix

**Step 5: Document & Learn (Post-incident)**
- Write incident report within 24 hours
- Schedule postmortem meeting (blameless)
- Identify action items to prevent recurrence
- Update runbooks and monitoring

### Incident Report Template

```markdown
# Incident Report: [Title]

**Date:** 2025-10-14  
**Severity:** SEV2  
**Status:** Resolved  
**Duration:** 1h 23m (14:15 UTC - 15:38 UTC)  
**Incident Commander:** Jane Doe  

## Summary
Brief description of what went wrong and impact to users.

## Impact
- **Users affected:** ~5,000 (20% of user base)
- **Services affected:** Tool calls to web-search backend
- **Error rate:** 45% during incident window
- **Revenue impact:** Estimated $500 in lost credits

## Timeline (UTC)
- **14:15** - Alert fired: High error rate on web-search backend
- **14:17** - On-call acknowledged, began investigation
- **14:25** - Root cause identified: SSL certificate expired
- **14:35** - Certificate renewed, propagation started
- **14:50** - Error rate dropped to 5%
- **15:38** - Error rate normalized, incident closed

## Root Cause
The SSL certificate for the web-search backend expired at 14:00 UTC. Auto-renewal via certbot failed due to DNS misconfiguration introduced in a recent infrastructure change.

## Resolution
1. Manually renewed certificate using `certbot renew --force-renewal`
2. Fixed DNS records to allow automatic renewal
3. Added monitoring for certificate expiry (alert 7 days before)

## Action Items
- [ ] Implement automated certificate expiry checks (Owner: DevOps, Due: 2025-10-21)
- [ ] Add certificate monitoring to weekly health checks (Owner: SRE, Due: 2025-10-18)
- [ ] Document certificate renewal process in runbook (Owner: Jane, Due: 2025-10-15)
- [ ] Set up redundant certificate authority (Let's Encrypt + ZeroSSL) (Owner: DevOps, Due: 2025-11-01)

## Lessons Learned
**What went well:**
- Alert fired quickly (within 2 minutes of issue)
- Team responded within SLA (<15 min for SEV2)
- Clear runbook for certificate issues

**What could be improved:**
- Certificate monitoring gaps
- Manual intervention required (should be automated)
- DNS change review process needs improvement
```

### On-Call Rotation

**Schedule:**
- Primary on-call: 1 week rotation
- Backup on-call: Overlaps with next primary
- Escalation to engineering manager if unresolved after 2 hours

**On-Call Responsibilities:**
- Monitor #only1mcp-alerts Slack channel
- Respond to pages within 15 minutes
- Triage and resolve incidents
- Update incident status in Statuspage.io
- Write incident reports for SEV1-SEV2 incidents

**On-Call Handoff:**
- Monday 10:00 AM UTC
- Handoff document includes:
  - Open incidents
  - Recent changes/deployments
  - Known issues and workarounds
  - Upcoming maintenance windows

---

## BACKUP & RECOVERY

### Data to Backup

1. **Configuration files**
   - `/etc/only1mcp/config.yaml`
   - `/etc/only1mcp/servers/*.yaml`
   - Environment-specific configs

2. **TLS certificates**
   - `/etc/only1mcp/certs/`

3. **Audit logs** (if persisted)
   - `/var/log/only1mcp/audit/`

4. **Server registry state** (if stateful mode)
   - Redis backup or database dump

### Backup Strategy

```bash
#!/bin/bash
# /opt/only1mcp/scripts/backup.sh - Daily backup script

BACKUP_DIR="/backup/only1mcp/$(date +%Y-%m-%d)"
mkdir -p "$BACKUP_DIR"

# 1. Configuration
echo "Backing up configuration..."
tar czf "$BACKUP_DIR/config.tar.gz" /etc/only1mcp/

# 2. Certificates
echo "Backing up certificates..."
tar czf "$BACKUP_DIR/certs.tar.gz" /etc/only1mcp/certs/

# 3. Audit logs (last 7 days)
echo "Backing up audit logs..."
find /var/log/only1mcp/audit/ -mtime -7 -type f | tar czf "$BACKUP_DIR/audit-logs.tar.gz" -T -

# 4. Metadata
echo "Creating backup manifest..."
cat > "$BACKUP_DIR/manifest.txt" <<EOF
Backup Date: $(date -Iseconds)
Hostname: $(hostname)
Only1MCP Version: $(only1mcp --version)
Config Hash: $(sha256sum /etc/only1mcp/config.yaml | awk '{print $1}')
EOF

# 5. Upload to S3 (encrypted)
echo "Uploading to S3..."
aws s3 sync "$BACKUP_DIR" "s3://only1mcp-backups/$(hostname)/$(date +%Y-%m-%d)/" \
    --sse AES256

# 6. Cleanup old local backups (>30 days)
find /backup/only1mcp/ -type d -mtime +30 -exec rm -rf {} \;

echo "✅ Backup complete: $BACKUP_DIR"
```

**Backup Schedule:**
- Daily at 02:00 UTC (cron)
- Retention: 30 days local, 90 days S3
- Weekly full backups retained for 1 year

### Disaster Recovery Plan

**RTO (Recovery Time Objective):** 1 hour  
**RPO (Recovery Point Objective):** 24 hours

**Recovery Procedure:**

```bash
#!/bin/bash
# /opt/only1mcp/scripts/restore.sh - Disaster recovery

BACKUP_DATE=${1:-$(date +%Y-%m-%d)}
BACKUP_DIR="/tmp/only1mcp-restore"

echo "🔄 Starting disaster recovery for backup: $BACKUP_DATE"

# 1. Download from S3
echo "📥 Downloading backup from S3..."
aws s3 sync "s3://only1mcp-backups/$(hostname)/$BACKUP_DATE/" "$BACKUP_DIR/"

# 2. Verify backup integrity
echo "✅ Verifying backup..."
cd "$BACKUP_DIR"
sha256sum -c manifest.txt || { echo "❌ Backup verification failed"; exit 1; }

# 3. Restore configuration
echo "📂 Restoring configuration..."
tar xzf config.tar.gz -C /

# 4. Restore certificates
echo "🔐 Restoring certificates..."
tar xzf certs.tar.gz -C /
chmod 600 /etc/only1mcp/certs/*.pem

# 5. Restore audit logs (optional)
echo "📝 Restoring audit logs..."
tar xzf audit-logs.tar.gz -C /var/log/only1mcp/audit/

# 6. Validate configuration
echo "🔍 Validating configuration..."
/usr/local/bin/only1mcp validate --config /etc/only1mcp/config.yaml

# 7. Restart service
echo "▶️  Restarting service..."
systemctl restart only1mcp

# 8. Health check
echo "🏥 Running health checks..."
sleep 5
curl -f http://localhost:8080/health || { echo "❌ Health check failed"; exit 1; }

echo "✅ Disaster recovery complete!"
```

**Testing Recovery:**
- Quarterly DR drills
- Restore to staging environment
- Validate all functionality
- Document any issues

---

## SCALING STRATEGY

### Horizontal Scaling

**Load Balancer Configuration (NGINX):**
```nginx
upstream only1mcp_backends {
    # Consistent hashing based on client IP
    hash $remote_addr consistent;
    
    server only1mcp-1.internal:8080 max_fails=3 fail_timeout=30s;
    server only1mcp-2.internal:8080 max_fails=3 fail_timeout=30s;
    server only1mcp-3.internal:8080 max_fails=3 fail_timeout=30s;
    
    # Health check
    check interval=3000 rise=2 fall=3 timeout=1000 type=http;
    check_http_send "GET /health HTTP/1.0\r\n\r\n";
    check_http_expect_alive http_2xx;
}

server {
    listen 443 ssl http2;
    server_name only1mcp.dev;
    
    ssl_certificate /etc/nginx/certs/fullchain.pem;
    ssl_certificate_key /etc/nginx/certs/privkey.pem;
    
    location / {
        proxy_pass http://only1mcp_backends;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        
        # WebSocket support
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
    }
}
```

**Auto-Scaling Policy:**
```yaml
# AWS Auto Scaling Group
Resources:
  Only1MCPAutoScalingGroup:
    Type: AWS::AutoScaling::AutoScalingGroup
    Properties:
      MinSize: 2
      MaxSize: 10
      DesiredCapacity: 3
      HealthCheckType: ELB
      HealthCheckGracePeriod: 300
      
      TargetTrackingScalingPolicies:
        # Scale on CPU
        - PolicyName: CPUUtilization
          TargetValue: 70.0
          PredefinedMetricSpecification:
            PredefinedMetricType: ASGAverageCPUUtilization
        
        # Scale on request count
        - PolicyName: RequestCount
          TargetValue: 1000.0  # 1000 req/s per instance
          CustomizedMetricSpecification:
            MetricName: only1mcp_requests_total
            Namespace: Only1MCP
            Statistic: Average
```

### Vertical Scaling

**Resource Limits:**
```yaml
# Kubernetes deployment
apiVersion: apps/v1
kind: Deployment
metadata:
  name: only1mcp
spec:
  replicas: 3
  template:
    spec:
      containers:
        - name: only1mcp
          image: ghcr.io/only1mcp/only1mcp:latest
          resources:
            requests:
              cpu: 500m
              memory: 512Mi
            limits:
              cpu: 2000m
              memory: 2Gi
          
          livenessProbe:
            httpGet:
              path: /health
              port: 8080
            initialDelaySeconds: 10
            periodSeconds: 10
          
          readinessProbe:
            httpGet:
              path: /ready
              port: 8080
            initialDelaySeconds: 5
            periodSeconds: 5
```

### Database Scaling (Future)

If adding persistent storage:
- **Read replicas** for read-heavy workloads
- **Sharding** by server ID or user ID
- **Caching layer** (Redis) for hot data
- **Connection pooling** (PgBouncer for PostgreSQL)

---

## SECURITY OPERATIONS

### Security Monitoring

**1. Access Logs Analysis:**
```bash
# Detect suspicious patterns
tail -f /var/log/only1mcp/access.log | grep -E "401|403|429" | \
  awk '{print $1}' | sort | uniq -c | sort -rn | head -10
```

**2. Intrusion Detection:**
- Install and configure `fail2ban` for rate limiting
- Monitor for:
  - Multiple failed auth attempts (>5 in 1 minute)
  - Unusual request patterns (SQL injection, XSS)
  - Large payload sizes (>10MB)

**3. Vulnerability Scanning:**
```bash
# Daily automated scan
0 2 * * * /usr/local/bin/trivy image ghcr.io/only1mcp/only1mcp:latest --severity HIGH,CRITICAL
```

**4. Audit Log Review:**
- Weekly manual review of audit logs
- Automated alerts for:
  - Admin API usage
  - Server additions/removals
  - Configuration changes
  - Failed authentication

### Incident Response - Security

**Security Incident Playbook:**

1. **Contain:**
   - Isolate affected systems
   - Block malicious IPs at firewall level
   - Disable compromised accounts

2. **Investigate:**
   - Preserve logs and evidence
   - Identify scope of breach
   - Determine attack vector

3. **Eradicate:**
   - Patch vulnerabilities
   - Remove malware/backdoors
   - Reset all credentials

4. **Recover:**
   - Restore from clean backups
   - Re-enable services gradually
   - Monitor closely for 72 hours

5. **Report:**
   - Notify affected users
   - Report to authorities if required (GDPR, etc.)
   - Document lessons learned

---

## RUNBOOKS & PROCEDURES

### Runbook: High Memory Usage

**Symptoms:**
- Memory usage >80% for >5 minutes
- OOM killer activating
- Slow response times

**Diagnosis:**
```bash
# Check current memory usage
free -h
ps aux --sort=-%mem | head -10

# Inspect Only1MCP process
pidof only1mcp
cat /proc/$(pidof only1mcp)/status | grep -E "VmRSS|VmSize"

# Check for memory leaks
valgrind --leak-check=full --log-file=/tmp/valgrind.log /usr/local/bin/only1mcp --config /etc/only1mcp/config.yaml
```

**Resolution:**
1. Clear cache: `curl -X POST http://localhost:8080/api/v1/admin/cache/clear`
2. Restart service: `systemctl restart only1mcp`
3. If persistent, check for memory leaks in code
4. Scale vertically (increase instance memory)

### Runbook: SSL Certificate Renewal

**Frequency:** 60 days before expiry

**Procedure:**
```bash
# 1. Check expiry date
openssl x509 -in /etc/only1mcp/certs/fullchain.pem -noout -enddate

# 2. Renew with certbot
certbot renew --dry-run  # Test first
certbot renew

# 3. Reload service (graceful)
systemctl reload only1mcp

# 4. Verify
curl -vI https://only1mcp.dev 2>&1 | grep "expire date"
```

### Runbook: Database Migration (Future)

**Pre-Migration:**
- [ ] Backup database
- [ ] Test migration on staging
- [ ] Schedule maintenance window
- [ ] Notify users

**Migration:**
```bash
# 1. Enable maintenance mode
curl -X POST http://localhost:8080/api/v1/admin/maintenance -d '{"enabled": true}'

# 2. Stop service
systemctl stop only1mcp

# 3. Run migrations
sqlx migrate run

# 4. Start service
systemctl start only1mcp

# 5. Verify
curl http://localhost:8080/health

# 6. Disable maintenance mode
curl -X POST http://localhost:8080/api/v1/admin/maintenance -d '{"enabled": false}'
```

**Rollback Plan:**
```bash
# Restore from backup
psql only1mcp < /backup/only1mcp-$(date +%Y-%m-%d).sql
systemctl restart only1mcp
```

---

## APPENDIX: Systemd Service File

```ini
# /etc/systemd/system/only1mcp.service
[Unit]
Description=Only1MCP - MCP Server Aggregator
After=network.target
Wants=network-online.target

[Service]
Type=notify
User=only1mcp
Group=only1mcp
WorkingDirectory=/opt/only1mcp

# Environment
Environment="ONLY1MCP_ENV=production"
Environment="ONLY1MCP_CONFIG_PATH=/etc/only1mcp/config.yaml"
EnvironmentFile=/etc/only1mcp/environment

# Binary
ExecStart=/usr/local/bin/only1mcp start --config ${ONLY1MCP_CONFIG_PATH}
ExecReload=/bin/kill -HUP $MAINPID

# Security
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/log/only1mcp /var/lib/only1mcp

# Resource limits
LimitNOFILE=65536
LimitNPROC=4096

# Restart policy
Restart=always
RestartSec=10s
StartLimitInterval=200
StartLimitBurst=3

# Logging
StandardOutput=journal
StandardError=journal
SyslogIdentifier=only1mcp

[Install]
WantedBy=multi-user.target
```

---

**Document Status:** ✅ COMPLETE  
**Next Review:** Quarterly or after major releases  
**Maintained By:** Only1MCP Operations Team  
**Emergency Contact:** ops@only1mcp.dev  
**Incident Hotline:** +1-555-ONLY1MCP
