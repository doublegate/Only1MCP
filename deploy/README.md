# Only1MCP Deployment Guide

This directory contains Infrastructure as Code (IaC) templates and configurations for deploying Only1MCP in various environments.

## Table of Contents

- [Docker Compose (Local Development)](#docker-compose-local-development)
- [Kubernetes (Production)](#kubernetes-production)
- [Terraform (AWS)](#terraform-aws)
- [CI/CD Integration](#cicd-integration)
- [Monitoring Setup](#monitoring-setup)

---

## Docker Compose (Local Development)

The Docker Compose setup provides a complete local development environment with all dependencies.

### Quick Start

```bash
# Start all services
docker-compose up -d

# View logs
docker-compose logs -f only1mcp

# Stop all services
docker-compose down

# Stop and remove volumes
docker-compose down -v
```

### Services Included

- **only1mcp**: Main proxy server (port 8080)
- **prometheus**: Metrics collection (port 9091)
- **grafana**: Dashboards and visualization (port 3000)
- **redis**: Caching and rate limiting (port 6379)
- **mcp-filesystem**: Example MCP server for filesystem operations
- **mcp-github**: Example MCP server for GitHub integration
- **nginx**: Reverse proxy (optional, port 80/443)
- **jaeger**: Distributed tracing (optional, port 16686)

### Configuration

Edit `docker-compose.yml` to customize:

- Ports and exposed services
- Environment variables
- Volume mounts
- Resource limits

### Profiles

Use Docker Compose profiles for optional services:

```bash
# Start with Nginx reverse proxy
docker-compose --profile production up -d

# Start with full observability stack
docker-compose --profile observability up -d
```

### Environment Variables

Create a `.env` file in the `deploy/` directory:

```env
GITHUB_TOKEN=your_github_token_here
RUST_LOG=info
```

### Accessing Services

- **Only1MCP Proxy**: http://localhost:8080
- **Prometheus**: http://localhost:9091
- **Grafana**: http://localhost:3000 (admin/admin)
- **Jaeger UI**: http://localhost:16686

---

## Kubernetes (Production)

Production-ready Kubernetes deployment with high availability, auto-scaling, and monitoring.

### Prerequisites

- Kubernetes cluster (1.24+)
- kubectl configured
- Helm (optional, for advanced deployments)

### Quick Deploy

```bash
# Create namespace
kubectl create namespace only1mcp

# Apply all manifests
kubectl apply -f kubernetes/deployment.yaml

# Check deployment status
kubectl -n only1mcp get all

# View logs
kubectl -n only1mcp logs -f deployment/only1mcp
```

### Configuration

The deployment includes:

- **Namespace**: Isolated environment
- **ConfigMap**: Application configuration
- **Secret**: Sensitive data (JWT tokens, API keys)
- **Deployment**: 3 replicas with rolling updates
- **Service**: ClusterIP for internal access
- **Ingress**: External access with TLS
- **HorizontalPodAutoscaler**: Auto-scaling (3-10 pods)
- **PodDisruptionBudget**: High availability (min 2 pods)
- **ServiceMonitor**: Prometheus integration

### Customization

Edit `kubernetes/deployment.yaml`:

```yaml
# Update image
image: ghcr.io/doublegate/only1mcp:v0.5.0

# Adjust resources
resources:
  requests:
    cpu: 500m
    memory: 512Mi
  limits:
    cpu: 2000m
    memory: 2Gi

# Configure auto-scaling
spec:
  minReplicas: 3
  maxReplicas: 10
  metrics:
    - type: Resource
      resource:
        name: cpu
        target:
          type: Utilization
          averageUtilization: 70
```

### Ingress Configuration

Update Ingress with your domain:

```yaml
spec:
  rules:
    - host: only1mcp.example.com  # Change this
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

### TLS Certificate

Create TLS secret:

```bash
kubectl create secret tls only1mcp-tls \
  --cert=path/to/cert.pem \
  --key=path/to/key.pem \
  -n only1mcp
```

Or use cert-manager for automatic certificates:

```yaml
apiVersion: cert-manager.io/v1
kind: Certificate
metadata:
  name: only1mcp-cert
  namespace: only1mcp
spec:
  secretName: only1mcp-tls
  issuerRef:
    name: letsencrypt-prod
    kind: ClusterIssuer
  dnsNames:
    - only1mcp.example.com
```

### Monitoring

The ServiceMonitor enables Prometheus to scrape metrics:

```bash
# Check Prometheus targets
kubectl -n monitoring get servicemonitor only1mcp

# View metrics
curl http://only1mcp-service:9090/metrics
```

---

## Terraform (AWS)

Enterprise-grade AWS infrastructure using ECS Fargate, Application Load Balancer, and managed services.

### Architecture

- **VPC**: Multi-AZ with public and private subnets
- **ECS Fargate**: Serverless container orchestration
- **Application Load Balancer**: HTTPS traffic distribution
- **ElastiCache Redis**: Caching and rate limiting
- **RDS** (optional): Persistent database
- **CloudWatch**: Centralized logging and monitoring
- **S3**: Configuration and backup storage
- **Secrets Manager**: Secure credential storage

### Prerequisites

- Terraform >= 1.0
- AWS CLI configured
- SSL certificate in ACM (for HTTPS)

### Setup

1. **Initialize Terraform**:

```bash
cd terraform
terraform init
```

2. **Create `terraform.tfvars`**:

```hcl
environment          = "production"
aws_region          = "us-east-1"
container_image     = "ghcr.io/doublegate/only1mcp:latest"
desired_count       = 3

# SSL certificate ARN from ACM
ssl_certificate_arn = "arn:aws:acm:us-east-1:123456789012:certificate/..."

# Secrets
jwt_secret          = "your-secure-jwt-secret"
db_password         = "your-secure-db-password"

# Monitoring
alarm_email         = "ops@example.com"

# VPC Configuration
vpc_cidr            = "10.0.0.0/16"
availability_zones  = ["us-east-1a", "us-east-1b", "us-east-1c"]
```

3. **Plan and Apply**:

```bash
# Review changes
terraform plan

# Apply infrastructure
terraform apply

# View outputs
terraform output
```

### Cost Optimization

```hcl
# Development environment
task_cpu         = 512
task_memory      = 1024
desired_count    = 1
min_capacity     = 1
max_capacity     = 3
redis_node_type  = "cache.t3.micro"
enable_rds       = false
```

```hcl
# Production environment
task_cpu         = 2048
task_memory      = 4096
desired_count    = 3
min_capacity     = 2
max_capacity     = 10
redis_node_type  = "cache.r6g.large"
enable_rds       = true
rds_instance_class = "db.r6g.xlarge"
```

### Terraform Modules

The deployment is modularized for reusability:

- `modules/vpc`: VPC and networking
- `modules/security`: Security groups and IAM
- `modules/alb`: Application Load Balancer
- `modules/ecs`: ECS cluster and services
- `modules/redis`: ElastiCache Redis
- `modules/rds`: RDS database (optional)
- `modules/cloudwatch`: Monitoring and alarms

### State Management

Configure S3 backend for state storage:

```bash
# Create S3 bucket
aws s3 mb s3://only1mcp-terraform-state

# Enable versioning
aws s3api put-bucket-versioning \
  --bucket only1mcp-terraform-state \
  --versioning-configuration Status=Enabled

# Create DynamoDB table for locking
aws dynamodb create-table \
  --table-name only1mcp-terraform-locks \
  --attribute-definitions AttributeName=LockID,AttributeType=S \
  --key-schema AttributeName=LockID,KeyType=HASH \
  --billing-mode PAY_PER_REQUEST
```

### Outputs

After deployment, Terraform provides:

```
alb_dns_name = "only1mcp-alb-1234567890.us-east-1.elb.amazonaws.com"
ecs_cluster_name = "production-only1mcp"
redis_endpoint = "production-only1mcp.abcdef.0001.use1.cache.amazonaws.com:6379"
rds_endpoint = "production-only1mcp.abcdef.us-east-1.rds.amazonaws.com:5432"
```

### Cleanup

```bash
# Destroy all resources
terraform destroy

# Destroy specific modules
terraform destroy -target=module.rds
```

---

## CI/CD Integration

The GitHub Actions workflow automatically deploys on code changes.

### Workflow Triggers

- **Push to main**: Deploy to production
- **Push to develop**: Deploy to staging
- **Pull requests**: Run tests and build
- **Tags (v*)**: Create release

### Deployment Process

1. **Build**: Multi-platform binaries (Linux, macOS, Windows)
2. **Test**: Run all tests and security audits
3. **Docker**: Build and push container images
4. **Deploy**: Update Kubernetes/ECS deployment

### Environment Secrets

Configure in GitHub repository settings:

- `AWS_ACCESS_KEY_ID`
- `AWS_SECRET_ACCESS_KEY`
- `KUBECONFIG` (for Kubernetes deployments)
- `JWT_SECRET`
- `DB_PASSWORD`

### Manual Deployment

```bash
# Trigger manual deployment
gh workflow run ci.yml -f environment=production

# Check workflow status
gh run list --workflow=ci.yml
```

---

## Monitoring Setup

### Prometheus

Metrics exposed at `/metrics` endpoint:

- Request rate and latency
- Error rate
- Cache hit rate
- Backend health
- System resources (CPU, memory)

### Grafana Dashboards

Import pre-built dashboards:

1. Login to Grafana (http://localhost:3000)
2. Go to Dashboards → Import
3. Use dashboard ID or upload JSON from `grafana/dashboards/`

**Available Dashboards**:
- Overview: High-level system metrics
- Performance: Request latency and throughput
- Errors: Error rates and types
- Resources: CPU, memory, network

### Alerts

Configure alerts in `prometheus.yml`:

```yaml
rule_files:
  - "alerts/high_error_rate.yml"
  - "alerts/high_latency.yml"
  - "alerts/backend_down.yml"
```

Example alert:

```yaml
groups:
  - name: only1mcp
    rules:
      - alert: HighErrorRate
        expr: rate(only1mcp_errors_total[5m]) > 0.05
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "High error rate detected"
          description: "Error rate is {{ $value }} errors/sec"
```

### CloudWatch (AWS)

For Terraform deployments, CloudWatch alarms are automatically configured:

- High CPU utilization (>80%)
- High memory utilization (>80%)
- High error rate (>5%)
- Low healthy host count (<2)

Alarms send notifications to the configured SNS topic.

---

## Troubleshooting

### Docker Compose

```bash
# Container won't start
docker-compose logs only1mcp
docker-compose ps

# Reset everything
docker-compose down -v
docker-compose up -d
```

### Kubernetes

```bash
# Pod not starting
kubectl -n only1mcp describe pod <pod-name>
kubectl -n only1mcp logs <pod-name>

# Check events
kubectl -n only1mcp get events --sort-by='.lastTimestamp'

# Debug with shell
kubectl -n only1mcp exec -it <pod-name> -- sh
```

### Terraform

```bash
# Plan failed
terraform plan -detailed-exitcode

# Apply failed mid-way
terraform refresh
terraform apply

# Inspect state
terraform state list
terraform state show <resource>
```

---

## Best Practices

### Security

- Use secrets management (Secrets Manager, Vault)
- Enable encryption at rest and in transit
- Implement least privilege IAM policies
- Regular security audits and updates

### High Availability

- Deploy across multiple availability zones
- Use auto-scaling for elasticity
- Configure health checks and circuit breakers
- Implement proper monitoring and alerting

### Cost Optimization

- Right-size resources based on metrics
- Use spot instances for non-critical workloads
- Enable auto-scaling to match demand
- Clean up unused resources regularly

### Backup and Disaster Recovery

- Regular database backups
- Configuration stored in version control
- Infrastructure as Code for reproducibility
- Documented recovery procedures

---

## Additional Resources

- [Kubernetes Documentation](https://kubernetes.io/docs/)
- [Terraform AWS Provider](https://registry.terraform.io/providers/hashicorp/aws/latest/docs)
- [Docker Compose Reference](https://docs.docker.com/compose/compose-file/)
- [Prometheus Best Practices](https://prometheus.io/docs/practices/)

## Support

For issues and questions:
- GitHub Issues: https://github.com/doublegate/Only1MCP/issues
- Documentation: ../docs/
- Examples: ../examples/
