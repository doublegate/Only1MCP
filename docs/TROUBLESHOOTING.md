# Only1MCP Troubleshooting Guide

Comprehensive troubleshooting guide for common issues and debugging techniques.

## Table of Contents
- [Quick Diagnostics](#quick-diagnostics)
- [Common Issues](#common-issues)
  - [Startup Issues](#startup-issues)
  - [Connection Issues](#connection-issues)
  - [Performance Issues](#performance-issues)
  - [Authentication Issues](#authentication-issues)
  - [Backend Issues](#backend-issues)
- [Error Messages](#error-messages)
- [Debugging Tools](#debugging-tools)
- [Log Analysis](#log-analysis)
- [Performance Debugging](#performance-debugging)
- [Network Debugging](#network-debugging)
- [Memory Issues](#memory-issues)
- [CPU Issues](#cpu-issues)
- [Configuration Issues](#configuration-issues)
- [Recovery Procedures](#recovery-procedures)
- [Getting Help](#getting-help)

## Quick Diagnostics

Run this command for a quick health check:

```bash
#!/bin/bash
# quick-check.sh

echo "=== Only1MCP Quick Diagnostics ==="
echo

echo "1. Process Status:"
if pgrep -f only1mcp > /dev/null; then
    echo "   ✓ Only1MCP is running (PID: $(pgrep -f only1mcp))"
else
    echo "   ✗ Only1MCP is not running"
fi

echo "2. Port Status:"
if netstat -tuln | grep -q ":8080"; then
    echo "   ✓ Port 8080 is listening"
else
    echo "   ✗ Port 8080 is not listening"
fi

echo "3. Health Check:"
if curl -sf http://localhost:8080/health > /dev/null; then
    echo "   ✓ Health endpoint responding"
else
    echo "   ✗ Health endpoint not responding"
fi

echo "4. Config Validation:"
if only1mcp validate /etc/only1mcp/config.yaml 2>/dev/null; then
    echo "   ✓ Configuration is valid"
else
    echo "   ✗ Configuration has errors"
fi

echo "5. Recent Errors:"
tail -n 5 /var/log/only1mcp/error.log 2>/dev/null || echo "   No recent errors"
```

## Common Issues

### Startup Issues

#### Issue: Only1MCP fails to start

**Symptoms:**
- Process exits immediately
- No log output
- Error message about configuration

**Solutions:**

1. **Check configuration syntax:**
```bash
only1mcp validate config.yaml
```

2. **Check file permissions:**
```bash
# Ensure config is readable
ls -la /etc/only1mcp/config.yaml
chmod 644 /etc/only1mcp/config.yaml

# Ensure binary is executable
chmod +x /usr/local/bin/only1mcp
```

3. **Check for port conflicts:**
```bash
# See what's using port 8080
lsof -i :8080
netstat -tulpn | grep :8080

# Kill conflicting process or change port
kill -9 <PID>
# OR
only1mcp start --port 8081
```

4. **Check system resources:**
```bash
# Memory
free -h

# Disk space
df -h

# File descriptors
ulimit -n
```

#### Issue: Configuration file not found

**Error:**
```
Error: Configuration file not found: config.yaml
```

**Solutions:**
```bash
# Specify config path explicitly
only1mcp start --config /path/to/config.yaml

# Or set environment variable
export ONLY1MCP_CONFIG=/path/to/config.yaml
only1mcp start

# Create default config
only1mcp config generate > config.yaml
```

### Connection Issues

#### Issue: Cannot connect to backend servers

**Symptoms:**
- "No backend available" errors
- All requests failing
- Circuit breakers open

**Debugging steps:**

1. **Check backend health:**
```bash
# List backend status
only1mcp health status

# Test specific backend
only1mcp health check github

# Manual connection test
curl -v https://backend-server.com/health
```

2. **Check network connectivity:**
```bash
# DNS resolution
nslookup backend-server.com
dig backend-server.com

# Network path
traceroute backend-server.com

# Firewall rules
iptables -L -n | grep 443
```

3. **Check authentication:**
```bash
# Verify environment variables
echo $GITHUB_TOKEN
echo $OPENAI_API_KEY

# Test with curl
curl -H "Authorization: Bearer $GITHUB_TOKEN" \
     https://api.github.com/user
```

#### Issue: Intermittent connection failures

**Solutions:**

1. **Increase timeouts:**
```yaml
servers:
  - id: slow-backend
    timeout: 30s  # Increase from default
    retry:
      max_attempts: 5
      backoff: exponential
```

2. **Configure connection pooling:**
```yaml
proxy:
  connection_pool:
    max_per_backend: 100
    min_idle: 5
    max_idle_time_ms: 300000
    validation_interval: 60s
```

3. **Enable circuit breaker:**
```yaml
health:
  circuit_breaker:
    enabled: true
    failure_threshold: 5
    recovery_timeout: 30s
```

### Performance Issues

#### Issue: High latency

**Symptoms:**
- Slow response times
- Timeouts
- High P99 latency metrics

**Investigation:**

1. **Profile the application:**
```bash
# Enable profiling
only1mcp start --profile

# Generate flame graph
curl http://localhost:6060/debug/pprof/profile?seconds=30 > profile.pb
go tool pprof -http=:8081 profile.pb
```

2. **Check cache effectiveness:**
```bash
# Cache statistics
curl http://localhost:9091/metrics | grep cache

# Calculate hit ratio
echo "scale=2; $(curl -s http://localhost:9091/metrics | grep cache_hits | awk '{print $2}') / ($(curl -s http://localhost:9091/metrics | grep cache_hits | awk '{print $2}') + $(curl -s http://localhost:9091/metrics | grep cache_misses | awk '{print $2}'))" | bc
```

3. **Analyze slow queries:**
```sql
-- For request logging
SELECT
    method,
    AVG(duration_ms) as avg_duration,
    MAX(duration_ms) as max_duration,
    COUNT(*) as count
FROM request_logs
WHERE timestamp > NOW() - INTERVAL '1 hour'
GROUP BY method
ORDER BY avg_duration DESC;
```

**Solutions:**

1. **Enable caching:**
```yaml
cache:
  enabled: true
  max_size_mb: 1000
  ttl_seconds: 300

  # Use Redis for distributed cache
  backend: redis
  redis:
    url: "redis://localhost:6379"
```

2. **Optimize routing:**
```yaml
proxy:
  load_balancer:
    algorithm: least_connections  # Better for varying response times

  # Enable request batching
  batching:
    enabled: true
    window_ms: 100
    max_batch_size: 50
```

3. **Enable compression:**
```yaml
context_optimization:
  compression:
    enabled: true
    algorithm: zstd
    level: 3
    min_size: 1024
```

#### Issue: High memory usage

**Symptoms:**
- OOM killer terminating process
- Gradual memory increase
- System slowdown

**Investigation:**

```bash
# Memory profiling
curl http://localhost:6060/debug/pprof/heap > heap.pb
go tool pprof -http=:8082 heap.pb

# Check for leaks
valgrind --leak-check=full --show-leak-kinds=all \
         only1mcp start --config config.yaml

# Monitor memory usage
watch -n 1 'ps aux | grep only1mcp'
```

**Solutions:**

1. **Limit cache size:**
```yaml
cache:
  max_size_mb: 500  # Reduce from unlimited
  max_entries: 10000

  eviction:
    policy: lru
    high_water_mark: 0.9
    low_water_mark: 0.7
```

2. **Configure memory limits:**
```bash
# Systemd
[Service]
MemoryLimit=2G
MemorySwapMax=0

# Docker
docker run -m 2g --memory-swap 2g only1mcp
```

### Authentication Issues

#### Issue: Authentication failures

**Error messages:**
```
Error: Authentication failed: Invalid API key
Error: OAuth token expired
Error: JWT signature verification failed
```

**Solutions:**

1. **API Key issues:**
```bash
# Regenerate API key
openssl rand -hex 32

# Test API key
curl -H "X-API-Key: your-key-here" http://localhost:8080/health
```

2. **OAuth issues:**
```bash
# Check token expiry
jwt decode $OAUTH_TOKEN

# Refresh token
curl -X POST https://auth.example.com/token \
     -d "grant_type=refresh_token" \
     -d "refresh_token=$REFRESH_TOKEN"
```

3. **Certificate issues:**
```bash
# Check certificate validity
openssl x509 -in cert.pem -text -noout

# Verify certificate chain
openssl verify -CAfile ca.pem cert.pem

# Test mTLS
curl --cert client.crt --key client.key \
     --cacert ca.crt https://localhost:8443
```

### Backend Issues

#### Issue: Circuit breaker constantly open

**Investigation:**
```bash
# Check circuit breaker status
curl http://localhost:9091/metrics | grep circuit_breaker

# Check backend health history
only1mcp health history --server github --last 1h
```

**Solutions:**

1. **Adjust circuit breaker settings:**
```yaml
circuit_breaker:
  failure_threshold: 10  # Increase from 5
  success_threshold: 3
  timeout: 60s  # Increase recovery timeout
  half_open_requests: 5
```

2. **Implement fallback:**
```yaml
servers:
  - id: primary
    endpoint: https://primary.example.com

  - id: fallback
    endpoint: https://fallback.example.com
    role: fallback  # Only use when primary fails
```

## Error Messages

### Common Error Codes

| Error Code | Meaning | Solution |
|------------|---------|----------|
| -32000 | Generic server error | Check logs for details |
| -32001 | Backend timeout | Increase timeout or check backend |
| -32002 | No backend available | Check server configuration |
| -32003 | Authentication failed | Verify credentials |
| -32004 | Rate limit exceeded | Wait or increase limits |
| -32600 | Invalid request | Check request format |
| -32601 | Method not found | Verify method name |
| -32602 | Invalid params | Check parameter format |
| -32603 | Internal error | Check server logs |

### Error Message Patterns

```bash
# Parse error logs for patterns
grep ERROR /var/log/only1mcp/error.log | \
  awk '{print $5}' | \
  sort | uniq -c | sort -rn | head -10

# Find correlated errors
grep -B5 -A5 "PANIC" /var/log/only1mcp/error.log

# Extract stack traces
awk '/stack backtrace:/,/^$/' /var/log/only1mcp/error.log
```

## Debugging Tools

### Enable Debug Logging

```bash
# Via command line
only1mcp start --log-level debug

# Via environment variable
RUST_LOG=debug only1mcp start

# Module-specific debugging
RUST_LOG=only1mcp::proxy=trace,only1mcp::auth=debug only1mcp start
```

### Request Tracing

```bash
# Enable request tracing
curl -H "X-Trace-ID: test-123" \
     -H "X-Debug: true" \
     http://localhost:8080/api/tools/list

# Follow trace in logs
grep "test-123" /var/log/only1mcp/*.log
```

### Performance Profiling

```bash
# CPU profiling
only1mcp start --profile-cpu profile.pb
go tool pprof -http=:8081 profile.pb

# Memory profiling
only1mcp start --profile-mem memprofile.pb
go tool pprof -http=:8082 memprofile.pb

# Trace execution
only1mcp start --trace trace.out
go tool trace trace.out
```

## Log Analysis

### Log Aggregation Queries

```bash
# Most common errors
grep ERROR *.log | cut -d' ' -f5- | sort | uniq -c | sort -rn

# Requests by status code
awk '$8 ~ /^[0-9]+$/ {print $8}' access.log | \
  sort | uniq -c | sort -rn

# Slowest requests
awk '$10 > 1000 {print $7, $10"ms"}' access.log | \
  sort -t' ' -k2 -rn | head -20

# Error rate over time
awk '{print substr($1,1,13) ":00"}' error.log | \
  uniq -c | awk '{print $2, $1}'
```

### Log Correlation

```python
#!/usr/bin/env python3
# correlate_logs.py

import re
import sys
from collections import defaultdict

# Parse logs
request_map = defaultdict(list)

for line in sys.stdin:
    # Extract request ID
    match = re.search(r'request_id=(\S+)', line)
    if match:
        request_id = match.group(1)
        request_map[request_id].append(line.strip())

# Find problematic requests
for request_id, logs in request_map.items():
    if any('ERROR' in log for log in logs):
        print(f"\n=== Request {request_id} ===")
        for log in logs:
            print(log)
```

## Performance Debugging

### Identify Bottlenecks

```bash
# System bottlenecks
iostat -x 1
vmstat 1
sar -n DEV 1

# Network bottlenecks
ss -s
netstat -i
iftop

# Process bottlenecks
strace -c -p $(pgrep only1mcp)
ltrace -c -p $(pgrep only1mcp)
```

### Query Analysis

```sql
-- Slow query analysis
SELECT
    method,
    percentile_cont(0.99) WITHIN GROUP (ORDER BY duration_ms) as p99,
    percentile_cont(0.95) WITHIN GROUP (ORDER BY duration_ms) as p95,
    percentile_cont(0.50) WITHIN GROUP (ORDER BY duration_ms) as p50,
    COUNT(*) as count
FROM request_logs
WHERE timestamp > NOW() - INTERVAL '1 hour'
GROUP BY method
HAVING COUNT(*) > 10
ORDER BY p99 DESC;
```

## Network Debugging

### Packet Capture

```bash
# Capture traffic
tcpdump -i any -w only1mcp.pcap port 8080

# Analyze with tshark
tshark -r only1mcp.pcap -Y "http.response.code >= 400"

# HTTP traffic analysis
tcpdump -i any -A -s 0 'tcp port 8080 and (((ip[2:2] - ((ip[0]&0xf)<<2)) - ((tcp[12]&0xf0)>>2)) != 0)'
```

### Connection Issues

```bash
# Check connection states
ss -tan | awk '{print $1}' | sort | uniq -c

# Find connection leaks
lsof -p $(pgrep only1mcp) | grep TCP | wc -l

# Monitor connections
watch -n 1 'ss -s'
```

## Memory Issues

### Memory Leak Detection

```bash
# Heap analysis
jemalloc_stats() {
    MALLOC_CONF=stats_print:true only1mcp start
}

# Valgrind analysis
valgrind --leak-check=full \
         --show-leak-kinds=all \
         --track-origins=yes \
         --verbose \
         --log-file=valgrind-out.txt \
         only1mcp start

# Address sanitizer
RUSTFLAGS="-Z sanitizer=address" \
    cargo build --target x86_64-unknown-linux-gnu

./target/x86_64-unknown-linux-gnu/debug/only1mcp
```

### Memory Profiling

```python
#!/usr/bin/env python3
# memory_analysis.py

import subprocess
import time
import matplotlib.pyplot as plt

pids = []
memory = []

# Monitor for 60 seconds
for i in range(60):
    output = subprocess.check_output(
        ['ps', 'aux']
    ).decode('utf-8')

    for line in output.split('\n'):
        if 'only1mcp' in line:
            parts = line.split()
            memory.append(float(parts[5]) / 1024)  # VSZ in MB
            break

    time.sleep(1)

# Plot
plt.plot(memory)
plt.xlabel('Time (seconds)')
plt.ylabel('Memory (MB)')
plt.title('Only1MCP Memory Usage')
plt.show()
```

## CPU Issues

### High CPU Usage

```bash
# Find CPU-intensive functions
perf record -F 99 -p $(pgrep only1mcp) -- sleep 30
perf report

# Flame graph
perf script | flamegraph.pl > flamegraph.svg

# Thread analysis
top -H -p $(pgrep only1mcp)

# System calls
strace -c -p $(pgrep only1mcp)
```

### CPU Profiling

```rust
// Add profiling code
#[cfg(feature = "profile")]
{
    let guard = pprof::ProfilerGuard::new(100)?;

    // Run workload

    if let Ok(report) = guard.report().build() {
        let file = File::create("flamegraph.svg")?;
        report.flamegraph(&mut BufWriter::new(file))?;
    }
}
```

## Configuration Issues

### Validation Errors

```bash
# Detailed validation
only1mcp validate --strict --verbose config.yaml

# Schema validation
ajv validate -s schema.json -d config.json

# YAML syntax check
yamllint config.yaml

# Environment variable expansion
envsubst < config.template.yaml > config.yaml
```

### Migration Issues

```bash
# Backup old config
cp config.yaml config.yaml.backup

# Migrate configuration
only1mcp config migrate \
    --from-version 0.9 \
    --to-version 1.0 \
    config.yaml.backup config.yaml

# Diff changes
diff -u config.yaml.backup config.yaml
```

## Recovery Procedures

### Emergency Recovery

```bash
#!/bin/bash
# emergency_recovery.sh

echo "Starting emergency recovery..."

# 1. Stop the service
systemctl stop only1mcp

# 2. Clear cache
redis-cli FLUSHALL

# 3. Reset circuit breakers
rm -f /var/lib/only1mcp/circuit_breakers.db

# 4. Restore last known good config
cp /backup/config.yaml.good /etc/only1mcp/config.yaml

# 5. Clear logs
truncate -s 0 /var/log/only1mcp/*.log

# 6. Start with minimal config
only1mcp start --config /etc/only1mcp/minimal.yaml &

# 7. Gradually enable features
sleep 30
only1mcp config enable cache
sleep 30
only1mcp config enable auth

echo "Recovery complete"
```

### Data Recovery

```bash
# Restore from backup
tar -xzf /backup/only1mcp-20240101.tar.gz -C /

# Rebuild cache
only1mcp cache rebuild

# Resync with backends
only1mcp sync --all

# Verify integrity
only1mcp test --suite integrity
```

## Getting Help

### Collecting Diagnostics

```bash
#!/bin/bash
# collect_diagnostics.sh

DIAG_DIR="/tmp/only1mcp-diag-$(date +%Y%m%d-%H%M%S)"
mkdir -p $DIAG_DIR

# System info
uname -a > $DIAG_DIR/system.txt
free -h >> $DIAG_DIR/system.txt
df -h >> $DIAG_DIR/system.txt

# Process info
ps aux | grep only1mcp > $DIAG_DIR/process.txt
lsof -p $(pgrep only1mcp) > $DIAG_DIR/connections.txt

# Configuration (sanitized)
only1mcp config show --sanitize > $DIAG_DIR/config.txt

# Recent logs
tail -n 1000 /var/log/only1mcp/error.log > $DIAG_DIR/error.log
tail -n 1000 /var/log/only1mcp/access.log > $DIAG_DIR/access.log

# Metrics snapshot
curl -s http://localhost:9091/metrics > $DIAG_DIR/metrics.txt

# Health status
only1mcp health status --json > $DIAG_DIR/health.json

# Create archive
tar -czf $DIAG_DIR.tar.gz -C /tmp $(basename $DIAG_DIR)
echo "Diagnostics collected: $DIAG_DIR.tar.gz"
```

### Support Channels

1. **GitHub Issues**: https://github.com/doublegate/Only1MCP/issues
   - Bug reports
   - Feature requests
   - Documentation issues

2. **Discord**: https://discord.gg/only1mcp
   - Real-time help
   - Community support
   - Development discussions

3. **Documentation**: https://docs.only1mcp.io
   - User guides
   - API reference
   - Best practices

4. **Commercial Support**: support@only1mcp.io
   - Enterprise support
   - Consulting services
   - Training

### Reporting Issues

When reporting issues, include:

1. **Version information:**
```bash
only1mcp version --build-info
```

2. **Configuration (sanitized):**
```bash
only1mcp config show --sanitize
```

3. **Error messages:**
```bash
tail -n 100 /var/log/only1mcp/error.log
```

4. **Steps to reproduce**

5. **Expected vs actual behavior**

6. **Diagnostics archive** (from collect_diagnostics.sh)