# Phase 1 MVP Todo List

**Generated:** October 16, 2025
**Project:** Only1MCP - High-Performance MCP Server Aggregator
**Current Status:** cargo check ✅ PASSES | cargo build ⚠️ 76 errors
**Target:** Fully compiling MVP with integration tests
**Estimated Total Effort:** 16.5 hours

---

## 🔥 CRITICAL PATH (Must Complete for MVP)

### TASK 1: Fix Config Struct - Add Missing Routing Field
**Priority:** CRITICAL
**Effort:** 30 minutes
**File:** `src/config/mod.rs`
**Dependencies:** None
**Blocks:** Tasks 2, 4

**Problem:**
- ProxyConfig missing `routing` field causes 8+ E0609 errors
- Handlers trying to access `config.proxy.routing.algorithm`

**Actions:**
1. Read current `src/config/mod.rs` ProxyConfig struct
2. Add `routing: RoutingConfig` field with proper serde defaults
3. Create RoutingConfig struct with:
   - `algorithm: String` (default: "round_robin")
   - `virtual_nodes: usize` (default: 150)
   - `sticky_sessions: bool` (default: false)
4. Update Default impl for ProxyConfig
5. Run `cargo check` to verify 8 fewer errors

**Success Criteria:**
- ✅ `config.proxy.routing` field accessible
- ✅ E0609 errors reduced from ~8 to 0 for routing field
- ✅ cargo check shows improvement

**Reference:** `ref_docs/06-Only1MCP_Configuration_Guide.md` (routing section)

---

### TASK 2: Implement ServerRegistry Methods
**Priority:** CRITICAL
**Effort:** 2 hours
**File:** `src/proxy/registry.rs`
**Dependencies:** Task 1 (Config fix)
**Blocks:** Task 4 (Handlers)

**Problem:**
- ServerRegistry struct exists but methods are stubs
- 15+ E0599 errors for missing methods
- Handlers can't access server info

**Actions:**
1. Read current `src/proxy/registry.rs` implementation
2. Implement `get_healthy_servers()` method:
   ```rust
   pub async fn get_healthy_servers(&self) -> Vec<String> {
       // Query health checker for healthy server IDs
       // Return filtered list
   }
   ```
3. Implement `get_server(&self, id: &str) -> Option<ServerInfo>`:
   ```rust
   // Look up server by ID from internal HashMap
   ```
4. Implement `list_servers(&self) -> Vec<ServerInfo>`:
   ```rust
   // Return all registered servers
   ```
5. Implement `add_server(&mut self, server: ServerInfo)`:
   ```rust
   // Add new server to registry
   // Update consistent hash ring
   ```
6. Implement `remove_server(&mut self, id: &str)`:
   ```rust
   // Remove server from registry
   // Update hash ring
   ```
7. Update ServerInfo struct with all necessary fields
8. Run `cargo check` to verify E0599 errors reduced

**Success Criteria:**
- ✅ All methods compile without errors
- ✅ E0599 errors reduced by ~15
- ✅ Methods return correct types
- ✅ Integration with health checking works

**Reference:** `ref_docs/14` section on server registry, `src/proxy/registry.rs` existing structure

---

### TASK 3: Fix Metrics Access Patterns
**Priority:** CRITICAL
**Effort:** 1 hour
**File:** `src/proxy/handler.rs`, `src/metrics/mod.rs`
**Dependencies:** None (can run parallel with Task 1-2)
**Blocks:** Task 4

**Problem:**
- E0615 errors: accessing metric fields as if they were methods
- Code has `metrics.cache_hits.inc()` but should be `metrics.cache_hits().inc()`
- 6+ E0615 errors

**Actions:**
1. Read `src/metrics/mod.rs` to understand metric structure
2. Find all instances of incorrect metric access:
   ```bash
   grep -r "metrics\.\w\+\.\(inc\|record\|set\)" src/
   ```
3. Fix patterns in `src/proxy/handler.rs`:
   - Change `state.metrics.cache_hits.inc()` → `state.metrics.cache_hits().inc()`
   - Change `state.metrics.tools_list_duration.record()` → `state.metrics.tools_list_duration().record()`
   - Same for all histogram, counter, gauge accesses
4. Verify metrics are actually declared in lazy_static! block
5. Run `cargo check` to verify E0615 errors resolved

**Success Criteria:**
- ✅ All E0615 errors resolved
- ✅ Metrics recording uses correct syntax
- ✅ Metrics actually increment during requests

**Reference:** `ref_docs/14` metrics section, `src/metrics/mod.rs` lazy_static declarations

---

### TASK 4: Complete Handler Implementations
**Priority:** CRITICAL
**Effort:** 3 hours
**File:** `src/proxy/handler.rs`
**Dependencies:** Tasks 1, 2, 3
**Blocks:** Task 5 (Tests)

**Problem:**
- Handler helper functions are stubs returning empty results
- No actual backend communication
- Missing error handling

**Actions:**
1. Implement `fetch_tools_from_server()`:
   ```rust
   async fn fetch_tools_from_server(
       state: AppState,
       server_id: String,
       request: McpRequest,
   ) -> Result<Vec<Tool>> {
       // Get server from registry
       // Get transport for server
       // Send tools/list request
       // Parse response
       // Return tools
   }
   ```
2. Implement `send_request_to_backend()`:
   ```rust
   async fn send_request_to_backend(
       state: AppState,
       server: ServerConfig,
       request: McpRequest,
   ) -> Result<Value> {
       // Match on transport type
       // For HTTP: use HttpTransport
       // For STDIO: use StdioTransport
       // Send request with timeout
       // Handle errors
       // Return response
   }
   ```
3. Implement `fetch_resources_from_server()` (similar to tools)
4. Implement `fetch_prompts_from_server()` (similar to tools)
5. Add proper error conversion from transport errors to ProxyError
6. Add timeout handling (5s default)
7. Add request tracing with span IDs
8. Test with actual MCP server (use filesystem MCP for testing)

**Success Criteria:**
- ✅ Can send request to STDIO backend successfully
- ✅ Can send request to HTTP backend successfully
- ✅ Errors are properly converted to JSON-RPC error format
- ✅ Timeouts work correctly
- ✅ At least one end-to-end request completes

**Reference:**
- `ref_docs/14` handler implementation section
- `ref_docs/01-MCP_Protocol_Specification.md` for JSON-RPC format
- `src/transport/http.rs` and `src/transport/stdio.rs` for transport usage

---

### TASK 5: Fix Remaining Type Issues
**Priority:** CRITICAL
**Effort:** 1 hour
**Files:** `src/auth/oauth.rs`, `src/health/circuit_breaker.rs`, `src/proxy/router.rs`
**Dependencies:** Tasks 1-4
**Blocks:** Task 6 (Compilation)

**Problem:**
- E0599: TokenInfo missing Clone derive
- E0599: CircuitBreaker methods not accessible via Arc
- E0599: ConsistentHashRing.add_node() missing or wrong signature
- E0277: Type conversion errors for ProxyError/RoutingError

**Actions:**
1. Add `#[derive(Clone)]` to TokenInfo in `src/auth/oauth.rs`
2. Fix CircuitBreakerManager methods:
   - Ensure `trip()` method exists and is public
   - Check Arc wrapper isn't hiding methods
3. Fix ConsistentHashRing in `src/routing/load_balancer.rs`:
   - Verify `add_node()` signature matches usage
   - Should take `add_node(&mut self, node: String)` or similar
4. Fix error conversions:
   - Add `From<Error>` for `RoutingError`
   - Add `From<Error>` for `ProxyError`
   - Ensure `?` operator works in all contexts
5. Fix HealthState access:
   - Add methods to HealthState: `new()`, `record_success()`, `record_failure()`, `average_latency()`
   - Ensure methods work when accessed via DashMap Ref/RefMut

**Success Criteria:**
- ✅ All E0599 errors resolved
- ✅ All E0277 errors resolved
- ✅ Type conversions work with `?` operator
- ✅ No trait bound errors

**Reference:** Error definitions in `src/error.rs`

---

### TASK 6: Achieve Full Compilation
**Priority:** CRITICAL
**Effort:** 1.5 hours
**Files:** Various
**Dependencies:** Tasks 1-5
**Blocks:** Task 7 (Tests)

**Problem:**
- 76 compilation errors remaining
- Some may be cascading from earlier fixes

**Actions:**
1. After Tasks 1-5, run full build: `cargo build 2>&1 | tee /tmp/build_errors.txt`
2. Categorize remaining errors by type
3. Fix errors in dependency order (dependencies first)
4. For each error:
   - Understand root cause
   - Check if it's a cascading error from earlier issues
   - Implement fix
   - Verify with `cargo check`
5. Run `cargo clippy` and fix critical warnings
6. Run `cargo fmt --check` and format if needed

**Success Criteria:**
- ✅ `cargo build --release` completes with 0 errors
- ✅ `cargo clippy -- -D warnings` passes
- ✅ `cargo fmt --check` passes
- ✅ All modules compile successfully

---

### TASK 7: Basic Integration Test
**Priority:** CRITICAL
**Effort:** 2 hours
**File:** `tests/integration/basic_routing.rs`
**Dependencies:** Task 6 (Compilation)
**Blocks:** MVP completion

**Problem:**
- No integration tests exist
- Need to verify end-to-end functionality

**Actions:**
1. Create `tests/integration/` directory
2. Create test helper: `tests/common/mod.rs`:
   ```rust
   // Mock MCP server setup
   // Test config generation
   // Async test utilities
   ```
3. Implement `tests/integration/basic_routing.rs`:
   ```rust
   #[tokio::test]
   async fn test_server_starts_and_binds() {
       // Start proxy server
       // Verify it binds to port
       // Verify health endpoint responds
   }

   #[tokio::test]
   async fn test_tools_list_aggregation() {
       // Start 2 mock MCP servers
       // Start proxy
       // Send tools/list request
       // Verify response aggregates both servers
   }

   #[tokio::test]
   async fn test_tools_call_routing() {
       // Start mock MCP server
       // Start proxy
       // Send tools/call request
       // Verify request routed correctly
       // Verify response returned
   }
   ```
4. Use wiremock for HTTP backend mocking
5. Use simple echo process for STDIO testing
6. Run tests: `cargo test --test basic_routing`

**Success Criteria:**
- ✅ At least 3 tests pass
- ✅ Tests complete in < 30 seconds
- ✅ One end-to-end flow verified
- ✅ Coverage shows handlers are exercised

**Reference:** `ref_docs/15-Only1MCP_Testing_Strategy.md`

---

## 🟡 HIGH PRIORITY (Should Complete for Quality MVP)

### TASK 8: Config Loading Integration Tests
**Priority:** HIGH
**Effort:** 1 hour
**File:** `tests/integration/config_loading.rs`
**Dependencies:** Task 6

**Actions:**
1. Test YAML config loading
2. Test TOML config loading
3. Test validation errors
4. Test default values
5. Test environment variable substitution

**Success Criteria:**
- ✅ 5+ config tests pass
- ✅ Invalid configs rejected properly

---

### TASK 9: Handler Integration Tests
**Priority:** HIGH
**Effort:** 1.5 hours
**File:** `tests/integration/handlers.rs`
**Dependencies:** Task 7

**Actions:**
1. Test resources/list aggregation
2. Test resources/read routing
3. Test prompts/list aggregation
4. Test error handling scenarios
5. Test timeout handling

**Success Criteria:**
- ✅ 8+ handler tests pass
- ✅ Error cases properly tested

---

### TASK 10: Fix All Clippy Warnings
**Priority:** HIGH
**Effort:** 1 hour
**Files:** Various
**Dependencies:** Task 6

**Actions:**
1. Run `cargo clippy --all-targets` to see all warnings
2. Fix unused imports (currently 17 warnings)
3. Fix any performance warnings
4. Fix any correctness warnings
5. Add `#[allow(dead_code)]` only where justified

**Success Criteria:**
- ✅ `cargo clippy --all-targets -- -D warnings` passes with 0 warnings
- ✅ No `#[allow(unused)]` attributes used unnecessarily

---

### TASK 11: Update Documentation
**Priority:** HIGH
**Effort:** 1.5 hours
**Files:** `docs/PHASE_1_PLAN.md`, `CLAUDE.local.md`, `CHANGELOG.md`, `README.md`
**Dependencies:** Tasks 1-10

**Actions:**
1. Update `docs/PHASE_1_PLAN.md`:
   - Check off all completed items
   - Mark Week 1-4 progress
2. Update `CLAUDE.local.md`:
   - Update build status to ✅ passing
   - Update phase progress to 100%
   - Document final decisions
3. Create `CHANGELOG.md` entry for v0.1.0-beta:
   - List all features implemented
   - Note known limitations
4. Update `README.md`:
   - Add quick start instructions
   - Add example configuration
   - Add build/run instructions

**Success Criteria:**
- ✅ All Phase 1 checkboxes marked complete
- ✅ Documentation reflects current state
- ✅ CHANGELOG entry is clear and complete

---

### TASK 12: Basic Performance Validation
**Priority:** HIGH
**Effort:** 1 hour
**File:** `benches/basic_proxy.rs`
**Dependencies:** Task 7

**Actions:**
1. Create basic benchmark in `benches/basic_proxy.rs`
2. Measure request latency overhead
3. Measure throughput (requests/second)
4. Measure memory usage
5. Document results in `docs/BENCHMARK_RESULTS.md`

**Success Criteria:**
- ✅ Latency overhead < 5ms (p99)
- ✅ Throughput > 1,000 req/s
- ✅ Memory usage < 50MB for 10 servers

---

## 🟢 MEDIUM PRIORITY (Nice to Have)

### TASK 13: Advanced Integration Tests
**Priority:** MEDIUM
**Effort:** 1.5 hours
**File:** `tests/integration/advanced.rs`
**Dependencies:** Task 9

**Actions:**
1. Test failover scenarios
2. Test load balancing distribution
3. Test circuit breaker behavior
4. Test cache hit/miss scenarios
5. Test concurrent requests

**Success Criteria:**
- ✅ 10+ advanced tests pass
- ✅ Failover works correctly
- ✅ Load balancing distributes evenly

---

### TASK 14: Hot-Reload Testing
**Priority:** MEDIUM
**Effort:** 1 hour
**File:** `tests/integration/hot_reload.rs`
**Dependencies:** Task 6

**Actions:**
1. Test config file modification detection
2. Test server list updates without restart
3. Test configuration validation on reload
4. Test rollback on invalid config

**Success Criteria:**
- ✅ Config changes detected within 1 second
- ✅ No downtime during reload
- ✅ Invalid configs don't crash server

---

### TASK 15: Comprehensive Error Scenario Tests
**Priority:** MEDIUM
**Effort:** 30 minutes
**File:** `tests/integration/error_scenarios.rs`
**Dependencies:** Task 9

**Actions:**
1. Test backend timeout handling
2. Test backend unavailable scenarios
3. Test malformed request handling
4. Test rate limit scenarios
5. Test authentication failures

**Success Criteria:**
- ✅ 8+ error scenario tests pass
- ✅ Proper error codes returned
- ✅ No panics on bad input

---

## 🔵 LOW PRIORITY (Can Defer to Phase 2)

### TASK 16: WebSocket Transport Implementation
**Priority:** LOW (Phase 2)
**Effort:** 4 hours
**File:** `src/transport/websocket.rs`
**Status:** DEFERRED

**Justification:** Phase 1 MVP focuses on STDIO and HTTP transports. WebSocket is Phase 2 enhancement.

---

### TASK 17: SSE Transport Implementation
**Priority:** LOW (Phase 2)
**Effort:** 3 hours
**File:** `src/transport/sse.rs`
**Status:** DEFERRED

**Justification:** SSE is Phase 2 enhancement for streaming responses.

---

## 📊 PROGRESS TRACKING

### Critical Path Completion
- [ ] Task 1: Config Fix (30 min)
- [ ] Task 2: ServerRegistry (2h)
- [ ] Task 3: Metrics Fix (1h)
- [ ] Task 4: Handlers (3h)
- [ ] Task 5: Type Issues (1h)
- [ ] Task 6: Compilation (1.5h)
- [ ] Task 7: Basic Tests (2h)

**Critical Path Total:** 11 hours

### High Priority Completion
- [ ] Task 8: Config Tests (1h)
- [ ] Task 9: Handler Tests (1.5h)
- [ ] Task 10: Clippy (1h)
- [ ] Task 11: Documentation (1.5h)
- [ ] Task 12: Benchmarks (1h)

**High Priority Total:** 6 hours

### Overall Progress
**Completed:** 0 / 15 active tasks
**Estimated Remaining:** 17 hours
**Phase 1 Status:** 65% → Target: 100%

---

## 🎯 SUCCESS CRITERIA FOR MVP COMPLETION

### Build & Compilation
- ✅ `cargo build --release` succeeds with 0 errors
- ✅ `cargo clippy --all-targets -- -D warnings` passes
- ✅ `cargo fmt --check` passes
- ✅ No compilation warnings except explicitly allowed

### Testing
- ✅ `cargo test` passes with ≥ 10 tests
- ✅ At least one end-to-end integration test
- ✅ Test coverage ≥ 60% (target: 80%)
- ✅ All tests run in < 1 minute

### Functionality
- ✅ Server starts and binds to configured port
- ✅ Can aggregate tools from 2+ MCP servers
- ✅ Can route tool calls to correct backend
- ✅ Health checking identifies unhealthy servers
- ✅ Configuration loading from YAML works
- ✅ Basic error handling returns proper JSON-RPC errors

### Documentation
- ✅ All Phase 1 checklist items marked complete
- ✅ README has quick start instructions
- ✅ CHANGELOG entry for v0.1.0-beta
- ✅ CLAUDE.local.md reflects final state

### Performance
- ✅ Latency overhead < 5ms (p99)
- ✅ Throughput ≥ 1,000 req/s
- ✅ Memory usage < 100MB baseline

---

## 📝 NOTES & CONVENTIONS

### Before Starting Each Task
1. Read relevant reference documentation
2. Read current file implementation
3. Understand dependencies and blockers
4. Plan approach before coding

### During Implementation
1. Run `cargo check` frequently (after every few changes)
2. Test incrementally (don't wait until task complete)
3. Keep changes focused (one logical change at a time)
4. Add comments for complex logic

### After Completing Each Task
1. Run full test suite: `cargo test`
2. Verify success criteria met
3. Update this todo list (mark complete)
4. Update CLAUDE.local.md with progress
5. Commit changes with descriptive message

### Error Debugging Strategy
1. Read error message carefully
2. Identify error type (E0599, E0277, etc.)
3. Look at surrounding context
4. Check reference documentation
5. Fix root cause, not just symptoms

---

## 🔗 QUICK REFERENCE LINKS

### Key Files
- Config: `src/config/mod.rs`
- Registry: `src/proxy/registry.rs`
- Handlers: `src/proxy/handler.rs`
- Metrics: `src/metrics/mod.rs`
- Errors: `src/error.rs`

### Documentation
- Phase 1 Plan: `docs/PHASE_1_PLAN.md`
- Architecture: `docs/ARCHITECTURE.md`
- Implementation Guide: `ref_docs/14-Only1MCP_Core_Proxy_Implementation_Guide.md`
- MCP Protocol: `ref_docs/01-MCP_Protocol_Specification.md`
- Testing Strategy: `ref_docs/15-Only1MCP_Testing_Strategy.md`

### Commands
```bash
# Quick compile check
cargo check

# Full build
cargo build --release

# Run tests
cargo test

# Run clippy
cargo clippy --all-targets -- -D warnings

# Format code
cargo fmt

# Run benchmarks
cargo bench

# Generate docs
cargo doc --open
```

---

**Last Updated:** October 16, 2025
**Next Review:** After Task 7 completion
**Maintained By:** Only1MCP Development Team
