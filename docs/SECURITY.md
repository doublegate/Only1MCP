# Security Policy

## Supported Versions

Only1MCP follows semantic versioning. We provide security updates for the following versions:

| Version | Supported          |
| ------- | ------------------ |
| 0.2.x   | :white_check_mark: |
| < 0.2.0 | :x:                |

## Reporting a Vulnerability

**DO NOT** report security vulnerabilities through public GitHub issues.

Instead, please report them via email to: **security@only1mcp.dev**

You should receive a response within 48 hours. If for some reason you do not, please follow up via GitHub to ensure we received your original message.

Please include the following information:

- Type of vulnerability (RCE, DoS, information disclosure, etc.)
- Full paths of source file(s) related to the manifestation
- Location of the affected source code (tag/branch/commit or direct URL)
- Step-by-step instructions to reproduce the issue
- Proof-of-concept or exploit code (if possible)
- Impact of the issue, including how an attacker might exploit it

---

## Security Best Practices

### Production Deployment

#### 1. Enable TLS/HTTPS

```yaml
server:
  tls:
    enabled: true
    cert_path: "/etc/only1mcp/certs/server.crt"
    key_path: "/etc/only1mcp/certs/server.key"
```

**Recommendations:**
- Use TLS 1.3 minimum
- Use Let's Encrypt for free, automated certificates
- Enable HSTS (HTTP Strict Transport Security)

#### 2. Configure Authentication

```yaml
auth:
  jwt:
    enabled: true
    secret: "${JWT_SECRET}"  # Use environment variable
    issuer: "only1mcp.example.com"
    audience: ["mcp-clients"]
    algorithm: "RS256"

  oauth:
    enabled: true
    providers:
      - name: "google"
        client_id: "${OAUTH_CLIENT_ID}"
        client_secret: "${OAUTH_CLIENT_SECRET}"
```

**Never commit secrets to version control!**

#### 3. Enable RBAC

```yaml
auth:
  rbac:
    enabled: true
    policies:
      - role: "admin"
        permissions: ["*"]
      - role: "user"
        permissions: ["tools:list", "tools:call"]
      - role: "readonly"
        permissions: ["tools:list"]
```

#### 4. Rate Limiting

```yaml
proxy:
  rate_limiting:
    enabled: true
    requests_per_second: 1000
    burst: 2000
    per_client: true
```

### Network Security

#### Firewall Configuration

```bash
# Allow only necessary ports
sudo ufw default deny incoming
sudo ufw default allow outgoing
sudo ufw allow 8080/tcp  # Or 443 for HTTPS
sudo ufw enable
```

#### Reverse Proxy (Recommended)

Use nginx or HAProxy in front of Only1MCP:

```nginx
# nginx.conf
upstream only1mcp {
    server 127.0.0.1:8080;
    keepalive 32;
}

server {
    listen 443 ssl http2;
    server_name only1mcp.example.com;

    ssl_certificate /etc/letsencrypt/live/example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/example.com/privkey.pem;
    ssl_protocols TLSv1.3;

    location / {
        proxy_pass http://only1mcp;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    }
}
```

### Container Security

#### Docker

```dockerfile
# Run as non-root user
USER only1mcp

# Read-only root filesystem
docker run --read-only \
  --tmpfs /tmp \
  --tmpfs /var/log/only1mcp \
  only1mcp:latest
```

#### Kubernetes

```yaml
securityContext:
  runAsNonRoot: true
  runAsUser: 1000
  readOnlyRootFilesystem: true
  allowPrivilegeEscalation: false
  capabilities:
    drop:
      - ALL
```

### Input Validation

Only1MCP validates all inputs:

- **Request bodies**: Max 10MB by default
- **Headers**: Max 8KB total
- **URLs**: Validated against whitelist (if configured)
- **JSON-RPC**: Schema validation for all methods

### Resource Limits

Prevent resource exhaustion:

```yaml
server:
  max_connections: 50000
  max_request_size_bytes: 10485760  # 10MB
  max_header_size_bytes: 8192       # 8KB

proxy:
  connection_pool:
    max_per_backend: 100
    timeout_seconds: 30

context_optimization:
  cache:
    max_entries: 10000
    max_memory_mb: 512
```

### Secrets Management

**DO NOT** hardcode secrets in configuration files!

#### Environment Variables

```bash
export JWT_SECRET=$(openssl rand -base64 32)
export OAUTH_CLIENT_SECRET="..."
only1mcp start
```

#### External Secrets Manager

```yaml
auth:
  jwt:
    secret: "${env:JWT_SECRET}"
    # Or use AWS Secrets Manager:
    # secret: "aws:secretsmanager:us-east-1:jwt-secret"
```

---

## Known Security Considerations

### 1. STDIO Transport

Running external processes via STDIO carries inherent risks:

**Mitigations:**
- Processes run with resource limits (CPU, memory)
- Sandbox mode enabled by default
- Command validation (path traversal prevention)
- Environment variable sanitization

### 2. SSE Transport

Server-Sent Events don't support standard authentication headers:

**Mitigations:**
- URL-based tokens for SSE endpoints
- Short-lived tokens with rotation
- Connection limits per IP

### 3. Cache Poisoning

Malicious responses could be cached:

**Mitigations:**
- Cache keys include request hash
- TTL limits prevent long-term poisoning
- Admin API to invalidate cache

### 4. DoS Protection

**Built-in Protections:**
- Connection limits
- Request rate limiting
- Circuit breakers prevent cascade failures
- Health checks isolate failing backends

**Additional Recommendations:**
- Deploy behind CDN/DDoS protection
- Use web application firewall (WAF)
- Monitor abnormal traffic patterns

---

## Dependency Security

### Automated Scanning

```bash
# Install cargo-audit
cargo install cargo-audit

# Run security audit
cargo audit

# Check for advisories
cargo audit --json | jq '.vulnerabilities'
```

### Update Strategy

- **Patch versions**: Applied automatically
- **Minor versions**: Reviewed quarterly
- **Major versions**: Comprehensive testing required

### Minimal Dependencies

Only1MCP uses only well-maintained, audited crates:

- **axum**: Official Tokio project
- **tokio**: Industry standard async runtime
- **serde**: De facto serialization standard
- **rustls**: Memory-safe TLS implementation

---

## Compliance

### Data Privacy

Only1MCP does not store:
- Request payloads (beyond cache TTL)
- User credentials
- Personal information

**Logs contain:**
- IP addresses (can be anonymized)
- Request timestamps
- HTTP methods and paths
- Response status codes

**GDPR Compliance:**
- Enable log anonymization in production
- Configure cache TTL appropriately
- Implement request deletion API if needed

### Audit Logging

```yaml
observability:
  audit_logging:
    enabled: true
    destination: "file:/var/log/only1mcp/audit.log"
    events:
      - "auth.success"
      - "auth.failure"
      - "config.change"
      - "admin.action"
```

---

## Security Checklist

### Deployment

- [ ] TLS enabled with valid certificates
- [ ] Authentication configured (JWT/OAuth)
- [ ] RBAC policies defined
- [ ] Rate limiting enabled
- [ ] Firewall rules configured
- [ ] Running as non-root user
- [ ] Resource limits set
- [ ] Secrets in environment/vault (not config files)
- [ ] Audit logging enabled
- [ ] Monitoring/alerting configured

### Operations

- [ ] Regular dependency updates (`cargo audit`)
- [ ] Log review for suspicious activity
- [ ] Access control reviews
- [ ] Backup strategy tested
- [ ] Incident response plan documented
- [ ] Security patch process defined

---

## Security Updates

Subscribe to security announcements:

- **GitHub**: Watch repository for security advisories
- **Email**: Subscribe to security@only1mcp.dev (coming soon)
- **RSS**: https://github.com/doublegate/Only1MCP/security/advisories.atom

---

## Acknowledgments

We thank the security researchers who have helped make Only1MCP more secure:

- (List will be updated as vulnerabilities are responsibly disclosed)

---

**Last Updated:** October 20, 2025
