# Only1MCP Deployment Guide

Comprehensive deployment guide for Only1MCP in various environments.

## Table of Contents
- [Prerequisites](#prerequisites)
- [Deployment Architectures](#deployment-architectures)
- [Local Development](#local-development)
- [Docker Deployment](#docker-deployment)
- [Kubernetes Deployment](#kubernetes-deployment)
- [Cloud Deployments](#cloud-deployments)
  - [AWS](#aws-deployment)
  - [Google Cloud](#google-cloud-deployment)
  - [Azure](#azure-deployment)
- [Production Best Practices](#production-best-practices)
- [Security Hardening](#security-hardening)
- [Performance Tuning](#performance-tuning)
- [High Availability](#high-availability)
- [Monitoring Setup](#monitoring-setup)
- [Backup and Recovery](#backup-and-recovery)
- [Troubleshooting](#troubleshooting)

## Prerequisites

### System Requirements

**Minimum Requirements:**
- CPU: 2 cores
- RAM: 4GB
- Disk: 10GB
- Network: 100 Mbps

**Recommended Production:**
- CPU: 8+ cores
- RAM: 16GB+
- Disk: 100GB SSD
- Network: 1 Gbps

### Software Dependencies
- Linux (Ubuntu 20.04+, RHEL 8+, or compatible)
- systemd or similar init system
- OpenSSL 1.1.1+
- (Optional) Docker 20.10+
- (Optional) Kubernetes 1.20+

### Network Requirements
- Outbound HTTPS (443) to MCP servers
- Inbound HTTP/HTTPS on configured ports
- (Optional) Internal ports for metrics/admin

## Deployment Architectures

### Single Instance
```
┌─────────────┐     ┌─────────────┐
│   Clients   │────▶│  Only1MCP   │
└─────────────┘     └──────┬──────┘
                           │
                    ┌──────▼──────┐
                    │ MCP Servers │
                    └─────────────┘
```

### Load Balanced
```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Clients   │────▶│Load Balancer│────▶│  Only1MCP   │
└─────────────┘     └─────────────┘     │  Instances  │
                                        └──────┬──────┘
                                               │
                                        ┌──────▼──────┐
                                        │ MCP Servers │
                                        └─────────────┘
```

### Multi-Region
```
        ┌──────────────┐
        │Global LB/CDN │
        └──────┬───────┘
               │
    ┌──────────┴──────────┐
    │                     │
┌───▼────┐          ┌────▼────┐
│Region 1│          │Region 2 │
│Only1MCP│          │Only1MCP │
└────────┘          └─────────┘
```

## Local Development

### Quick Start
```bash
# Clone repository
git clone https://github.com/doublegate/Only1MCP
cd only1mcp

# Build from source
cargo build --release

# Generate configuration
./target/release/only1mcp config generate --template solo > config.yaml

# Start proxy
./target/release/only1mcp start --config config.yaml
```

### Development Configuration
```yaml
# dev-config.yaml
server:
  host: 127.0.0.1
  port: 8080
  workers: 2

servers:
  - id: local-stdio
    transport: stdio
    command: ["mcp-server"]
    enabled: true

proxy:
  hot_reload: true

logging:
  level: debug
  format: pretty

development:
  debug: true
  mock_servers: true
```

## Docker Deployment

### Building the Image
```dockerfile
# Dockerfile
FROM rust:1.70 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/only1mcp /usr/local/bin/
COPY config/templates /etc/only1mcp/templates

EXPOSE 8080 9090 9091
USER nobody

ENTRYPOINT ["only1mcp"]
CMD ["start", "--config", "/config/only1mcp.yaml"]
```

### Running with Docker
```bash
# Build image
docker build -t only1mcp:latest .

# Run container
docker run -d \
    --name only1mcp \
    -p 8080:8080 \
    -p 9090:9090 \
    -p 9091:9091 \
    -v $(pwd)/config.yaml:/config/only1mcp.yaml:ro \
    -v $(pwd)/data:/data \
    --restart unless-stopped \
    only1mcp:latest

# With environment variables
docker run -d \
    --name only1mcp \
    -p 8080:8080 \
    -e ONLY1MCP_CONFIG=/config/only1mcp.yaml \
    -e GITHUB_TOKEN=${GITHUB_TOKEN} \
    -e OPENAI_API_KEY=${OPENAI_API_KEY} \
    -v $(pwd)/config:/config:ro \
    only1mcp:latest
```

### Docker Compose
```yaml
# docker-compose.yml
version: '3.8'

services:
  only1mcp:
    image: only1mcp:latest
    ports:
      - "8080:8080"
      - "9090:9090"  # Admin
      - "9091:9091"  # Metrics
    volumes:
      - ./config.yaml:/config/only1mcp.yaml:ro
      - cache_data:/cache
      - ./logs:/logs
    environment:
      - ONLY1MCP_CONFIG=/config/only1mcp.yaml
      - RUST_LOG=info
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 30s
      timeout: 10s
      retries: 3
    networks:
      - only1mcp_net

  redis:
    image: redis:7-alpine
    volumes:
      - redis_data:/data
    networks:
      - only1mcp_net

  prometheus:
    image: prom/prometheus:latest
    ports:
      - "9090:9090"
    volumes:
      - ./prometheus.yml:/etc/prometheus/prometheus.yml:ro
      - prometheus_data:/prometheus
    networks:
      - only1mcp_net

volumes:
  cache_data:
  redis_data:
  prometheus_data:

networks:
  only1mcp_net:
    driver: bridge
```

## Kubernetes Deployment

### Namespace and ConfigMap
```yaml
# namespace.yaml
apiVersion: v1
kind: Namespace
metadata:
  name: only1mcp

---
# configmap.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: only1mcp-config
  namespace: only1mcp
data:
  only1mcp.yaml: |
    server:
      host: 0.0.0.0
      port: 8080
      workers: 0  # auto-detect

    servers:
      - id: github
        transport: http
        endpoint: https://api.github.com

    proxy:
      load_balancer:
        algorithm: round_robin

    cache:
      enabled: true
      max_size_mb: 1000
```

### Deployment
```yaml
# deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: only1mcp
  namespace: only1mcp
  labels:
    app: only1mcp
spec:
  replicas: 3
  selector:
    matchLabels:
      app: only1mcp
  template:
    metadata:
      labels:
        app: only1mcp
    spec:
      containers:
      - name: only1mcp
        image: only1mcp/only1mcp:latest
        ports:
        - containerPort: 8080
          name: http
        - containerPort: 9090
          name: admin
        - containerPort: 9091
          name: metrics
        env:
        - name: ONLY1MCP_CONFIG
          value: /config/only1mcp.yaml
        volumeMounts:
        - name: config
          mountPath: /config
          readOnly: true
        - name: cache
          mountPath: /cache
        resources:
          requests:
            memory: "512Mi"
            cpu: "500m"
          limits:
            memory: "2Gi"
            cpu: "2000m"
        livenessProbe:
          httpGet:
            path: /health/live
            port: 8080
          initialDelaySeconds: 10
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /health/ready
            port: 8080
          initialDelaySeconds: 5
          periodSeconds: 5
      volumes:
      - name: config
        configMap:
          name: only1mcp-config
      - name: cache
        emptyDir:
          sizeLimit: 1Gi
```

### Service and Ingress
```yaml
# service.yaml
apiVersion: v1
kind: Service
metadata:
  name: only1mcp
  namespace: only1mcp
spec:
  selector:
    app: only1mcp
  ports:
  - name: http
    port: 8080
    targetPort: 8080
  - name: admin
    port: 9090
    targetPort: 9090
  - name: metrics
    port: 9091
    targetPort: 9091
  type: ClusterIP

---
# ingress.yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: only1mcp
  namespace: only1mcp
  annotations:
    cert-manager.io/cluster-issuer: letsencrypt-prod
    nginx.ingress.kubernetes.io/proxy-body-size: "10m"
    nginx.ingress.kubernetes.io/proxy-read-timeout: "300"
spec:
  ingressClassName: nginx
  tls:
  - hosts:
    - mcp.example.com
    secretName: only1mcp-tls
  rules:
  - host: mcp.example.com
    http:
      paths:
      - path: /
        pathType: Prefix
        backend:
          service:
            name: only1mcp
            port:
              number: 8080
```

### Horizontal Pod Autoscaler
```yaml
# hpa.yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: only1mcp
  namespace: only1mcp
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: only1mcp
  minReplicas: 2
  maxReplicas: 10
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
  - type: Resource
    resource:
      name: memory
      target:
        type: Utilization
        averageUtilization: 80
```

## Cloud Deployments

### AWS Deployment

#### ECS Fargate
```json
{
  "family": "only1mcp",
  "networkMode": "awsvpc",
  "requiresCompatibilities": ["FARGATE"],
  "cpu": "1024",
  "memory": "2048",
  "containerDefinitions": [
    {
      "name": "only1mcp",
      "image": "only1mcp/only1mcp:latest",
      "portMappings": [
        {
          "containerPort": 8080,
          "protocol": "tcp"
        }
      ],
      "environment": [
        {
          "name": "ONLY1MCP_CONFIG",
          "value": "/config/only1mcp.yaml"
        }
      ],
      "mountPoints": [
        {
          "sourceVolume": "config",
          "containerPath": "/config"
        }
      ],
      "logConfiguration": {
        "logDriver": "awslogs",
        "options": {
          "awslogs-group": "/ecs/only1mcp",
          "awslogs-region": "us-east-1",
          "awslogs-stream-prefix": "ecs"
        }
      },
      "healthCheck": {
        "command": ["CMD-SHELL", "curl -f http://localhost:8080/health || exit 1"],
        "interval": 30,
        "timeout": 5,
        "retries": 3
      }
    }
  ],
  "volumes": [
    {
      "name": "config",
      "efsVolumeConfiguration": {
        "fileSystemId": "fs-12345678",
        "rootDirectory": "/config"
      }
    }
  ]
}
```

#### CloudFormation Template
```yaml
# cloudformation.yaml
AWSTemplateFormatVersion: '2010-09-09'
Description: 'Only1MCP Deployment Stack'

Parameters:
  VpcId:
    Type: AWS::EC2::VPC::Id
  SubnetIds:
    Type: List<AWS::EC2::Subnet::Id>
  CertificateArn:
    Type: String

Resources:
  LoadBalancer:
    Type: AWS::ElasticLoadBalancingV2::LoadBalancer
    Properties:
      Type: application
      Subnets: !Ref SubnetIds
      SecurityGroups:
        - !Ref SecurityGroup

  TargetGroup:
    Type: AWS::ElasticLoadBalancingV2::TargetGroup
    Properties:
      Port: 8080
      Protocol: HTTP
      TargetType: ip
      VpcId: !Ref VpcId
      HealthCheckPath: /health

  Listener:
    Type: AWS::ElasticLoadBalancingV2::Listener
    Properties:
      LoadBalancerArn: !Ref LoadBalancer
      Port: 443
      Protocol: HTTPS
      Certificates:
        - CertificateArn: !Ref CertificateArn
      DefaultActions:
        - Type: forward
          TargetGroupArn: !Ref TargetGroup

  SecurityGroup:
    Type: AWS::EC2::SecurityGroup
    Properties:
      GroupDescription: Only1MCP Security Group
      VpcId: !Ref VpcId
      SecurityGroupIngress:
        - IpProtocol: tcp
          FromPort: 443
          ToPort: 443
          CidrIp: 0.0.0.0/0
        - IpProtocol: tcp
          FromPort: 8080
          ToPort: 8080
          SourceSecurityGroupId: !Ref SecurityGroup
```

### Google Cloud Deployment

#### Cloud Run
```bash
# Build and push to Google Container Registry
gcloud builds submit --tag gcr.io/PROJECT_ID/only1mcp

# Deploy to Cloud Run
gcloud run deploy only1mcp \
    --image gcr.io/PROJECT_ID/only1mcp \
    --platform managed \
    --region us-central1 \
    --allow-unauthenticated \
    --port 8080 \
    --memory 2Gi \
    --cpu 2 \
    --max-instances 10 \
    --set-env-vars "ONLY1MCP_CONFIG=/config/only1mcp.yaml"
```

#### GKE Deployment
```bash
# Create GKE cluster
gcloud container clusters create only1mcp-cluster \
    --zone us-central1-a \
    --num-nodes 3 \
    --machine-type n1-standard-2 \
    --enable-autoscaling \
    --min-nodes 2 \
    --max-nodes 10

# Get credentials
gcloud container clusters get-credentials only1mcp-cluster \
    --zone us-central1-a

# Deploy application
kubectl apply -f kubernetes/
```

### Azure Deployment

#### Azure Container Instances
```bash
# Create resource group
az group create --name only1mcp-rg --location eastus

# Create container instance
az container create \
    --resource-group only1mcp-rg \
    --name only1mcp \
    --image only1mcp/only1mcp:latest \
    --ports 8080 9090 9091 \
    --cpu 2 \
    --memory 4 \
    --environment-variables \
        ONLY1MCP_CONFIG=/config/only1mcp.yaml \
    --azure-file-volume-account-name mystorageaccount \
    --azure-file-volume-account-key $STORAGE_KEY \
    --azure-file-volume-share-name config \
    --azure-file-volume-mount-path /config
```

#### AKS Deployment
```bash
# Create AKS cluster
az aks create \
    --resource-group only1mcp-rg \
    --name only1mcp-aks \
    --node-count 3 \
    --node-vm-size Standard_B2s \
    --enable-cluster-autoscaler \
    --min-count 2 \
    --max-count 10

# Get credentials
az aks get-credentials \
    --resource-group only1mcp-rg \
    --name only1mcp-aks

# Deploy application
kubectl apply -f kubernetes/
```

## Production Best Practices

### Configuration Management

1. **Use Environment Variables for Secrets**
```yaml
servers:
  - id: github
    auth:
      token_env: GITHUB_TOKEN  # Use env var
```

2. **Separate Config by Environment**
```bash
config/
├── base.yaml
├── development.yaml
├── staging.yaml
└── production.yaml
```

3. **Version Control Configuration**
```bash
git tag config-v1.2.3
git push --tags
```

### Deployment Checklist

- [ ] SSL/TLS certificates configured
- [ ] Authentication enabled
- [ ] Rate limiting configured
- [ ] Monitoring endpoints accessible
- [ ] Logging to persistent storage
- [ ] Backup strategy implemented
- [ ] Disaster recovery plan documented
- [ ] Load testing completed
- [ ] Security audit performed
- [ ] Documentation updated

## Security Hardening

### Network Security

1. **Firewall Rules**
```bash
# Allow only required ports
ufw allow 8080/tcp comment 'Only1MCP HTTP'
ufw allow 9091/tcp from 10.0.0.0/8 comment 'Metrics internal only'
ufw enable
```

2. **TLS Configuration**
```yaml
server:
  tls:
    enabled: true
    cert_path: /etc/ssl/certs/only1mcp.crt
    key_path: /etc/ssl/private/only1mcp.key
    min_version: "1.3"
    cipher_suites:
      - TLS_AES_256_GCM_SHA384
      - TLS_CHACHA20_POLY1305_SHA256
```

3. **mTLS for Service-to-Service**
```yaml
server:
  tls:
    client_auth:
      enabled: true
      ca_path: /etc/ssl/certs/ca-bundle.crt
      required: true
```

### Authentication & Authorization

1. **OAuth2 Configuration**
```yaml
auth:
  providers:
    - type: oauth2
      issuer: https://auth.example.com
      client_id: ${OAUTH_CLIENT_ID}
      client_secret: ${OAUTH_CLIENT_SECRET}
```

2. **API Key Rotation**
```bash
# Generate new API key
openssl rand -hex 32

# Update configuration
only1mcp server update-key --old-key $OLD_KEY --new-key $NEW_KEY
```

3. **RBAC Configuration**
```yaml
authorization:
  roles:
    - name: admin
      permissions: ["*"]
    - name: developer
      permissions: ["tools:*", "resources:read"]
    - name: viewer
      permissions: ["tools:read", "resources:read"]
```

## Performance Tuning

### System Tuning

```bash
# /etc/sysctl.d/99-only1mcp.conf
net.core.somaxconn = 65535
net.ipv4.tcp_max_syn_backlog = 8192
net.core.netdev_max_backlog = 65536
net.ipv4.ip_local_port_range = 1024 65535
net.ipv4.tcp_tw_reuse = 1
net.ipv4.tcp_fin_timeout = 30
fs.file-max = 2097152
fs.nr_open = 2097152
```

### Proxy Configuration

```yaml
proxy:
  worker_threads: 0  # Auto-detect CPU cores
  max_connections: 100000

  connection_pool:
    max_per_backend: 500
    min_idle: 10
    max_idle_time_ms: 300000

  load_balancer:
    algorithm: least_connections

cache:
  enabled: true
  max_size_mb: 10000
  backend: redis  # Use Redis for distributed cache
```

### Resource Limits

```yaml
# Kubernetes resources
resources:
  requests:
    memory: "2Gi"
    cpu: "1000m"
  limits:
    memory: "8Gi"
    cpu: "4000m"
```

## High Availability

### Active-Active Setup

```yaml
# haproxy.cfg
global
    maxconn 100000
    log stdout local0

defaults
    mode http
    timeout connect 5s
    timeout client 30s
    timeout server 30s
    option httplog

frontend only1mcp_frontend
    bind *:80
    bind *:443 ssl crt /etc/ssl/certs/only1mcp.pem
    redirect scheme https if !{ ssl_fc }
    default_backend only1mcp_backend

backend only1mcp_backend
    balance leastconn
    option httpchk GET /health
    server only1mcp1 10.0.1.10:8080 check
    server only1mcp2 10.0.1.11:8080 check
    server only1mcp3 10.0.1.12:8080 check
```

### Database Replication

```yaml
# Redis Sentinel configuration
sentinel monitor mymaster 10.0.1.20 6379 2
sentinel down-after-milliseconds mymaster 5000
sentinel parallel-syncs mymaster 1
sentinel failover-timeout mymaster 10000
```

### Disaster Recovery

```bash
#!/bin/bash
# backup.sh

# Backup configuration
tar -czf /backup/config-$(date +%Y%m%d).tar.gz /etc/only1mcp/

# Backup cache data
redis-cli --rdb /backup/redis-$(date +%Y%m%d).rdb

# Upload to S3
aws s3 cp /backup/ s3://only1mcp-backups/ --recursive

# Cleanup old backups
find /backup -type f -mtime +30 -delete
```

## Monitoring Setup

### Prometheus Configuration

```yaml
# prometheus.yml
scrape_configs:
  - job_name: 'only1mcp'
    static_configs:
      - targets: ['only1mcp:9091']
    metrics_path: '/metrics'
    scrape_interval: 15s
```

### Grafana Dashboard

Import dashboard ID: `15000` or use custom dashboard:

```json
{
  "dashboard": {
    "title": "Only1MCP Monitoring",
    "panels": [
      {
        "title": "Request Rate",
        "targets": [
          {
            "expr": "rate(only1mcp_requests_total[5m])"
          }
        ]
      },
      {
        "title": "Latency P99",
        "targets": [
          {
            "expr": "histogram_quantile(0.99, only1mcp_request_duration_seconds)"
          }
        ]
      }
    ]
  }
}
```

### Alerting Rules

```yaml
# alerts.yml
groups:
  - name: only1mcp
    rules:
      - alert: HighErrorRate
        expr: rate(only1mcp_errors_total[5m]) > 0.05
        for: 5m
        annotations:
          summary: "High error rate detected"

      - alert: HighLatency
        expr: histogram_quantile(0.99, only1mcp_request_duration_seconds) > 1
        for: 5m
        annotations:
          summary: "P99 latency above 1 second"
```

## Backup and Recovery

### Automated Backups

```bash
# /etc/cron.d/only1mcp-backup
0 2 * * * root /usr/local/bin/only1mcp-backup.sh
```

### Recovery Procedure

1. **Configuration Recovery**
```bash
# Restore configuration
tar -xzf /backup/config-20240101.tar.gz -C /

# Verify configuration
only1mcp validate /etc/only1mcp/config.yaml
```

2. **Data Recovery**
```bash
# Restore Redis cache
redis-cli --rdb /backup/redis-20240101.rdb
redis-cli FLUSHDB
redis-cli --rdb /backup/redis-20240101.rdb RESTORE
```

3. **Service Recovery**
```bash
# Restart services
systemctl restart only1mcp
systemctl status only1mcp
```

## Troubleshooting

### Common Issues

1. **Port Already in Use**
```bash
# Find process using port
lsof -i :8080
# or
netstat -tulpn | grep :8080
```

2. **Permission Denied**
```bash
# Fix permissions
chown -R only1mcp:only1mcp /var/lib/only1mcp
chmod 755 /var/lib/only1mcp
```

3. **Out of Memory**
```bash
# Check memory usage
free -h
ps aux | grep only1mcp

# Increase limits
systemctl edit only1mcp
# Add: LimitNOFILE=65535
```

### Debug Mode

```bash
# Enable debug logging
RUST_LOG=debug only1mcp start --log-level debug

# Enable backtrace
RUST_BACKTRACE=full only1mcp start

# Profile performance
only1mcp start --profile
```

### Health Checks

```bash
# Check proxy health
curl http://localhost:8080/health

# Check specific server
only1mcp health check github

# Full diagnostic
only1mcp test --suite all
```

## Appendix

### Systemd Service File

```ini
[Unit]
Description=Only1MCP Proxy Server
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=only1mcp
Group=only1mcp
WorkingDirectory=/var/lib/only1mcp

Environment="RUST_LOG=info"
ExecStartPre=/usr/local/bin/only1mcp validate /etc/only1mcp/config.yaml
ExecStart=/usr/local/bin/only1mcp start --config /etc/only1mcp/config.yaml
ExecReload=/bin/kill -HUP $MAINPID

Restart=always
RestartSec=10
KillMode=mixed
KillSignal=SIGTERM

# Hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/only1mcp /var/log/only1mcp

# Limits
LimitNOFILE=65535
LimitNPROC=4096

[Install]
WantedBy=multi-user.target
```

### Example Configurations

Configuration templates are available in the repository:
- `config/templates/solo.yaml` - Single developer setup
- `config/templates/team.yaml` - Small team deployment
- `config/templates/enterprise.yaml` - Enterprise production

### Support

For deployment assistance:
- Documentation: https://docs.only1mcp.io
- GitHub Issues: https://github.com/doublegate/Only1MCP/issues
- Discord: https://discord.gg/only1mcp