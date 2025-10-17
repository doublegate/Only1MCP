# /fix-failing-tests

Systematically investigate and fix failing tests in Only1MCP.

---

## 🎯 PURPOSE

Investigate test failures using Sequential Thinking, identify root causes, fix implementations, and ensure all tests pass.

---

## 📋 PHASE 1: IDENTIFY FAILING TESTS

### Step 1.1: Run Full Test Suite

```bash
# Run all tests with output
cargo test --workspace -- --nocapture 2>&1 | tee /tmp/test-output.txt

# Get summary
cargo test --workspace 2>&1 | tail -20
```

### Step 1.2: Parse Failures

Identify:
- Test names that failed
- Test file locations
- Failure messages
- Expected vs actual values

Extract failing test names:
```bash
grep "FAILED" /tmp/test-output.txt | awk '{print $2}'
```

---

## 📋 PHASE 2: INVESTIGATE EACH FAILURE (Sequential Thinking MCP)

For each failing test, use Sequential Thinking with 6+ thoughts:

```
Investigate [test_name] failure:

Thought 1: What does this test expect to happen?
  - Read test code to understand assertions
  - Identify expected values
  - Understand test setup

Thought 2: What is actually happening?
  - Parse failure output
  - Identify actual values returned
  - Note error messages

Thought 3: Where is the discrepancy?
  - Compare expected vs actual
  - Identify which assertion failed
  - Determine if setup or implementation issue

Thought 4: What is the root cause?
  - Trace through implementation
  - Identify incorrect logic
  - Check for edge cases not handled
  - Verify test assumptions valid

Thought 5: How to fix it?
  - Determine if implementation needs fixing
  - Or if test expectations are wrong
  - Consider side effects of fix
  - Verify fix won't break other tests

Thought 6: What additional tests are needed?
  - Identify gaps in test coverage
  - Consider edge cases not tested
  - Plan regression tests

Final diagnosis: [Root cause and fix strategy]
```

---

## 📋 PHASE 3: ANALYZE KNOWN FAILING TESTS

### Current Known Issues (from CLAUDE.local.md)

**6 cache tests failing** in `tests/response_caching.rs`:
- Known issues: stats tracking, layer routing, eviction

### Step 3.1: Read Test File

```bash
# Read failing test file
cat tests/response_caching.rs
```

### Step 3.2: Run Individual Tests

```bash
# Run each failing test individually to isolate
cargo test --test response_caching test_cache_stats_tracking -- --nocapture --exact
cargo test --test response_caching test_cache_layer_routing -- --nocapture --exact
cargo test --test response_caching test_cache_eviction -- --nocapture --exact
# ... repeat for all 6 failing tests
```

### Step 3.3: Analyze Implementation

Read relevant implementation files:
```bash
# Read cache implementation
cat src/cache/mod.rs

# Check related files
cat src/proxy/handler.rs | grep -A 20 "cache"
```

---

## 📋 PHASE 4: IMPLEMENT FIXES

### Step 4.1: Fix Implementation Issues

For each root cause identified:

1. **Locate the code** that needs fixing
2. **Read the file** first to understand context
3. **Make precise fix** addressing root cause
4. **Add comments** explaining why fix is needed
5. **Consider side effects** - ensure fix doesn't break other functionality

Example fix pattern:
```rust
// Before (buggy):
pub fn get_stats(&self) -> CacheStats {
    // Missing implementation or incorrect logic
}

// After (fixed):
pub fn get_stats(&self) -> CacheStats {
    CacheStats {
        hit_count: self.metrics.hits.load(Ordering::Relaxed),
        miss_count: self.metrics.misses.load(Ordering::Relaxed),
        // ... all fields properly populated
    }
}
```

### Step 4.2: Fix Test Issues (if tests are wrong)

If analysis shows test expectations are incorrect:

1. **Verify** implementation behavior is actually correct
2. **Update test** to match correct behavior
3. **Add comment** explaining why test was wrong
4. **Document** correct behavior in test name/doc comment

---

## 📋 PHASE 5: ADD ADDITIONAL TESTS

### Step 5.1: Identify Test Gaps

Based on Sequential Thinking analysis, identify:
- Edge cases not covered
- Error conditions not tested
- Integration scenarios missing
- Boundary conditions not validated

### Step 5.2: Write New Tests

For each gap:
```rust
#[tokio::test]
async fn test_[edge_case_description]() {
    // Arrange: Set up edge case scenario

    // Act: Execute the operation

    // Assert: Verify correct handling
}
```

---

## 📋 PHASE 6: VALIDATE ALL TESTS PASS

### Step 6.1: Run Full Test Suite

```bash
# Run all tests
cargo test --workspace

# Run with single thread if race conditions suspected
cargo test --workspace -- --test-threads=1
```

**Success Criteria**: All tests passing (XX/XX passing)

### Step 6.2: Verify Specific Test Categories

```bash
# Unit tests
cargo test --lib

# Integration tests
cargo test --test '*'

# Specific test file
cargo test --test response_caching

# Specific test
cargo test test_cache_stats_tracking -- --exact
```

### Step 6.3: Run Multiple Times (for flaky tests)

```bash
# Run 10 times to catch intermittent failures
for i in {1..10}; do
  echo "Run $i:"
  cargo test --workspace || break
done
```

---

## 📋 PHASE 7: UPDATE DOCUMENTATION

### Step 7.1: Update CLAUDE.local.md

Remove "Known Issues" entry for fixed tests:

```markdown
**Known Issue**: 6 cache tests failing (under investigation)
```

Replace with:

```markdown
**Resolved**: Fixed 6 cache test failures - stats tracking, layer routing, and eviction now working correctly
```

### Step 7.2: Update Test Count

If tests were added or removed, update test counts in:
- README.md badges
- CHANGELOG.md if significant
- CLAUDE.local.md Build Status section

---

## 📋 PHASE 8: GENERATE FIX REPORT

Provide comprehensive report:

```markdown
## Test Failure Investigation & Fix Report

**Date**: [Date and Time]
**Tests Analyzed**: [N] failing tests
**Tests Fixed**: [M] tests
**Tests Added**: [P] new tests
**Final Status**: [XX]/[XX] passing ✅

### Failing Tests Investigated

#### Test 1: `[test_name]`
- **Location**: [file:line]
- **Failure**: [Brief description of what failed]
- **Expected**: [What test expected]
- **Actual**: [What actually happened]
- **Root Cause**: [Technical explanation]
- **Fix Applied**: [How it was fixed]
- **Files Modified**: [List files changed]

#### Test 2: `[test_name]`
[Same structure]

... [Repeat for all failing tests]

### Implementation Fixes

**Files Modified**:
1. `[file path]` - [Changes made]
2. `[file path]` - [Changes made]

**Lines Changed**: [Total lines modified/added]

### Root Causes Summary

1. **[Category]**: [N] tests
   - [Brief explanation of root cause]
   - [Fix strategy applied]

2. **[Category]**: [M] tests
   - [Brief explanation]

### New Tests Added

1. `test_[name]` - [What it tests]
2. `test_[name]` - [What it tests]

**Total New Tests**: +[P]

### Test Suite Status

**Before Fixes**:
- Total: [XX] tests
- Passing: [YY] ([ZZ]%)
- Failing: [FF]

**After Fixes**:
- Total: [XX + P] tests
- Passing: [XX + P] ([100]%) ✅
- Failing: 0

### Validation Results

- ✅ All tests passing: [XX]/[XX]
- ✅ Flaky test check: Passed 10 consecutive runs
- ✅ No test timeouts
- ✅ No memory leaks detected

### Documentation Updates

- ✅ CLAUDE.local.md: Known Issues section updated
- ✅ Test counts updated in README.md
- ✅ No CHANGELOG update needed (test fixes are internal)

### Technical Insights

[Any architectural insights, patterns discovered, or recommendations for preventing similar issues]

### Next Steps

[Any follow-up actions needed, or "None - all tests passing"]
```

---

## ✅ SUCCESS CRITERIA

Tests are considered fixed when:

- ✅ All tests passing (cargo test shows XX/XX passing 100%)
- ✅ Tests pass consistently (10+ consecutive runs)
- ✅ Root causes identified and documented
- ✅ Implementation fixes applied correctly
- ✅ No side effects introduced (other tests still pass)
- ✅ Additional tests added for found gaps
- ✅ Documentation updated (CLAUDE.local.md)
- ✅ Comprehensive fix report provided

---

## 🔍 DEBUGGING TECHNIQUES

### For Stats Tracking Issues
- Check atomic counters initialized
- Verify load/store operations use correct Ordering
- Ensure counters incremented at right times

### For Layer Routing Issues
- Trace request path through cache layers
- Verify layer selection logic
- Check TTL and capacity configurations

### For Eviction Issues
- Confirm LRU policy correctly implemented
- Check eviction listener callbacks
- Verify entry removal on capacity exceeded

### For Race Conditions
- Run tests with `--test-threads=1`
- Add logging/tracing to implementation
- Check for missing synchronization

### For Intermittent Failures
- Run tests multiple times (10+ runs)
- Check for uninitialized state
- Verify test cleanup/teardown

---

**Execute this command to systematically fix all failing tests.**
