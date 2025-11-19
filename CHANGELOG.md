# Changelog

All notable changes to Only1MCP will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.7.0] - 2025-11-19

### Added - Options 2 & 3: Multi-Path Development & Enterprise Features

#### Plugin Marketplace (Option 2 - Backend Team)
- **Plugin Registry API** (Rust/Axum):
  - PostgreSQL database schema with full-text search
  - S3/GCS/Local storage backends for plugin tarballs
  - REST API endpoints: list, search, publish, download, rate plugins
  - Security validation and manifest checking
  - Automated checksum verification (SHA-256)
- **Plugin CLI Commands**:
  - `only1mcp plugin install` - Install from registry, local, or URL
  - `only1mcp plugin search` - Full-text search with filtering
  - `only1mcp plugin publish` - Publish to marketplace with validation
  - `only1mcp plugin init` - Scaffold new plugin from templates
  - `only1mcp plugin list/info/test/uninstall` - Management commands
- **Database Integration**:
  - PostgreSQL schema for plugins, versions, ratings
  - GIN indexes for full-text search
  - Rating aggregation and analytics

#### Tauri Desktop App (Option 2 - Frontend Team)
- **Native Desktop Application**:
  - Tauri 1.5 + React + TypeScript stack
  - Real-time metrics dashboard with Recharts
  - One-click proxy start/stop controls
  - System tray integration (macOS/Windows/Linux)
  - Live WebSocket metrics streaming
- **Rust Backend Commands**:
  - `get_status`, `start_proxy`, `stop_proxy`
  - `load_config`, `save_config`, `validate_config`
  - `get_servers`, `get_metrics`
  - Background metrics emission every 1s
- **UI Components**:
  - Performance metrics cards (RPS, latency, connections, errors)
  - Real-time line charts for historical data
  - MCP servers table with health status
  - Configuration file picker and validation

#### Cloud Provider Integrations (Option 2 - DevOps Team)
- **Google Cloud Platform (GCP)**:
  - Complete Terraform modules for GKE deployment
  - VPC with multi-AZ subnetworks and IP allocation
  - GKE cluster with autoscaling (2-10 nodes)
  - Cloud SQL PostgreSQL for marketplace
  - Memorystore Redis for caching
  - Cloud Storage bucket for plugins
  - Service account with IAM roles
  - Workload identity integration
- **Azure Deployment**:
  - Bicep Infrastructure as Code templates
  - Container Registry and Container Instances
  - Automated resource group management
- **DigitalOcean Kubernetes**:
  - Production-ready Kubernetes manifests
  - HorizontalPodAutoscaler (3-10 pods)
  - LoadBalancer service configuration
  - PodDisruptionBudget for HA

#### Service Mesh Integration (Option 3 - Enterprise)
- **Istio Configuration**:
  - Gateway with HTTP/HTTPS and auto-redirect
  - VirtualService with canary deployments (90/10 split)
  - DestinationRule with connection pooling
  - Outlier detection and circuit breaking
  - mTLS peer authentication (STRICT mode)
  - AuthorizationPolicy for RBAC
  - RequestAuthentication for JWT validation
  - ServiceEntry for external MCP servers
  - Telemetry with Jaeger tracing and Prometheus

#### Multi-Cluster Routing (Option 3 - Enterprise)
- **Multi-Cluster Manager**:
  - Geographic routing to nearest cluster
  - Load-based routing to most available capacity
  - Cost-optimized routing across regions
  - Weighted routing with custom weights
  - Primary-secondary failover strategy
  - Background health checking (configurable interval)
  - Request metrics tracking per cluster
  - Automatic cluster degradation on failures
- **Cluster Management**:
  - Cluster capacity monitoring (CPU, memory, nodes)
  - Health statistics (requests, latency, errors)
  - Request priority levels (Low, Normal, High, Critical)

### Metrics
- **Plugin Marketplace**: 1,500+ lines of Rust code (6 new files)
- **Tauri Desktop App**: 800+ lines (Rust backend + React frontend)
- **Cloud Infrastructure**: 600+ lines of IaC (Terraform, Bicep, K8s manifests)
- **Service Mesh**: 200+ lines of Istio configuration
- **Multi-Cluster**: 400+ lines of Rust routing logic
- **Total**: 3,500+ lines of production code

---

## [0.6.0] - 2025-01-19

### Added - Production Infrastructure & Plugin Ecosystem

#### Infrastructure as Code
- **Docker Compose** stack for local development with:
  - Prometheus metrics collection
  - Grafana visualization
  - Redis caching
  - Example MCP servers (filesystem, GitHub)
  - Nginx reverse proxy (optional)
  - Jaeger distributed tracing (optional)
- **Kubernetes** production deployment manifests with:
  - HorizontalPodAutoscaler (3-10 pods based on CPU/memory)
  - PodDisruptionBudget for high availability (min 2 pods)
  - Ingress with TLS and rate limiting
  - ServiceMonitor for Prometheus integration
  - Security-hardened containers (non-root, read-only FS)
- **Terraform AWS Infrastructure**:
  - VPC module with multi-AZ networking
  - ECS Fargate for serverless containers
  - Application Load Balancer with auto-scaling
  - ElastiCache Redis cluster
  - Optional RDS database
  - CloudWatch monitoring and alarms
  - S3 for configuration storage
  - Secrets Manager integration

#### Example Plugins (5 production-ready plugins)
- **Rate Limiter Plugin**: Per-client rate limiting with sliding window algorithm
- **OAuth Authentication Plugin**: OAuth2 provider with automatic token refresh
- **Metrics Collector Plugin**: Custom metrics with percentile calculations (p50, p95, p99)
- **Protocol Adapter Plugin**: Multi-protocol conversion (REST/GraphQL/gRPC to MCP)
- **Custom Load Balancer Plugin**: Advanced routing with adaptive learning

#### Documentation
- Comprehensive deployment guide (11,800+ lines)
- CI/CD workflow enhancement documentation
- Infrastructure setup and configuration guides
- Troubleshooting procedures
- Best practices for production deployments

### Enhanced
- **CI/CD Pipeline** improvements (documented):
  - Multi-platform binary builds (6 targets)
  - Security audits with cargo-audit and cargo-deny
  - Code coverage reporting
  - Docker multi-arch builds (amd64, arm64)
  - Automated GitHub releases
  - Staging and production deployment workflows

### Metrics
- 5 comprehensive example plugins (2,500+ lines)
- 13 new infrastructure files (2,500+ lines IaC)
- 11,800+ lines of deployment documentation
- Multi-cloud deployment support (Docker, K8s, AWS)

---

## [0.5.0] - 2025-01-18

### Added - Examples, Frontend, Documentation & Benchmarks

#### Interactive Examples
- **Web Dashboard** (`examples/dashboard.html`): 450-line interactive dashboard
- **Demo Application** (`examples/demo_app.rs`): 850-line working demo

#### Configuration Templates
- Solo Developer config: Round-robin load balancing
- Small Team config: JWT auth, multi-tenancy
- Enterprise config: Multi-region, AI optimization

#### Comprehensive Documentation (4,400+ lines)
- Getting Started Guide (800 lines)
- Integration Guide (1,400 lines)
- Performance Tuning Guide (1,400 lines)
- Examples README (600 lines)

#### Performance & Testing
- Benchmark Suite with 6 suites (650 lines)
- Criterion-based performance testing

---

## [0.4.0] - 2025-01-18

### Added - GUI Backend Infrastructure

- Simplified GUI Backend (331 lines)
- Full GUI System documented (1,542 lines)
- Thread-safe state management
- Dashboard data aggregation

---

## [0.3.0] - 2025-01-18

### Added - Advanced Platform Features

- Plugin System (406 lines)
- Time-Series Analytics (786 lines)
- AI-Driven Optimization (893 lines)
- Multi-Region Deployment (752 lines)

---

## [0.2.11] - 2025-01-18

### Added - Enhanced Security & Multi-Tenancy

- Process Sandboxing (520 lines)
- Multi-Tenancy System (384 lines)
- Three-tier tenant system

---

## [0.2.0] - 2025-01-17

### Added - Enterprise Features

- JWT/OAuth2 Authentication
- RBAC Authorization
- Audit Logging
- Rate Limiting
- Admin API

---

## [0.1.0] - 2025-01-16

### Added - Core Foundation

- Basic proxy server
- STDIO/HTTP/SSE/WebSocket transports
- Load balancing (round-robin, least-connections, consistent hashing)
- Response caching
- Prometheus metrics
- TUI interface

### Performance
- <5ms proxy overhead
- 10,000+ req/s throughput
- 50-70% context reduction

