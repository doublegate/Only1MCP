# Only1MCP Monitoring & Observability Guide

Comprehensive guide for monitoring, observability, and alerting for Only1MCP deployments.

## Table of Contents
- [Overview](#overview)
- [Metrics](#metrics)
  - [Core Metrics](#core-metrics)
  - [Custom Metrics](#custom-metrics)
  - [Prometheus Setup](#prometheus-setup)
- [Logging](#logging)
  - [Log Levels](#log-levels)
  - [Structured Logging](#structured-logging)
  - [Log Aggregation](#log-aggregation)
- [Tracing](#tracing)
  - [OpenTelemetry](#opentelemetry)
  - [Jaeger Setup](#jaeger-setup)
  - [Distributed Tracing](#distributed-tracing)
- [Health Checks](#health-checks)
- [Dashboards](#dashboards)
  - [Grafana Setup](#grafana-setup)
  - [Dashboard Templates](#dashboard-templates)
- [Alerting](#alerting)
  - [Alert Rules](#alert-rules)
  - [Notification Channels](#notification-channels)
- [APM Integration](#apm-integration)
- [SLA Monitoring](#sla-monitoring)
- [Troubleshooting](#troubleshooting)

## Overview

Only1MCP provides comprehensive observability through:
- **Metrics**: Prometheus-compatible metrics endpoint
- **Logging**: Structured JSON logging with multiple outputs
- **Tracing**: OpenTelemetry-compatible distributed tracing
- **Health Checks**: Kubernetes-compatible health endpoints

### Architecture

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│   Only1MCP   │────▶│  Prometheus  │────▶│   Grafana    │
│   Metrics    │     └──────────────┘     └──────────────┘
└──────────────┘              │                    │
       │                      ▼                    │
       │              ┌──────────────┐            │
       │              │ Alert Manager│────────────┘
       │              └──────────────┘
       │
┌──────────────┐     ┌──────────────┐
│   Only1MCP   │────▶│   Loki/ELK   │
│     Logs     │     └──────────────┘
└──────────────┘
       │
┌──────────────┐     ┌──────────────┐
│   Only1MCP   │────▶│    Jaeger    │
│    Traces    │     └──────────────┘
└──────────────┘
```

## Metrics

### Core Metrics

Only1MCP exposes metrics at `/metrics` endpoint (default port 9091).

#### Request Metrics

| Metric | Type | Description | Labels |
|--------|------|-------------|--------|
| `only1mcp_requests_total` | Counter | Total number of requests | method, status, server |
| `only1mcp_request_duration_seconds` | Histogram | Request duration | method, status, server |
| `only1mcp_request_size_bytes` | Histogram | Request body size | method |
| `only1mcp_response_size_bytes` | Histogram | Response body size | method |
| `only1mcp_active_requests` | Gauge | Currently active requests | method |

#### Server Metrics

| Metric | Type | Description | Labels |
|--------|------|-------------|--------|
| `only1mcp_backend_up` | Gauge | Backend server health (0/1) | server_id |
| `only1mcp_backend_latency_seconds` | Histogram | Backend response time | server_id, method |
| `only1mcp_backend_errors_total` | Counter | Backend error count | server_id, error_type |
| `only1mcp_circuit_breaker_state` | Gauge | Circuit breaker state | server_id, state |

#### Cache Metrics

| Metric | Type | Description | Labels |
|--------|------|-------------|--------|
| `only1mcp_cache_hits_total` | Counter | Cache hit count | cache_level |
| `only1mcp_cache_misses_total` | Counter | Cache miss count | cache_level |
| `only1mcp_cache_size_bytes` | Gauge | Current cache size | cache_level |
| `only1mcp_cache_evictions_total` | Counter | Cache evictions | cache_level, reason |

#### System Metrics

| Metric | Type | Description | Labels |
|--------|------|-------------|--------|
| `only1mcp_cpu_usage_percent` | Gauge | CPU usage percentage | - |
| `only1mcp_memory_usage_bytes` | Gauge | Memory usage | type |
| `only1mcp_goroutines` | Gauge | Number of goroutines | - |
| `only1mcp_open_connections` | Gauge | Open connections | type |

### Custom Metrics

Define custom metrics in configuration:

```yaml
monitoring:
  custom_metrics:
    - name: tool_usage
      type: counter
      help: "Tool usage by name"
      labels: [tool_name, user]

    - name: auth_attempts
      type: histogram
      help: "Authentication attempt duration"
      labels: [method, result]
      buckets: [0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0]
```

### Prometheus Setup

#### Configuration

```yaml
# prometheus.yml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

scrape_configs:
  - job_name: 'only1mcp'
    static_configs:
      - targets: ['localhost:9091']
        labels:
          environment: 'production'
          region: 'us-east-1'

    metrics_path: '/metrics'
    scrape_timeout: 10s

    # TLS configuration
    tls_config:
      ca_file: /etc/prometheus/ca.crt
      cert_file: /etc/prometheus/client.crt
      key_file: /etc/prometheus/client.key

  - job_name: 'only1mcp_federation'
    honor_labels: true
    metrics_path: '/federate'
    params:
      'match[]':
        - '{job="only1mcp"}'
    static_configs:
      - targets:
        - 'prometheus-region1:9090'
        - 'prometheus-region2:9090'
```

#### Recording Rules

```yaml
# rules.yml
groups:
  - name: only1mcp_aggregation
    interval: 30s
    rules:
      - record: only1mcp:request_rate5m
        expr: rate(only1mcp_requests_total[5m])

      - record: only1mcp:error_rate5m
        expr: rate(only1mcp_requests_total{status=~"5.."}[5m])

      - record: only1mcp:p99_latency5m
        expr: histogram_quantile(0.99, rate(only1mcp_request_duration_seconds_bucket[5m]))

      - record: only1mcp:cache_hit_ratio5m
        expr: |
          rate(only1mcp_cache_hits_total[5m]) /
          (rate(only1mcp_cache_hits_total[5m]) + rate(only1mcp_cache_misses_total[5m]))
```

## Logging

### Log Levels

Configure log levels per module:

```yaml
logging:
  level: info  # Global level

  modules:
    only1mcp::proxy: debug
    only1mcp::auth: warn
    only1mcp::cache: info
    only1mcp::transport: trace
```

### Structured Logging

All logs are emitted as structured JSON:

```json
{
  "timestamp": "2024-01-15T10:30:45.123Z",
  "level": "INFO",
  "message": "Request processed",
  "fields": {
    "request_id": "abc123",
    "method": "tools/call",
    "duration_ms": 45,
    "server_id": "github",
    "status": 200
  },
  "target": "only1mcp::proxy::handler",
  "span": {
    "trace_id": "1234567890abcdef",
    "span_id": "abcdef123456"
  }
}
```

### Log Aggregation

#### Elasticsearch/Logstash/Kibana (ELK)

```yaml
# filebeat.yml
filebeat.inputs:
- type: log
  enabled: true
  paths:
    - /var/log/only1mcp/*.log
  json.keys_under_root: true
  json.add_error_key: true

  processors:
    - add_kubernetes_metadata:
        host: ${NODE_NAME}
        matchers:
        - logs_path:
            logs_path: "/var/log/containers/"

output.logstash:
  hosts: ["logstash:5044"]
  ssl.certificate_authorities: ["/etc/pki/tls/certs/ca-bundle.crt"]
```

#### Loki/Promtail

```yaml
# promtail.yml
clients:
  - url: http://loki:3100/loki/api/v1/push

scrape_configs:
  - job_name: only1mcp
    static_configs:
      - targets:
          - localhost
        labels:
          job: only1mcp
          __path__: /var/log/only1mcp/*.log

    pipeline_stages:
      - json:
          expressions:
            level: level
            message: message
            request_id: fields.request_id
            duration: fields.duration_ms

      - labels:
          level:
          request_id:

      - metrics:
          log_lines_total:
            type: Counter
            description: "Total log lines"
            source: level
            config:
              action: inc
```

## Tracing

### OpenTelemetry

#### Configuration

```yaml
monitoring:
  tracing:
    enabled: true

    exporter:
      type: otlp
      endpoint: "http://otel-collector:4317"

    sampler:
      type: probability
      rate: 0.01  # Sample 1% of traces

      # Adaptive sampling
      adaptive:
        max_traces_per_second: 100

    propagation:
      - w3c
      - jaeger
      - baggage
```

#### Instrumentation

```rust
#[instrument(skip(state))]
async fn handle_request(
    state: AppState,
    request: Request,
) -> Result<Response> {
    let span = tracing::info_span!(
        "handle_request",
        request_id = %request.id,
        method = %request.method,
    );

    async move {
        // Process request
        info!("Processing request");
        let result = process(request).await?;
        info!("Request completed");
        Ok(result)
    }
    .instrument(span)
    .await
}
```

### Jaeger Setup

```yaml
# docker-compose.yml
services:
  jaeger:
    image: jaegertracing/all-in-one:latest
    ports:
      - "6831:6831/udp"  # Thrift compact
      - "6832:6832/udp"  # Thrift binary
      - "16686:16686"     # Web UI
      - "14268:14268"     # HTTP collector
    environment:
      - COLLECTOR_ZIPKIN_HTTP_PORT=9411
      - SPAN_STORAGE_TYPE=elasticsearch
      - ES_SERVER_URLS=http://elasticsearch:9200
```

### Distributed Tracing

Enable trace context propagation:

```yaml
servers:
  - id: backend1
    propagate_trace_context: true
    headers:
      - traceparent
      - tracestate
      - baggage
```

## Health Checks

### Endpoints

| Endpoint | Description | Response |
|----------|-------------|----------|
| `/health/live` | Liveness check | 200 if alive |
| `/health/ready` | Readiness check | 200 if ready to serve |
| `/health/startup` | Startup check | 200 when initialized |

### Health Check Response

```json
{
  "status": "healthy",
  "timestamp": "2024-01-15T10:30:45Z",
  "version": "1.0.0",
  "uptime_seconds": 3600,
  "checks": {
    "database": {
      "status": "healthy",
      "latency_ms": 2
    },
    "cache": {
      "status": "healthy",
      "size_mb": 245
    },
    "backends": {
      "status": "degraded",
      "healthy": 4,
      "unhealthy": 1,
      "details": {
        "github": "healthy",
        "openai": "healthy",
        "anthropic": "healthy",
        "filesystem": "healthy",
        "database": "unhealthy"
      }
    }
  }
}
```

### Configuration

```yaml
monitoring:
  health:
    enabled: true

    endpoints:
      liveness: /health/live
      readiness: /health/ready
      startup: /health/startup

    checks:
      - name: database
        type: tcp
        target: "postgres:5432"
        timeout: 5s
        critical: true

      - name: redis
        type: redis
        target: "redis:6379"
        timeout: 3s
        critical: false

      - name: backends
        type: custom
        script: /usr/local/bin/check-backends.sh
        timeout: 10s
```

## Dashboards

### Grafana Setup

```yaml
# docker-compose.yml
services:
  grafana:
    image: grafana/grafana:latest
    ports:
      - "3000:3000"
    environment:
      - GF_SECURITY_ADMIN_PASSWORD=admin
      - GF_INSTALL_PLUGINS=redis-datasource,vertamedia-clickhouse-datasource
    volumes:
      - ./grafana/provisioning:/etc/grafana/provisioning
      - grafana-storage:/var/lib/grafana
```

### Dashboard Templates

#### Overview Dashboard

```json
{
  "dashboard": {
    "title": "Only1MCP Overview",
    "panels": [
      {
        "title": "Request Rate",
        "gridPos": {"h": 8, "w": 12, "x": 0, "y": 0},
        "targets": [{
          "expr": "sum(rate(only1mcp_requests_total[5m])) by (method)"
        }],
        "type": "graph",
        "yaxes": [{"format": "reqps"}]
      },
      {
        "title": "Error Rate",
        "gridPos": {"h": 8, "w": 12, "x": 12, "y": 0},
        "targets": [{
          "expr": "sum(rate(only1mcp_requests_total{status=~\"5..\"}[5m])) by (method)"
        }],
        "type": "graph",
        "alert": {
          "conditions": [{
            "evaluator": {"params": [0.01], "type": "gt"},
            "operator": {"type": "and"},
            "query": {"params": ["A", "5m", "now"]},
            "reducer": {"params": [], "type": "avg"},
            "type": "query"
          }]
        }
      },
      {
        "title": "P50/P95/P99 Latency",
        "gridPos": {"h": 8, "w": 24, "x": 0, "y": 8},
        "targets": [
          {
            "expr": "histogram_quantile(0.5, sum(rate(only1mcp_request_duration_seconds_bucket[5m])) by (le))",
            "legendFormat": "P50"
          },
          {
            "expr": "histogram_quantile(0.95, sum(rate(only1mcp_request_duration_seconds_bucket[5m])) by (le))",
            "legendFormat": "P95"
          },
          {
            "expr": "histogram_quantile(0.99, sum(rate(only1mcp_request_duration_seconds_bucket[5m])) by (le))",
            "legendFormat": "P99"
          }
        ],
        "type": "graph",
        "yaxes": [{"format": "s"}]
      }
    ]
  }
}
```

#### Backend Health Dashboard

```json
{
  "dashboard": {
    "title": "Backend Health",
    "panels": [
      {
        "title": "Backend Status",
        "type": "stat",
        "targets": [{
          "expr": "only1mcp_backend_up"
        }],
        "fieldConfig": {
          "defaults": {
            "mappings": [
              {"type": "value", "value": 0, "text": "Down", "color": "red"},
              {"type": "value", "value": 1, "text": "Up", "color": "green"}
            ]
          }
        }
      },
      {
        "title": "Circuit Breaker Status",
        "type": "table",
        "targets": [{
          "expr": "only1mcp_circuit_breaker_state",
          "format": "table",
          "instant": true
        }],
        "transformations": [
          {
            "id": "organize",
            "options": {
              "excludeByName": {"Time": true},
              "renameByName": {
                "server_id": "Server",
                "state": "State",
                "Value": "Status"
              }
            }
          }
        ]
      }
    ]
  }
}
```

## Alerting

### Alert Rules

```yaml
# alerts.yml
groups:
  - name: only1mcp_alerts
    interval: 30s
    rules:
      # High error rate
      - alert: HighErrorRate
        expr: |
          (
            sum(rate(only1mcp_requests_total{status=~"5.."}[5m]))
            /
            sum(rate(only1mcp_requests_total[5m]))
          ) > 0.05
        for: 5m
        labels:
          severity: critical
          team: platform
        annotations:
          summary: "High error rate detected ({{ $value | humanizePercentage }})"
          description: "Error rate is above 5% for the last 5 minutes"
          runbook_url: "https://wiki.example.com/runbooks/only1mcp-high-error-rate"

      # High latency
      - alert: HighLatency
        expr: |
          histogram_quantile(0.99,
            sum(rate(only1mcp_request_duration_seconds_bucket[5m])) by (le)
          ) > 1
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High P99 latency ({{ $value | humanizeDuration }})"

      # Backend down
      - alert: BackendDown
        expr: only1mcp_backend_up == 0
        for: 2m
        labels:
          severity: critical
        annotations:
          summary: "Backend {{ $labels.server_id }} is down"

      # Cache hit rate low
      - alert: LowCacheHitRate
        expr: |
          rate(only1mcp_cache_hits_total[5m])
          /
          (rate(only1mcp_cache_hits_total[5m]) + rate(only1mcp_cache_misses_total[5m]))
          < 0.5
        for: 15m
        labels:
          severity: info
        annotations:
          summary: "Cache hit rate below 50%"

      # Memory usage high
      - alert: HighMemoryUsage
        expr: |
          only1mcp_memory_usage_bytes / only1mcp_memory_limit_bytes > 0.9
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Memory usage above 90%"

      # Too many open connections
      - alert: TooManyConnections
        expr: only1mcp_open_connections > 10000
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Too many open connections ({{ $value }})"
```

### Notification Channels

#### Alertmanager Configuration

```yaml
# alertmanager.yml
global:
  smtp_smarthost: 'smtp.example.com:587'
  smtp_from: 'alertmanager@example.com'
  smtp_auth_username: 'alertmanager@example.com'
  smtp_auth_password: 'password'

route:
  group_by: ['alertname', 'cluster', 'service']
  group_wait: 10s
  group_interval: 10s
  repeat_interval: 1h
  receiver: 'team-platform'

  routes:
    - match:
        severity: critical
      receiver: 'pagerduty'
      continue: true

    - match:
        severity: warning
      receiver: 'slack'

receivers:
  - name: 'team-platform'
    email_configs:
      - to: 'platform-team@example.com'

  - name: 'slack'
    slack_configs:
      - api_url: 'YOUR_SLACK_WEBHOOK_URL'
        channel: '#alerts'
        title: 'Only1MCP Alert'
        text: '{{ range .Alerts }}{{ .Annotations.summary }}{{ end }}'

  - name: 'pagerduty'
    pagerduty_configs:
      - service_key: 'YOUR_PAGERDUTY_SERVICE_KEY'
        description: '{{ .GroupLabels.alertname }}'
```

## APM Integration

### DataDog Integration

```yaml
monitoring:
  apm:
    provider: datadog

    datadog:
      agent_host: localhost
      agent_port: 8126
      service_name: only1mcp
      env: production
      version: "1.0.0"

      tags:
        - "region:us-east-1"
        - "team:platform"

      sampling:
        rate: 0.1
```

### New Relic Integration

```yaml
monitoring:
  apm:
    provider: newrelic

    newrelic:
      app_name: "Only1MCP Production"
      license_key: ${NEW_RELIC_LICENSE_KEY}

      distributed_tracing:
        enabled: true

      transaction_tracer:
        enabled: true
        threshold: 100  # ms
```

## SLA Monitoring

### SLI/SLO Configuration

```yaml
sla:
  objectives:
    - name: availability
      sli: |
        sum(rate(only1mcp_requests_total{status!~"5.."}[5m]))
        /
        sum(rate(only1mcp_requests_total[5m]))
      target: 0.999  # 99.9%
      window: 30d

    - name: latency_p99
      sli: |
        histogram_quantile(0.99,
          sum(rate(only1mcp_request_duration_seconds_bucket[5m])) by (le)
        )
      target_lt: 0.1  # 100ms
      window: 7d

    - name: error_rate
      sli: |
        sum(rate(only1mcp_requests_total{status=~"5.."}[5m]))
        /
        sum(rate(only1mcp_requests_total[5m]))
      target_lt: 0.001  # 0.1%
      window: 7d
```

### Error Budget Dashboard

```json
{
  "panels": [
    {
      "title": "Error Budget Remaining",
      "type": "gauge",
      "targets": [{
        "expr": "(1 - (1 - 0.999) * 30 * 24 * 60 / sum(increase(only1mcp_requests_total{status=~\"5..\"}[30d]))) * 100"
      }],
      "fieldConfig": {
        "defaults": {
          "min": 0,
          "max": 100,
          "thresholds": {
            "steps": [
              {"value": 0, "color": "red"},
              {"value": 50, "color": "yellow"},
              {"value": 80, "color": "green"}
            ]
          },
          "unit": "percent"
        }
      }
    }
  ]
}
```

## Troubleshooting

### Common Issues

#### Metrics Not Appearing

```bash
# Check if metrics endpoint is accessible
curl http://localhost:9091/metrics

# Verify Prometheus is scraping
curl http://prometheus:9090/api/v1/targets | jq '.data.activeTargets[] | select(.labels.job=="only1mcp")'

# Check for errors in Prometheus logs
docker logs prometheus | grep only1mcp
```

#### High Cardinality Metrics

```promql
# Find high cardinality metrics
topk(10, count by (__name__)({__name__=~"only1mcp.*"}))

# Check label cardinality
count(count by (request_id) (only1mcp_requests_total))
```

#### Missing Traces

```bash
# Verify trace export
curl -X POST http://localhost:4318/v1/traces \
  -H "Content-Type: application/json" \
  -d '{"resourceSpans":[]}'

# Check Jaeger for traces
curl "http://jaeger:16686/api/traces?service=only1mcp&limit=10"
```

### Debug Commands

```bash
# Enable debug metrics
only1mcp start --metrics-debug

# Export metrics snapshot
curl http://localhost:9091/metrics > metrics.txt

# Analyze cardinality
promtool tsdb analyze metrics.txt

# Validate alert rules
promtool check rules alerts.yml
```

### Performance Optimization

```yaml
# Reduce metric cardinality
monitoring:
  metrics:
    # Limit labels
    max_labels_per_metric: 10

    # Aggregate similar paths
    path_aggregation:
      - pattern: "/api/v1/users/[0-9]+"
        replacement: "/api/v1/users/{id}"

    # Sample high-volume metrics
    sampling:
      only1mcp_trace_spans: 0.01
      only1mcp_log_events: 0.1
```

## Best Practices

1. **Use appropriate retention periods**
   - Metrics: 15-30 days
   - Logs: 7-14 days
   - Traces: 3-7 days

2. **Implement metric aggregation**
   - Pre-aggregate at collection time
   - Use recording rules for common queries

3. **Control cardinality**
   - Limit unique label combinations
   - Use bounded label values

4. **Set up tiered alerting**
   - Page for critical issues
   - Slack/email for warnings
   - Dashboard for info level

5. **Document runbooks**
   - Link runbooks in alert annotations
   - Keep runbooks up-to-date
   - Include resolution steps

6. **Regular review**
   - Monthly SLO review
   - Quarterly alert tuning
   - Annual capacity planning