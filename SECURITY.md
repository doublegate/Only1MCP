# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |
| < 0.1.0 | :x:                |

## Reporting a Vulnerability

We take security vulnerabilities seriously. If you discover a security issue, please follow these steps:

1. **DO NOT** create a public GitHub issue
2. Email security details to: security@only1mcp.dev
3. Include:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested fix (if any)

### Response Timeline

- **Initial Response:** Within 48 hours
- **Status Update:** Within 7 days
- **Resolution Target:** Within 30 days for critical issues

## Security Measures

### Process Sandboxing

Only1MCP implements strict sandboxing for STDIO processes:
- Resource limits (CPU, memory)
- User privilege dropping
- Process isolation
- Network restrictions

### Authentication

- JWT token validation with RS256
- API key management with bcrypt
- OAuth2/OIDC support with PKCE
- Role-based access control (RBAC)

### Transport Security

- TLS 1.3 minimum
- Certificate pinning available
- Mutual TLS (mTLS) support
- Encrypted configuration storage

### Input Validation

- All inputs sanitized
- JSON schema validation
- Rate limiting per endpoint
- Request size limits

## Security Best Practices

### Configuration

```yaml
# Recommended security configuration
security:
  tls:
    enabled: true
    min_version: "1.3"
    cipher_suites:
      - TLS_AES_256_GCM_SHA384
      - TLS_CHACHA20_POLY1305_SHA256

  auth:
    required: true
    jwt:
      algorithm: RS256
      issuer: "only1mcp"
      audience: "mcp-proxy"

  rate_limiting:
    enabled: true
    global_limit: 10000
    per_client_limit: 1000

  sandbox:
    enabled: true
    max_memory_mb: 512
    max_cpu_percent: 50
```

### Deployment

1. **Run as non-root user**
   ```bash
   useradd -r -s /bin/false only1mcp
   chown -R only1mcp:only1mcp /opt/only1mcp
   ```

2. **Use systemd security features**
   ```ini
   [Service]
   User=only1mcp
   PrivateTmp=true
   ProtectSystem=strict
   ProtectHome=true
   NoNewPrivileges=true
   ```

3. **Network isolation**
   - Use private networks for backend servers
   - Implement firewall rules
   - Enable DDoS protection

### Audit Logging

Enable comprehensive audit logging:
```yaml
observability:
  audit:
    enabled: true
    events:
      - authentication
      - authorization
      - configuration_change
      - server_access
      - admin_actions
    retention: 365d
```

## Known Security Considerations

### MCP Protocol

- MCP servers have access to execute tools
- Carefully vet all MCP servers before adding
- Use read-only connections where possible
- Implement tool whitelisting

### STDIO Transport

- Spawned processes inherit environment
- Sanitize all environment variables
- Use absolute paths for commands
- Validate command arguments

### Caching

- Cached responses may contain sensitive data
- Use encryption for cache storage
- Implement cache key namespacing
- Clear cache on security events

## Security Headers

Only1MCP sets the following security headers:

```
Strict-Transport-Security: max-age=63072000; includeSubDomains; preload
X-Frame-Options: DENY
X-Content-Type-Options: nosniff
X-XSS-Protection: 1; mode=block
Content-Security-Policy: default-src 'self'
```

## Dependency Management

- Regular dependency updates via Dependabot
- Security audit with `cargo audit`
- License compliance checking
- SBOM generation for releases

## Compliance

Only1MCP can be configured for:
- SOC2 compliance
- ISO 27001 requirements
- HIPAA (with encryption)
- GDPR (with data controls)

## Security Tools

### Static Analysis
```bash
# Run security lints
cargo clippy -- -W clippy::all

# Security audit
cargo audit

# Dependency licenses
cargo license
```

### Dynamic Analysis
```bash
# Fuzzing
cargo fuzz run proxy_fuzzer

# Memory safety
valgrind ./target/release/only1mcp

# Network analysis
tcpdump -i lo -w only1mcp.pcap
```

## Responsible Disclosure

We support responsible disclosure:
- Security researchers will be acknowledged
- We won't take legal action for good-faith reports
- We'll work with you to understand and fix issues

## Security Updates

Subscribe to security updates:
- GitHub Security Advisories
- Mailing list: security-announce@only1mcp.dev
- RSS feed: https://only1mcp.dev/security.rss

## Contact

- Security Team: security@only1mcp.dev
- PGP Key: [0xDEADBEEF]
- Bug Bounty: https://only1mcp.dev/bounty