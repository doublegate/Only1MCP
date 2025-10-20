# Only1MCP Deployment Guide

Comprehensive guide for deploying Only1MCP in production environments.

**Version:** 0.2.0
**Last Updated:** October 20, 2025

---

## Table of Contents

- [Prerequisites](#prerequisites)
- [Docker Deployment](#docker-deployment)
- [Docker Compose Stack](#docker-compose-stack)
- [Kubernetes Deployment](#kubernetes-deployment)
- [systemd Service (Bare Metal/VM)](#systemd-service)
- [Configuration](#configuration)
- [Security Hardening](#security-hardening)
- [Monitoring & Observability](#monitoring--observability)
- [Troubleshooting](#troubleshooting)

---

## Prerequisites

### System Requirements

**Minimum (Development):**
- CPU: 2 cores
- RAM: 2GB
- Disk: 10GB
- OS: Linux (kernel 5.4+), macOS 11+, Windows Server 2019+

**Recommended (Production):**
- CPU: 4+ cores
- RAM: 8GB+
- Disk: 50GB SSD
- OS: Ubuntu 22.04 LTS, Debian 12, RHEL 9

### Software Dependencies

- **Docker**: 24.0+ (for containerized deployments)
- **Kubernetes**: 1.28+ (for K8s deployments)
- **Rust**: 1.75+ (for source builds)
- **systemd**: 245+ (for service management)

---

## Docker Deployment

### Quick Start

```bash
# Build image
docker build -t only1mcp:latest .

# Run container
docker run -d \
  --name only1mcp \
  -p 8080:8080 \
  -v $(pwd)/config/templates/solo.yaml:/etc/only1mcp/only1mcp.yaml:ro \
  only1mcp:latest
```

### Production Deployment

```bash
# Create network
docker network create only1mcp-net

# Run with resource limits and health checks
docker run -d \
  --name only1mcp \
  --network only1mcp-net \
  -p 8080:8080 \
  --memory="512m" \
  --cpus="1.0" \
  --restart=unless-stopped \
  --health-cmd="curl -f http://localhost:8080/health || exit 1" \
  --health-interval=30s \
  --health-timeout=10s \
  --health-retries=3 \
  -v /path/to/production-config.yaml:/etc/only1mcp/only1mcp.yaml:ro \
  -v only1mcp-logs:/var/log/only1mcp \
  -e RUST_LOG=info \
  only1mcp:latest
```

### Verify Deployment

```bash
# Check container health
docker ps --filter name=only1mcp

# View logs
docker logs -f only1mcp

# Test health endpoint
curl http://localhost:8080/health

# Check metrics
curl http://localhost:8080/api/v1/admin/metrics
```

---

## Docker Compose Stack

Complete observability stack with Prometheus and Grafana.

### Start Stack

```bash
# Start all services
docker-compose up -d

# View logs
docker-compose logs -f only1mcp

# Check service status
docker-compose ps
```

### Services

- **only1mcp**: Proxy server (port 8080)
- **prometheus**: Metrics collection (port 9090)
- **grafana**: Metrics visualization (port 3000, admin/admin)
- **mock-backend-1/2**: Test backends (ports 9001, 9002)

### Stop Stack

```bash
docker-compose down

# Remove volumes (WARNING: destroys data)
docker-compose down -v
```

---

## Kubernetes Deployment

### Prerequisites

```bash
# Verify cluster access
kubectl cluster-info
kubectl get nodes
```

### Deploy

```bash
# Create namespace
kubectl create namespace only1mcp

# Apply ConfigMap
kubectl apply -f deployments/kubernetes/configmap.yaml -n only1mcp

# Deploy application
kubectl apply -f deployments/kubernetes/deployment.yaml -n only1mcp

# Create services
kubectl apply -f deployments/kubernetes/service.yaml -n only1mcp
```

### Verify Deployment

```bash
# Check pods
kubectl get pods -n only1mcp -l app=only1mcp

# Check services
kubectl get svc -n only1mcp

# View logs
kubectl logs -n only1mcp -l app=only1mcp -f

# Port-forward for testing
kubectl port-forward -n only1mcp svc/only1mcp 8080:80
```

### Scaling

```bash
# Scale to 5 replicas
kubectl scale deployment/only1mcp --replicas=5 -n only1mcp

# Autoscaling (HPA)
kubectl autoscale deployment/only1mcp \
  --cpu-percent=70 \
  --min=3 \
  --max=10 \
  -n only1mcp
```

### Rolling Updates

```bash
# Update image
kubectl set image deployment/only1mcp \
  only1mcp=only1mcp:v0.3.0 \
  -n only1mcp

# Monitor rollout
kubectl rollout status deployment/only1mcp -n only1mcp

# Rollback if needed
kubectl rollout undo deployment/only1mcp -n only1mcp
```

---

## systemd Service

For bare metal or VM deployments.

### Installation

```bash
# Create user and directories
sudo useradd -r -s /bin/false only1mcp
sudo mkdir -p /opt/only1mcp /etc/only1mcp /var/lib/only1mcp /var/log/only1mcp
sudo chown -R only1mcp:only1mcp /opt/only1mcp /var/lib/only1mcp /var/log/only1mcp

# Install binary
sudo cp target/release/only1mcp /usr/local/bin/
sudo chmod +x /usr/local/bin/only1mcp

# Install configuration
sudo cp config/templates/solo.yaml /etc/only1mcp/only1mcp.yaml
sudo chown only1mcp:only1mcp /etc/only1mcp/only1mcp.yaml

# Install systemd service
sudo cp deployments/systemd/only1mcp.service /etc/systemd/system/
sudo systemctl daemon-reload
```

### Management

```bash
# Enable at boot
sudo systemctl enable only1mcp

# Start service
sudo systemctl start only1mcp

# Check status
sudo systemctl status only1mcp

# View logs
sudo journalctl -u only1mcp -f

# Restart
sudo systemctl restart only1mcp

# Stop
sudo systemctl stop only1mcp
```

---

## Configuration

### Production Configuration Template

```yaml
server:
  host: "0.0.0.0"
  port: 8080
  worker_threads: 0  # Auto-detect
  max_connections: 50000
  tls:
    enabled: true  # Enable for HTTPS
    cert_path: "/etc/only1mcp/certs/server.crt"
    key_path: "/etc/only1mcp/certs/server.key"

servers:
  - id: "primary-backend"
    name: "Primary MCP Server"
    enabled: true
    transport:
      type: "http"
      url: "http://backend1:3000"
    health_check:
      enabled: true
      interval_seconds: 10
      timeout_seconds: 5
      healthy_threshold: 2
      unhealthy_threshold: 3
      path: "/health"
    weight: 100

proxy:
  load_balancer:
    algorithm: "round_robin"
    virtual_nodes: 150
  connection_pool:
    max_per_backend: 100
    min_idle: 10

context_optimization:
  cache:
    enabled: true
    max_entries: 10000
    ttl_seconds: 300
  batching:
    enabled: true
    window_ms: 100
    max_batch_size: 10

observability:
  logging:
    level: "info"
    format: "json"
```

### Environment Variables

```bash
# Override config path
export ONLY1MCP_CONFIG=/custom/path/config.yaml

# Set log level
export RUST_LOG=debug

# Disable colors (for log aggregation)
export NO_COLOR=1
```

---

## Security Hardening

### TLS Configuration

```bash
# Generate self-signed certificate (development only)
openssl req -x509 -newkey rsa:4096 \
  -keyout server.key -out server.crt \
  -days 365 -nodes \
  -subj "/CN=only1mcp.example.com"

# Production: Use Let's Encrypt
certbot certonly --standalone -d only1mcp.example.com
```

### Firewall Rules

```bash
# UFW (Ubuntu/Debian)
sudo ufw allow 8080/tcp
sudo ufw enable

# firewalld (RHEL/CentOS)
sudo firewall-cmd --permanent --add-port=8080/tcp
sudo firewall-cmd --reload

# iptables
sudo iptables -A INPUT -p tcp --dport 8080 -j ACCEPT
```

### Resource Limits

```yaml
# Kubernetes
resources:
  requests:
    memory: "256Mi"
    cpu: "250m"
  limits:
    memory: "512Mi"
    cpu: "1000m"
```

```bash
# systemd
LimitNOFILE=65536
LimitNPROC=4096
```

---

## Monitoring & Observability

### Prometheus Metrics

```yaml
# prometheus.yml
scrape_configs:
  - job_name: 'only1mcp'
    static_configs:
      - targets: ['only1mcp:8080']
    metrics_path: '/api/v1/admin/metrics'
    scrape_interval: 15s
```

### Key Metrics

- `http_requests_total` - Total HTTP requests
- `http_request_duration_seconds` - Request latency histogram
- `cache_hits_total` / `cache_misses_total` - Cache efficiency
- `server_health_status` - Backend health (0=unhealthy, 1=healthy)
- `batched_requests_total` - Request batching efficiency

### Grafana Dashboards

Import dashboard from `deployments/grafana/dashboards/only1mcp.json`

---

## Troubleshooting

### Common Issues

**Port Already in Use**
```bash
# Find process using port 8080
sudo lsof -i :8080
# or
sudo netstat -tulpn | grep 8080

# Change port in config
server:
  port: 8081
```

**Cannot Connect to Backend**
```bash
# Test backend connectivity
curl http://backend-host:3000/health

# Check network rules
sudo iptables -L -n -v

# Verify DNS resolution
nslookup backend-host
```

**High Memory Usage**
```yaml
# Reduce cache size
context_optimization:
  cache:
    max_entries: 5000  # Reduce from 10000
```

**Slow Response Times**
```bash
# Check backend health
curl http://localhost:8080/api/v1/admin/servers

# View metrics
curl http://localhost:8080/api/v1/admin/metrics | grep latency

# Enable caching
context_optimization:
  cache:
    enabled: true
```

### Logs

```bash
# Docker
docker logs -f only1mcp

# systemd
sudo journalctl -u only1mcp -f --since "1 hour ago"

# Kubernetes
kubectl logs -n only1mcp -l app=only1mcp -f
```

### Health Checks

```bash
# Basic health
curl http://localhost:8080/health

# Detailed status
curl http://localhost:8080/api/v1/admin/health

# List servers
curl http://localhost:8080/api/v1/admin/servers

# List tools
curl http://localhost:8080/api/v1/admin/tools
```

---

## Production Checklist

- [ ] TLS enabled with valid certificates
- [ ] Firewall configured
- [ ] Resource limits set
- [ ] Monitoring configured (Prometheus/Grafana)
- [ ] Logging aggregation enabled
- [ ] Backup strategy defined
- [ ] Disaster recovery plan documented
- [ ] Security audit completed
- [ ] Load testing performed
- [ ] Runbook created for on-call team

---

## Support

- **Documentation**: https://github.com/doublegate/Only1MCP/docs
- **Issues**: https://github.com/doublegate/Only1MCP/issues
- **Discussions**: https://github.com/doublegate/Only1MCP/discussions

---

**Made with ❤️ and Rust**
