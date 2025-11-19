# Real-Time Analytics Dashboard Guide

Comprehensive guide to Only1MCP's real-time analytics and visualization capabilities.

## Overview

Only1MCP provides powerful real-time analytics through:
- Live metrics dashboards
- Historical trend analysis
- Custom query language
- Alert configuration
- Performance insights

## Quick Start

### Access Dashboards

**Prometheus Metrics**:
```
http://localhost:9090/metrics
```

**Grafana Dashboards**:
```
http://localhost:3000
Username: admin
Password: admin
```

**Admin API**:
```
http://localhost:8081/api/analytics
```

### Pre-built Dashboards

1. **Overview Dashboard** - High-level system health
2. **Performance Dashboard** - Request latency and throughput
3. **Error Dashboard** - Error rates and types
4. **Resource Dashboard** - CPU, memory, network usage
5. **Server Dashboard** - Per-server metrics
6. **Cache Dashboard** - Cache efficiency and hit rates

## Metrics Collection

### Core Metrics

**Request Metrics**:
```
only1mcp_requests_total
only1mcp_requests_duration_seconds
only1mcp_requests_in_flight
only1mcp_request_size_bytes
only1mcp_response_size_bytes
```

**Error Metrics**:
```
only1mcp_errors_total{type,server}
only1mcp_error_rate
only1mcp_timeouts_total
only1mcp_connection_errors_total
```

**Performance Metrics**:
```
only1mcp_latency_p50
only1mcp_latency_p95
only1mcp_latency_p99
only1mcp_throughput_rps
```

**Resource Metrics**:
```
only1mcp_cpu_usage_percent
only1mcp_memory_usage_bytes
only1mcp_open_connections
only1mcp_goroutines_count
```

**Cache Metrics**:
```
only1mcp_cache_hits_total
only1mcp_cache_misses_total
only1mcp_cache_hit_rate
only1mcp_cache_size_bytes
only1mcp_cache_evictions_total
```

### Custom Metrics

Define custom metrics in configuration:

```yaml
metrics:
  custom:
    - name: "business_requests"
      type: "counter"
      help: "Business-specific request counter"
      labels: ["customer_tier", "api_version"]
    
    - name: "processing_duration"
      type: "histogram"
      help: "Request processing duration"
      buckets: [0.1, 0.5, 1.0, 2.5, 5.0, 10.0]
```

## Real-Time Dashboards

### Dashboard Configuration

**Grafana Dashboard JSON**:
```json
{
  "dashboard": {
    "title": "Only1MCP Overview",
    "panels": [
      {
        "title": "Request Rate",
        "targets": [
          {
            "expr": "rate(only1mcp_requests_total[5m])"
          }
        ],
        "type": "graph"
      },
      {
        "title": "Error Rate",
        "targets": [
          {
            "expr": "rate(only1mcp_errors_total[5m])"
          }
        ],
        "type": "graph"
      }
    ]
  }
}
```

### Live Updates

**WebSocket Connection**:
```javascript
const ws = new WebSocket('ws://localhost:8081/api/metrics/stream');

ws.onmessage = (event) => {
  const metrics = JSON.parse(event.data);
  updateDashboard(metrics);
};
```

**Server-Sent Events**:
```javascript
const eventSource = new EventSource('http://localhost:8081/api/metrics/sse');

eventSource.onmessage = (event) => {
  const data = JSON.parse(event.data);
  updateCharts(data);
};
```

## Custom Query Language

### Query Syntax

**Basic Queries**:
```
# Get metric current value
metric_name

# Filter by label
metric_name{label="value"}

# Multiple labels
metric_name{label1="value1", label2="value2"}
```

**Aggregations**:
```
# Sum across all series
sum(metric_name)

# Average
avg(metric_name)

# Min/Max
min(metric_name)
max(metric_name)

# Count
count(metric_name)
```

**Time Windows**:
```
# Rate over 5 minutes
rate(metric_name[5m])

# Increase over 1 hour
increase(metric_name[1h])

# Average over time
avg_over_time(metric_name[10m])
```

**Mathematical Operations**:
```
# Addition
metric1 + metric2

# Percentage
(metric1 / metric2) * 100

# Combined
(sum(rate(requests[5m])) / sum(rate(capacity[5m]))) * 100
```

### Example Queries

**Request Success Rate**:
```
sum(rate(only1mcp_requests_total{status="success"}[5m]))
/ sum(rate(only1mcp_requests_total[5m])) * 100
```

**Average Latency by Server**:
```
avg(only1mcp_latency_seconds) by (server)
```

**Cache Efficiency**:
```
sum(rate(only1mcp_cache_hits[5m]))
/ (sum(rate(only1mcp_cache_hits[5m])) + sum(rate(only1mcp_cache_misses[5m])))
```

**Top 5 Slowest Endpoints**:
```
topk(5, avg(only1mcp_request_duration_seconds) by (endpoint))
```

## Alert Configuration

### Alert Rules

**Prometheus Alerts**:
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
          description: "Error rate is {{ $value | humanizePercentage }}"
      
      - alert: HighLatency
        expr: only1mcp_latency_p95 > 1.0
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "High latency detected"
          description: "P95 latency is {{ $value }}s"
      
      - alert: ServerDown
        expr: up{job="only1mcp"} == 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "Server {{ $labels.instance }} is down"
```

### Alert Channels

**Email Notifications**:
```yaml
alerting:
  email:
    enabled: true
    smtp_host: "smtp.gmail.com"
    smtp_port: 587
    from: "alerts@only1mcp.com"
    to: ["ops@company.com"]
```

**Slack Integration**:
```yaml
alerting:
  slack:
    enabled: true
    webhook_url: "https://hooks.slack.com/services/..."
    channel: "#only1mcp-alerts"
    username: "Only1MCP"
```

**PagerDuty**:
```yaml
alerting:
  pagerduty:
    enabled: true
    service_key: "your-service-key"
    severity_mapping:
      critical: "critical"
      warning: "warning"
```

## Performance Insights

### Automated Insights

**Bottleneck Detection**:
```
Insight: Server "mcp-filesystem" is bottleneck
- Handles 45% of requests
- 2.5x slower than average
- Recommendation: Add replica or increase capacity
```

**Optimization Opportunities**:
```
Insight: Cache hit rate could improve
- Current: 65%
- Potential: 85% (+20%)
- Action: Increase TTL from 5m to 15m
- Estimated savings: 120 requests/min
```

**Resource Utilization**:
```
Insight: Under-utilized resources detected
- Average CPU: 25%
- Average Memory: 40%
- Recommendation: Reduce from 4 to 2 instances
- Estimated cost savings: 50%
```

### Trend Analysis

**Weekly Patterns**:
```
Monday: Peak at 10am (500 req/s)
Tuesday-Thursday: Steady 300 req/s
Friday: Drop after 2pm (-40%)
Weekend: Minimal traffic (50 req/s)

Recommendation: Schedule maintenance on Sunday 2-4am
```

**Seasonal Trends**:
```
Month-over-month growth: +15%
Year-over-year growth: +180%
Predicted capacity needs (6 months): 1200 req/s
Current capacity: 800 req/s
Action required: Scale up by Q2
```

## Dashboard Customization

### Custom Panels

**JavaScript Panel**:
```javascript
// Custom heatmap visualization
function renderHeatmap(data) {
  const hours = Array.from({length: 24}, (_, i) => i);
  const days = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'];
  
  return (
    <div className="heatmap">
      {days.map(day => (
        <div className="row">
          {hours.map(hour => (
            <div 
              className="cell"
              style={{
                backgroundColor: getColor(data[day][hour])
              }}
              title={`${day} ${hour}:00 - ${data[day][hour]} req/s`}
            />
          ))}
        </div>
      ))}
    </div>
  );
}
```

**Custom Charts**:
```javascript
import { LineChart, BarChart, PieChart } from 'recharts';

function MetricsDashboard({ metrics }) {
  return (
    <div>
      <LineChart data={metrics.timeseries}>
        <Line dataKey="requests" stroke="#8884d8" />
        <Line dataKey="errors" stroke="#ff0000" />
      </LineChart>
      
      <BarChart data={metrics.byServer}>
        <Bar dataKey="throughput" fill="#82ca9d" />
      </BarChart>
      
      <PieChart data={metrics.errorTypes}>
        <Pie dataKey="count" nameKey="type" />
      </PieChart>
    </div>
  );
}
```

### Dashboard Templates

**Performance Overview**:
- Request rate (line chart)
- Error rate (line chart)
- Latency percentiles (multi-line)
- Active connections (gauge)
- Top endpoints (table)

**Server Health**:
- CPU usage by server (stacked area)
- Memory usage (line chart)
- Network I/O (dual-axis)
- Health status (status panel)
- Recent errors (log panel)

**Business Metrics**:
- Requests by customer tier (pie)
- Revenue per endpoint (bar)
- API version usage (stacked bar)
- Geographic distribution (map)
- Conversion funnel (sankey)

## API Reference

### Metrics Endpoint

```http
GET /api/metrics

Response:
{
  "timestamp": "2025-01-19T14:30:00Z",
  "metrics": {
    "requests_total": 1500000,
    "requests_per_second": 125.5,
    "error_rate": 0.012,
    "latency_p50": 25.3,
    "latency_p95": 89.7,
    "latency_p99": 156.2
  }
}
```

### Query Endpoint

```http
POST /api/query
Content-Type: application/json

{
  "query": "sum(rate(only1mcp_requests_total[5m]))",
  "time": "2025-01-19T14:30:00Z"
}

Response:
{
  "status": "success",
  "data": {
    "resultType": "vector",
    "result": [
      {
        "metric": {},
        "value": [1705675800, "125.5"]
      }
    ]
  }
}
```

### Range Query

```http
POST /api/query_range

{
  "query": "only1mcp_latency_p95",
  "start": "2025-01-19T00:00:00Z",
  "end": "2025-01-19T23:59:59Z",
  "step": "5m"
}
```

## Integration

### Grafana Setup

1. Add Prometheus data source
2. Import dashboard JSON
3. Configure variables
4. Set refresh interval
5. Save and share

### Custom Applications

```python
import requests

def get_metrics():
    response = requests.get('http://localhost:8081/api/metrics')
    return response.json()

def query_metric(query):
    response = requests.post(
        'http://localhost:8081/api/query',
        json={'query': query}
    )
    return response.json()['data']['result']

# Usage
metrics = get_metrics()
print(f"Current RPS: {metrics['requests_per_second']}")

# Custom query
error_rate = query_metric(
    'rate(only1mcp_errors_total[5m]) / rate(only1mcp_requests_total[5m])'
)
```

## Best Practices

1. **Dashboard Design**:
   - Keep dashboards focused
   - Use consistent color schemes
   - Include time range selectors
   - Add helpful annotations

2. **Alert Configuration**:
   - Start with critical alerts only
   - Tune thresholds based on baselines
   - Avoid alert fatigue
   - Include runbook links

3. **Performance**:
   - Limit dashboard panel count
   - Use appropriate time ranges
   - Leverage caching
   - Optimize queries

4. **Data Retention**:
   - High-resolution: 24 hours
   - Medium-resolution: 7 days
   - Low-resolution: 90 days
   - Archive: 1+ years

## Troubleshooting

**Dashboards not loading**:
- Check Prometheus connection
- Verify data source configuration
- Review query syntax
- Check time range

**Missing metrics**:
- Confirm metric collection is enabled
- Check scrape interval
- Review metric retention
- Verify label filters

**Slow queries**:
- Reduce time range
- Optimize query complexity
- Add caching
- Use recording rules

---

For support, see [GitHub Issues](https://github.com/doublegate/Only1MCP/issues).
