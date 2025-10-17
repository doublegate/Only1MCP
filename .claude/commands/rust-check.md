# /rust-check

Fast Rust quality check pipeline - format, lint, test, build verification.

---

## 🎯 PURPOSE

Execute comprehensive Rust quality checks in optimized phases (fast-fail → comprehensive → build).

---

## 📋 PHASE 0: VALIDATE PREREQUISITES

### Step 0.1: Check Rust Toolchain

```bash
if ! command -v cargo &> /dev/null; then
  echo "❌ ERROR: cargo not found"
  echo "Install Rust: https://rustup.rs"
  exit 1
fi

if [ ! -f "Cargo.toml" ]; then
  echo "❌ ERROR: Not in a Rust project (no Cargo.toml)"
  exit 1
fi

echo "✅ Prerequisites validated"
echo ""
```

### Step 0.2: Check Required Tools

```bash
MISSING_TOOLS=()
! command -v rustfmt &> /dev/null && MISSING_TOOLS+=("rustfmt")
! command -v cargo-clippy &> /dev/null && MISSING_TOOLS+=("clippy")

if [ ${#MISSING_TOOLS[@]} -gt 0 ]; then
  echo "⚠️  WARNING: Missing tools (install with rustup component add):"
  printf '  - %s\n' "${MISSING_TOOLS[@]}"
  echo ""
fi
```

---

## 📋 PHASE 1: FAST-FAIL QUALITY CHECKS

**Objective**: Catch formatting and lint issues as quickly as possible before running expensive tests.

### Step 1.1: Format Check

```bash
echo "Running format check..."
cargo fmt --all -- --check
```

**Expected**: Zero formatting issues
**On Failure**: Run `cargo fmt --all` to auto-fix

### Step 1.2: Clippy Lint

```bash
echo "Running clippy lint..."
cargo clippy --all-targets --all-features -- -D warnings
```

**Expected**: Zero clippy warnings (or <5 non-critical)
**On Failure**: Review clippy suggestions and fix issues

---

## 📋 PHASE 2: COMPREHENSIVE TEST SUITE

**Objective**: Run full test suite to ensure all functionality works.

### Step 2.1: Execute Tests

```bash
echo "Running test suite..."
cargo test --workspace
```

**Expected**: All tests passing
**Current Status**: Target ~52 tests (may vary as features added)

### Step 2.2: Detailed Test Breakdown (if failures)

```bash
# Unit tests
cargo test --workspace --lib

# Integration tests
cargo test --workspace --test '*'

# Doc tests
cargo test --workspace --doc
```

**Duration**: ~30-60 seconds (varies by feature count)

---

## 📋 PHASE 3: RELEASE BUILD VERIFICATION

**Objective**: Verify optimized release build compiles without warnings.

### Step 3.1: Clean Release Build

```bash
echo "Building release binary..."
cargo build --release --all-targets
```

**Expected**: Zero errors, <5 warnings
**Duration**: ~30-60 seconds

### Step 3.2: Binary Size Check

```bash
if [ -f "target/release/only1mcp" ]; then
  echo ""
  echo "Binary size:"
  ls -lh target/release/only1mcp
fi
```

---

## 📊 SUCCESS CRITERIA

✅ **All phases must pass:**
- Phase 1: Zero formatting issues, zero clippy errors
- Phase 2: All tests passing (XX/XX = 100%)
- Phase 3: Release build successful

✅ **Performance Benchmarks:**
- Phase 1: <10 seconds (fast-fail)
- Phase 2: <60 seconds (test suite)
- Phase 3: <60 seconds (release build)
- **Total**: <3 minutes end-to-end

---

## 🚨 FAILURE HANDLING

### Format Failures (Phase 1.1)

**Action**: Run `cargo fmt --all` to auto-fix
**Report**: Display file paths with formatting violations

### Clippy Failures (Phase 1.2)

**Action**: Display clippy suggestions with fix hints
**Common Issues**:
- Unused variables (prefix with `_`)
- Unnecessary clones (use references)
- Inefficient algorithms (use clippy suggestions)

### Test Failures (Phase 2)

**Action**: Display failed test names and error messages
**Debug Commands**:
```bash
# Show stdout/stderr
cargo test --workspace -- --nocapture

# Run specific test
cargo test test_name -- --exact

# Run with logging
RUST_LOG=debug cargo test test_name
```

**Next Steps**: Use /fix-failing-tests command for systematic investigation

### Build Failures (Phase 3)

**Action**: Display compiler errors with line numbers
**Common Issues**:
- Type mismatches (check function signatures)
- Lifetime issues (review borrowing)
- Missing dependencies (check Cargo.toml)

---

## 🎯 USAGE PATTERNS

**Full Pipeline**: `/rust-check` (runs all phases)
**Pre-Commit**: Run before every git commit
**Pre-Push**: Run before every git push
**Post-Feature**: Run after implementing new features

---

## 🔗 RELATED COMMANDS

- `/fix-failing-tests` - Systematic test debugging
- `/phase-commit` - Create comprehensive commit (after rust-check passes)
- `/update-docs` - Synchronize documentation
- `/next-phase-feature` - Includes rust-check in validation phase

**Workflow Integration**:
1. Implement feature
2. Run `/rust-check` → Fix issues → Re-run
3. Run `/update-docs` → Sync documentation
4. Run `/phase-commit` → Create detailed commit

---

## 📋 DELIVERABLES

**On Success**:
1. **Quality Report**: All checks passed, zero issues found
2. **Test Summary**: XX/XX tests passing
3. **Build Artifacts**: Release binary in `target/release/only1mcp`

**On Failure**:
1. **Failure Report**: Which phase failed, specific errors
2. **Fix Commands**: Suggested commands to resolve issues
3. **Investigation Guide**: How to debug the specific failure type

---

**Execute this comprehensive quality pipeline before every commit.**
