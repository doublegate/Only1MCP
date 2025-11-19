# Performance Tuning Guide

This guide provides tips and best practices for optimizing Only1MCP performance in production deployments.

## Table of Contents

- [Quick Wins](#quick-wins)
- [Configuration Optimization](#configuration-optimization)
- [System Tuning](#system-tuning)
- [Benchmarking](#benchmarking)
- [Monitoring](#monitoring)
- [Troubleshooting Performance](#troubleshooting-performance)

---

## Quick Wins

These optimizations can be applied immediately for significant performance improvements:

### 1. Enable Response Caching

```yaml
cache:
  enabled: true
  ttl: 600  # 10 minutes
  max_size: 10000
  compression: "zstd"  # Best compression ratio
```

**Expected Impact**: 50-70% reduction in backend load, 80-90% faster responses for cached content.

### 2. Enable Request Batching

```yaml
batching:
  enabled: true
  window_ms: 50  # Balance between latency and batching efficiency
  max_batch_size: 50
```

**Expected Impact**: 30-40% reduction in transport overhead for bursty traffic.

### 3. Use Appropriate Load Balancing

```yaml
load_balancing:
  strategy: "weighted_least_connections"  # Best for variable backend load
  health_check_interval: 15  # Faster failure detection
  failure_threshold: 2  # Quick failover
```

**Expected Impact**: Better backend utilization, fewer failed requests.

### 4. Enable AI Optimization

```yaml
ai_optimization:
  enabled: true
  ml_routing:
    enabled: true  # Learn optimal routing
  predictive_cache:
    enabled: true  # Preload popular content
```

**Expected Impact**: 15-25% latency improvement over time as ML learns patterns.

---

## Configuration Optimization

### Connection Pool Settings

```yaml
transport:
  http:
    pool_size: 50  # Connections per backend
    pool_timeout_ms: 5000
    keep_alive: true
    keep_alive_timeout_secs: 300
```

### Timeout Configuration

```yaml
timeouts:
  backend: 5000  # 5 seconds
  connect: 2000  # 2 seconds
  request: 30000  # 30 seconds
```

**Recommendations**:
- **Low latency required**: Use shorter timeouts (1-2s)
- **High reliability required**: Use longer timeouts (10-30s)
- **Mixed workload**: Use tiered timeouts per server

### Rate Limiting

```yaml
rate_limiting:
  enabled: true
  algorithm: "token_bucket"  # Better burst handling than fixed window
  requests_per_minute: 1000
  burst_size: 1500  # Allow 50% burst capacity
```

### Cache Strategies

**For read-heavy workloads**:
```yaml
cache:
  ttl: 3600  # 1 hour
  max_size: 100000
  eviction_policy: "lru"
```

**For write-heavy workloads**:
```yaml
cache:
  ttl: 60  # 1 minute
  max_size: 1000
  write_through: true
```

**For mixed workloads**:
```yaml
cache:
  ttl: 300  # 5 minutes
  max_size: 10000
  adaptive_ttl: true  # Adjust based on access patterns
```

---

## System Tuning

### Linux Kernel Parameters

Edit `/etc/sysctl.conf`:

```bash
# Increase TCP backlog
net.core.somaxconn = 65535
net.ipv4.tcp_max_syn_backlog = 8192

# TCP tuning
net.ipv4.tcp_fin_timeout = 15
net.ipv4.tcp_tw_reuse = 1
net.ipv4.tcp_keepalive_time = 300
net.ipv4.tcp_keepalive_probes = 5
net.ipv4.tcp_keepalive_intvl = 15

# Increase file descriptors
fs.file-max = 2097152

# Network buffer sizes
net.core.rmem_max = 134217728
net.core.wmem_max = 134217728
net.ipv4.tcp_rmem = 4096 87380 67108864
net.ipv4.tcp_wmem = 4096 65536 67108864

# Disable swapping
vm.swappiness = 1
```

Apply with:
```bash
sudo sysctl -p
```

### File Descriptor Limits

Edit `/etc/security/limits.conf`:

```
only1mcp soft nofile 65535
only1mcp hard nofile 65535
```

### Systemd Service Limits

In `/etc/systemd/system/only1mcp.service`:

```ini
[Service]
LimitNOFILE=65535
LimitNPROC=4096
```

### CPU Affinity

Pin Only1MCP to specific CPU cores for better cache locality:

```bash
taskset -c 0-7 ./only1mcp start
```

Or in systemd:
```ini
[Service]
CPUAffinity=0-7
```

---

## Benchmarking

### Using Criterion Benchmarks

Run included benchmarks:

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark
cargo bench stress_test

# Generate HTML report
cargo bench -- --save-baseline my-baseline
```

### Load Testing with wrk

```bash
# Install wrk
sudo apt-get install wrk

# Basic load test
wrk -t12 -c400 -d30s http://localhost:8080/health

# POST requests
wrk -t12 -c400 -d30s -s post.lua http://localhost:8080/mcp
```

Example `post.lua`:
```lua
wrk.method = "POST"
wrk.headers["Content-Type"] = "application/json"
wrk.body = '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}'
```

### Load Testing with k6

```javascript
import http from 'k6/http';
import { check } from 'k6';

export let options = {
  vus: 100,  // 100 virtual users
  duration: '5m',
};

export default function() {
  let res = http.post('http://localhost:8080/mcp', JSON.stringify({
    jsonrpc: "2.0",
    id: 1,
    method: "tools/list",
    params: {}
  }), {
    headers: { 'Content-Type': 'application/json' },
  });

  check(res, {
    'status is 200': (r) => r.status === 200,
    'response time < 100ms': (r) => r.timings.duration < 100,
  });
}
```

### Profiling

**CPU Profiling**:
```bash
# Install flamegraph tools
cargo install flamegraph

# Generate flamegraph
cargo flamegraph --bin only1mcp

# Open flamegraph.svg in browser
```

**Memory Profiling**:
```bash
# Install heaptrack
sudo apt-get install heaptrack

# Profile memory usage
heaptrack ./target/release/only1mcp start

# Analyze results
heaptrack_gui heaptrack.only1mcp.<pid>.gz
```

---

## Monitoring

### Prometheus Metrics

Enable Prometheus metrics:

```yaml
metrics:
  enabled: true
  prometheus_port: 9090
  collection_interval: 10
```

Key metrics to monitor:

```
# Request rate
rate(only1mcp_requests_total[5m])

# Error rate
rate(only1mcp_errors_total[5m]) / rate(only1mcp_requests_total[5m])

# Latency percentiles
histogram_quantile(0.95, only1mcp_request_duration_seconds_bucket)
histogram_quantile(0.99, only1mcp_request_duration_seconds_bucket)

# Cache hit rate
only1mcp_cache_hits / (only1mcp_cache_hits + only1mcp_cache_misses)

# Backend health
only1mcp_backend_healthy
```

### Grafana Dashboard

Example dashboard configuration:

```json
{
  "panels": [
    {
      "title": "Request Rate",
      "targets": [{
        "expr": "rate(only1mcp_requests_total[5m])"
      }]
    },
    {
      "title": "Latency (p95)",
      "targets": [{
        "expr": "histogram_quantile(0.95, only1mcp_request_duration_seconds_bucket)"
      }]
    },
    {
      "title": "Error Rate",
      "targets": [{
        "expr": "rate(only1mcp_errors_total[5m])"
      }]
    },
    {
      "title": "Cache Hit Rate",
      "targets": [{
        "expr": "only1mcp_cache_hits / (only1mcp_cache_hits + only1mcp_cache_misses)"
      }]
    }
  ]
}
```

### Logging Configuration

**For development**:
```yaml
log_level: "debug"
log_format: "pretty"
```

**For production**:
```yaml
log_level: "warn"  # Reduce noise
log_format: "json"  # Structured logging
log_output: "/var/log/only1mcp/proxy.log"
log_rotation: "daily"
log_retention_days: 30
```

---

## Troubleshooting Performance

### High CPU Usage

**Symptoms**: CPU usage consistently >80%

**Solutions**:
1. Enable caching to reduce backend calls
2. Increase batch window to reduce processing overhead
3. Scale horizontally (multiple instances)
4. Profile to find hot code paths

```bash
# Check CPU usage by thread
top -H -p $(pidof only1mcp)

# Profile for 30 seconds
cargo flamegraph --bin only1mcp -- start &
sleep 30
kill $(pidof only1mcp)
```

### High Memory Usage

**Symptoms**: Memory usage growing unbounded

**Solutions**:
1. Reduce cache size
2. Reduce metrics retention
3. Limit event log size
4. Check for memory leaks

```bash
# Monitor memory growth
watch -n 1 'ps aux | grep only1mcp | grep -v grep'

# Detailed memory analysis
heaptrack ./target/release/only1mcp start
```

### High Latency

**Symptoms**: p95 latency >100ms

**Solutions**:
1. Enable caching
2. Reduce backend timeout
3. Enable ML routing for optimal server selection
4. Check network latency to backends

```bash
# Measure latency distribution
wrk -t4 -c100 -d30s --latency http://localhost:8080/health

# Check backend latency
curl -w "@curl-format.txt" -o /dev/null -s http://backend:3000
```

curl-format.txt:
```
time_namelookup:  %{time_namelookup}\n
time_connect:     %{time_connect}\n
time_appconnect:  %{time_appconnect}\n
time_pretransfer: %{time_pretransfer}\n
time_redirect:    %{time_redirect}\n
time_starttransfer: %{time_starttransfer}\n
time_total:       %{time_total}\n
```

### Connection Pool Exhaustion

**Symptoms**: "No available connections" errors

**Solutions**:
1. Increase pool size
2. Reduce connection idle time
3. Add more backend instances
4. Enable connection reuse

```yaml
transport:
  http:
    pool_size: 100  # Increase from 50
    pool_timeout_ms: 10000  # Allow more wait time
    keep_alive: true
```

### Cache Inefficiency

**Symptoms**: Low cache hit rate (<50%)

**Solutions**:
1. Increase TTL for stable data
2. Increase cache size
3. Enable predictive caching
4. Analyze access patterns

```bash
# Check cache metrics
curl http://localhost:9090/metrics | grep cache

# Expected output:
# only1mcp_cache_hits 8000
# only1mcp_cache_misses 2000
# Hit rate = 80%
```

---

## Performance Targets

### Expected Performance

**Single Instance**:
- **Throughput**: 10,000-15,000 req/s
- **Latency (p95)**: <50ms
- **Memory**: <500MB
- **CPU**: 2-4 cores at 50% utilization

**Clustered (3 instances)**:
- **Throughput**: 30,000-45,000 req/s
- **Latency (p95)**: <30ms
- **Memory**: <1.5GB total
- **Availability**: 99.9%

### Scaling Guidelines

| Request Rate | Instances | CPU | Memory | Notes |
|--------------|-----------|-----|--------|-------|
| <1,000/s | 1 | 1 core | 256MB | Dev/small team |
| 1,000-5,000/s | 1-2 | 2 cores | 512MB | Small production |
| 5,000-15,000/s | 2-3 | 4 cores | 1GB | Medium production |
| 15,000-50,000/s | 3-6 | 8 cores | 2GB | Large production |
| >50,000/s | 6+ | 16+ cores | 4GB+ | Enterprise |

---

## Best Practices Checklist

- [ ] Enable response caching with appropriate TTL
- [ ] Enable request batching for bursty traffic
- [ ] Use weighted least connections load balancing
- [ ] Configure appropriate timeouts
- [ ] Enable AI optimization for learning
- [ ] Set up Prometheus monitoring
- [ ] Configure log rotation
- [ ] Tune kernel parameters
- [ ] Increase file descriptor limits
- [ ] Run benchmarks before production
- [ ] Set up alerts for key metrics
- [ ] Document baseline performance
- [ ] Plan capacity for 2x expected load
- [ ] Enable TLS with session reuse
- [ ] Use connection pooling
- [ ] Monitor and tune cache hit rate
- [ ] Profile regularly for hot paths
- [ ] Test failover scenarios
- [ ] Document scaling procedures
- [ ] Set up load testing pipeline

---

## Resources

- [Benchmark suites](../benches/)
- [Example configurations](configs/)
- [System architecture](../docs/ARCHITECTURE.md)
- [Prometheus metrics guide](https://prometheus.io/docs/guides/cadvisor/)
- [Linux performance tools](http://www.brendangregg.com/linuxperf.html)
