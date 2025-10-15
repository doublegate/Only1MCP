# Changelog

All notable changes to Only1MCP will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added (January 2025)
- Complete OAuth2/OIDC authentication system with PKCE support (550 lines)
- Hierarchical RBAC with dynamic policy engine (700+ lines)
- HTTP transport with bb8 connection pooling and retry logic (455 lines)
- Advanced load balancing with 5 algorithms (666 lines)
- Circuit breaker pattern implementation (436 lines)
- Comprehensive user documentation suite (5,000+ lines total):
  - Configuration Guide with YAML/TOML/JSON schemas
  - CLI Reference with all commands and options
  - Deployment Guide for Docker, Kubernetes, and cloud
  - Monitoring Guide with Prometheus/Grafana/Jaeger setup
  - Troubleshooting Guide with 60+ scenarios
- Enterprise configuration templates (solo, team, enterprise)
- Complete CI/CD workflows (release.yml, benchmark.yml)
- API Reference documentation
- Architecture documentation with diagrams
- Project Roadmap with 4-phase plan
- Phase 1-4 detailed planning documents

### Changed
- License updated to GPL v3
- Repository moved to https://github.com/doublegate/Only1MCP
- Status updated to Beta Ready
- Documentation links updated to point to actual files

### Fixed
- All GitHub URLs updated to correct repository
- Documentation structure organized in /docs directory
- Configuration templates validated and enhanced

### Security
- OAuth2/OIDC implementation with secure token handling
- JWT validation with RS256/HS256 support
- RBAC with hierarchical roles and dynamic policies
- TLS 1.3 minimum enforcement
- Process sandboxing for STDIO transport

## [0.1.0] - TBD (Target: 4 weeks)

### Planned - MVP Release
- Core proxy routing functionality
- STDIO transport for process-based MCP servers
- HTTP transport for remote MCP servers
- Server registry with hot-swap capability
- YAML configuration loading
- CLI commands (start, list, validate, test)
- Basic logging and error handling
- Integration tests with real MCP servers

## [0.2.0] - TBD (Target: 8 weeks)

### Planned - Advanced Features
- Consistent hashing load balancer
- Least connections routing
- Active health checks with circuit breakers
- Response caching with TTL
- Request batching (opt-in)
- Prometheus metrics export
- Enhanced CLI tools
- Performance benchmarks

## [0.3.0] - TBD (Target: 12 weeks)

### Planned - Enterprise Ready
- OAuth2 / JWT authentication
- Role-based access control (RBAC)
- Audit logging
- TLS 1.3 support
- Rate limiting
- OpenTelemetry tracing
- Interactive TUI
- Grafana dashboard templates
- Complete user documentation

## [1.0.0] - TBD (Target: 16 weeks)

### Planned - Production Release
- Plugin system (dynamic libraries, WASM)
- AI-driven routing optimization
- Container orchestration (optional)
- Advanced observability features
- Performance optimizations
- Security hardening
- Complete test coverage (>90%)
- Production deployment guides

---

## Categories

### Added
New features or functionality.

### Changed
Changes in existing functionality.

### Deprecated
Features marked for removal in future releases.

### Removed
Removed features.

### Fixed
Bug fixes.

### Security
Security vulnerability fixes.

---

**Note:** Only1MCP is currently in active development (Phase 1 MVP).
Release dates are estimates and subject to change based on development progress.

For detailed task breakdown, see [Master Tracker](to-dos/master-tracker.md).
