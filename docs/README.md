# Only1MCP Documentation

This directory contains comprehensive documentation for the Only1MCP project, covering architecture, configuration, deployment, and operational guides.

## Table of Contents

### User Guides

- **[Configuration Guide](CONFIGURATION_GUIDE.md)** - Complete guide to configuring Only1MCP, including YAML/TOML schemas, validation, hot-reload setup, and configuration templates
- **[CLI Reference](CLI_REFERENCE.md)** - Command-line interface documentation with all commands, flags, and usage examples
- **[Deployment Guide](DEPLOYMENT_GUIDE.md)** - Production deployment strategies including Docker, Kubernetes, bare metal, and cloud platforms
- **[Monitoring Guide](MONITORING_GUIDE.md)** - Observability setup with Prometheus metrics, OpenTelemetry tracing, logging, and alerting rules
- **[Troubleshooting Guide](TROUBLESHOOTING.md)** - Common issues, debugging techniques, performance tuning, and problem resolution

### API & Architecture

- **[API Reference](API_REFERENCE.md)** - Complete REST and WebSocket API documentation with request/response schemas
- **[Architecture](ARCHITECTURE.md)** - System architecture overview, component design, and data flow diagrams

### Development Phases

- **[Phase 1 Plan](PHASE_1_PLAN.md)** - MVP implementation (Weeks 1-4): Core proxy, STDIO transport, basic configuration
- **[Phase 2 Plan](PHASE_2_PLAN.md)** - Advanced features (Weeks 5-8): Load balancing, health checking, caching, TUI
- **[Phase 3 Plan](PHASE_3_PLAN.md)** - Enterprise features (Weeks 9-12): OAuth2/JWT, RBAC, audit logging, web dashboard
- **[Phase 4 Plan](PHASE_4_PLAN.md)** - Extensions (Weeks 13+): Plugin system, AI optimization, GUI application

### Project Information

- **[Extraction Summary](EXTRACTION_SUMMARY.md)** - Summary of documentation extraction and organization process

## Quick Navigation

### Getting Started
1. Read the [Configuration Guide](CONFIGURATION_GUIDE.md) to set up your first config
2. Check the [CLI Reference](CLI_REFERENCE.md) for command usage
3. Review the [Deployment Guide](DEPLOYMENT_GUIDE.md) for your target environment

### Troubleshooting
1. Consult the [Troubleshooting Guide](TROUBLESHOOTING.md) for common issues
2. Check the [Monitoring Guide](MONITORING_GUIDE.md) for observability setup
3. Review logs with appropriate `RUST_LOG` levels

### Development
1. See [Architecture](ARCHITECTURE.md) for system design
2. Review phase plans to understand the development roadmap
3. Check the [API Reference](API_REFERENCE.md) for integration details

## Documentation Standards

All documentation in this directory follows these conventions:

- **Markdown Format**: All docs use GitHub-flavored Markdown
- **Code Examples**: Include working examples with expected output
- **Version Info**: Documentation is updated with each release
- **Cross-References**: Internal links use relative paths

## Contributing to Documentation

When updating documentation:

1. Maintain consistent formatting and structure
2. Include practical examples for all features
3. Update related documents when making changes
4. Test all code examples before committing
5. Use clear, concise language

## Additional Resources

- **Reference Documentation**: See `/ref_docs/` for comprehensive AI-extracted specifications
- **Configuration Templates**: See `/config/templates/` for example configurations
- **Source Code**: See `/src/` for implementation details
- **Tests**: See `/tests/` for integration test examples

## Documentation Roadmap

Planned additions:
- Performance tuning guide
- Advanced routing strategies
- Plugin development tutorial
- Multi-region deployment patterns
- Disaster recovery procedures

---

**Last Updated**: 2024-01-15
**Version**: 0.1.0
