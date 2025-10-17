# /next-phase-feature

Execute next Phase 4 enhancement sprint with full workflow automation (planning, implementation, testing, documentation, commit).

---

## 🎯 MISSION OVERVIEW

**Purpose**: Implement the next Phase 2 feature from PHASE_2_MASTER_PLAN.md with complete workflow automation.

**Expected Duration**: 6-16 hours (varies by feature complexity)

**MCP Server Usage**: MANDATORY use of Sequential Thinking, Context7, and Memory throughout workflow.

---

## 📋 PHASE 0: CONTEXT GATHERING

### Step 0.1: Read Project State

Read these files to understand current state:

```bash
# Current project state
cat to-dos/Phase_2/PHASE_2_MASTER_PLAN.md

# Session history and decisions
cat CLAUDE.local.md

# Recent commits for pattern understanding
git log --oneline -5
```

### Step 0.2: Identify Next Feature

Analyze PHASE_2_MASTER_PLAN.md to determine which feature is next:

- Feature 1: Configuration Hot-Reload → ✅ COMPLETE (d8e499b)
- Feature 2: Active Health Checking → ✅ COMPLETE (64cd843)
- Feature 3: Response Caching TTL/LRU → ✅ COMPLETE (6391c78)
- Feature 4: Request Batching → **NEXT** (if not started)
- Feature 5: TUI Interface → Pending
- Feature 6: Performance Benchmarking → Pending

Identify the next uncompleted feature and note its:
- Priority level
- Complexity estimate
- Dependencies
- Estimated time
- Technical approach

---

## 📋 PHASE 1: ANALYSIS & DESIGN (Sequential Thinking MCP)

### Step 1.1: Use Sequential Thinking for Design

**MANDATORY**: Use Sequential Thinking MCP with 15+ thoughts minimum.

```
Analyze [Feature Name] implementation:

Thought 1: What is the core purpose of this feature?
Thought 2: What are the key technical requirements from PHASE_2_MASTER_PLAN.md?
Thought 3: What dependencies are needed? (crates to add to Cargo.toml)
Thought 4: What modules/files need creation or modification?
Thought 5: How does this integrate with existing components?
Thought 6: What data structures are needed?
Thought 7: What are the performance implications?
Thought 8: What edge cases must be handled?
Thought 9: What testing strategy is appropriate? (unit, integration, edge cases)
Thought 10: What metrics should be tracked? (Prometheus)
Thought 11: How should configuration be structured? (YAML schema)
Thought 12: What error conditions exist and how to handle them?
Thought 13: What are the security considerations?
Thought 14: How should this be documented? (README, CHANGELOG, inline docs)
Thought 15: What are the acceptance criteria for "complete"?
Thought 16+: [Continue analysis as needed for complex features]

Generate hypothesis: "[Feature] should be implemented using [approach] because [reasons]"
Verify: [Check hypothesis against requirements and constraints]
Final design: [Detailed implementation plan with components, interfaces, data flow]
```

**Output**: Comprehensive implementation plan with all design decisions documented.

### Step 1.2: Document Design Decisions

Record key architectural decisions identified during Sequential Thinking:

- Chosen approach and rationale
- Crates/libraries to use
- Module structure
- Integration points
- Performance considerations
- Security implications

---

## 📋 PHASE 2: RESEARCH (Context7 MCP)

### Step 2.1: Research Necessary Crates

For each crate identified in Phase 1, use Context7 MCP to research:

**Example for crate research**:
```
Use Context7 to fetch documentation for [crate-name]:
- Best practices for [specific use case]
- Common patterns and examples
- Performance characteristics
- Error handling approaches
- Integration with tokio/async
```

### Step 2.2: Study Similar Implementations

If PHASE_2_MASTER_PLAN.md references similar features or patterns:
- Read relevant sections of master plan
- Check existing codebase for similar patterns
- Review ref_docs/ for implementation guides

---

## 📋 PHASE 3: IMPLEMENTATION

### Step 3.1: Add Dependencies to Cargo.toml

Add all required crates with justification comments:

```toml
# Feature [N]: [Feature Name]
[dependency-name] = "[version]"  # [Justification: what it's used for]
```

Verify dependencies compile:
```bash
cargo check
```

### Step 3.2: Create/Modify Necessary Files

Follow the implementation plan from Phase 1. For each file:

1. **Read existing file first** (if modifying)
2. **Implement fully** - NO stubs, NO TODOs, NO placeholders
3. **Add comprehensive inline documentation** (rustdoc comments)
4. **Handle all edge cases** identified in Phase 1
5. **Add error handling** for all failure modes

**Critical**: Implementation must be 100% complete before moving to testing.

### Step 3.3: Integration with Existing Components

Integrate the new feature with:
- ProxyServer (src/proxy/server.rs)
- Configuration system (src/config/mod.rs)
- Metrics system (src/metrics/mod.rs)
- Any other relevant components

---

## 📋 PHASE 4: TESTING

### Step 4.1: Write Comprehensive Unit Tests

**Minimum 6 test cases** covering:

1. **Happy path**: Feature works correctly with valid inputs
2. **Edge case 1**: Boundary conditions
3. **Edge case 2**: Empty/null/zero values
4. **Error handling**: Invalid inputs handled gracefully
5. **Integration**: Works with related components
6. **Concurrency**: Thread-safe if applicable

```rust
// tests/[feature_name].rs or in-module tests
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_[feature]_happy_path() {
        // Arrange
        // Act
        // Assert
    }

    #[tokio::test]
    async fn test_[feature]_edge_case_1() {
        // Test boundary conditions
    }

    // ... 4+ more tests
}
```

### Step 4.2: Write Integration Tests

Create integration test file if needed:

```bash
# tests/[feature_name]_integration.rs
```

Integration tests should validate:
- Feature works in realistic scenarios
- Interacts correctly with other components
- Configuration changes apply correctly
- Metrics are recorded properly

### Step 4.3: Validate All Tests Pass

```bash
cargo test --workspace
```

**Success Criteria**: All tests passing (X/X passing where X includes new tests).

If tests fail:
- Debug and fix immediately
- Do NOT proceed to documentation with failing tests
- Add additional tests if gaps discovered

---

## 📋 PHASE 5: DOCUMENTATION

### Step 5.1: Update README.md

Add feature to Phase 2 Features subsection:

```markdown
#### Phase 2 Features

- ✅ **Configuration Hot-Reload** - Zero-downtime config updates
- ✅ **Active Health Checking** - Proactive backend monitoring
- ✅ **Response Caching (TTL/LRU)** - Automatic cache management
- ✅ **[New Feature Name]** - [One-line description]
```

Update Project Status section with feature details.

Update test count badges if changed:
```markdown
![Tests](https://img.shields.io/badge/tests-XX%2FXX%20passing-green)
```

### Step 5.2: Update CHANGELOG.md

Add comprehensive entry to [0.2.0-dev] section:

```markdown
### Added
- **[Feature Name]**: [Detailed description]
  - [Sub-feature or component 1]
  - [Sub-feature or component 2]
  - [Performance characteristics]
  - [Configuration options]
```

### Step 5.3: Create Feature-Specific Documentation (if needed)

For complex features, create docs/[feature_name].md with:
- Feature overview
- Configuration guide
- Usage examples
- API reference
- Troubleshooting

### Step 5.4: Add Inline Documentation

Ensure all public APIs have rustdoc comments:

```rust
/// [Brief description]
///
/// # Arguments
/// * `param1` - [Description]
///
/// # Returns
/// [Description of return value]
///
/// # Errors
/// [When errors occur]
///
/// # Examples
/// ```
/// [Usage example]
/// ```
pub fn feature_function(param1: Type) -> Result<ReturnType, Error> {
    // Implementation
}
```

---

## 📋 PHASE 6: MEMORY TRACKING (Memory MCP)

### Step 6.1: Create Feature Implementation Entity

```
Use Memory MCP to create entity:

Entity Name: "Phase2_Feature[N]_[FeatureName]_Implementation"
Entity Type: "FeatureImplementation"
Observations:
- Feature: [Feature Name and number]
- Implementation approach: [Brief technical summary]
- Files created: [List with line counts]
- Files modified: [List with changes]
- Dependencies added: [List with justifications]
- Crates used: [List of new crates]
- Testing: [Test count and coverage summary]
- Performance: [Metrics and characteristics]
- Integration points: [How it connects to existing code]
```

### Step 6.2: Create Technical Decisions Entity

```
Use Memory MCP to create entity:

Entity Name: "Phase2_Feature[N]_TechnicalDecisions"
Entity Type: "ArchitecturalDecision"
Observations:
- Decision 1: [What was decided and why]
- Decision 2: [Alternative considered and why rejected]
- Decision 3: [Trade-off analysis]
- Performance considerations: [Optimization decisions]
- Security considerations: [Security design choices]
- Testing strategy: [Why this testing approach]
- Documentation approach: [Documentation decisions]
- Future considerations: [Extensibility or tech debt notes]
```

### Step 6.3: Create Edge Cases/Issues Entity (if applicable)

```
Use Memory MCP to create entity:

Entity Name: "Phase2_Feature[N]_EdgeCases"
Entity Type: "KnowledgeBase"
Observations:
- Edge case 1: [Description and how handled]
- Edge case 2: [Description and solution]
- Known limitation 1: [What and why]
- Known limitation 2: [Mitigation strategy]
- Potential issue 1: [Future consideration]
- Testing challenge 1: [How overcome]
```

---

## 📋 PHASE 7: VALIDATION

### Step 7.1: Run Comprehensive Quality Checks

```bash
# Format check
cargo fmt --check

# Linting
cargo clippy --all-targets --all-features -- -D warnings

# Full test suite
cargo test --workspace

# Release build
cargo build --release
```

**Success Criteria**:
- ✅ cargo fmt: No formatting issues
- ✅ cargo clippy: <5 warnings total
- ✅ cargo test: All tests passing (X/X passing)
- ✅ cargo build --release: Successful, no errors

### Step 7.2: Verify Documentation Complete

Checklist:
- [ ] README.md updated with feature
- [ ] CHANGELOG.md has comprehensive entry
- [ ] Inline rustdoc comments complete
- [ ] Feature-specific docs created (if needed)
- [ ] All code examples compile

### Step 7.3: Verify Memory Entities Created

Checklist:
- [ ] Implementation entity created with 8+ observations
- [ ] Technical decisions entity created with 6+ observations
- [ ] Edge cases entity created (if applicable)
- [ ] All entities readable with read_graph

---

## 📋 PHASE 8: COMPLETION REPORTING

### Step 8.1: Update CLAUDE.local.md

Add session summary for feature completion:

```markdown
#### Session [N] ([Date]) - **PHASE 2 FEATURE [N] COMPLETE** 🎉
1. **Feature [N]: [Feature Name]** ([commit hash])
   - [Key implementation detail 1]
   - [Key implementation detail 2]
   - [Test count]: +[X] tests ([previous]→[new])
   - [Metrics added]: [count] new metrics
   - [Documentation status]
   - **Known Issues**: [Any issues or none]
```

Update Phase Progress section:
```markdown
- ✅ **Feature [N]: [Feature Name]** (COMPLETE - [Date])
  - ✅ [Sub-feature 1]
  - ✅ [Sub-feature 2]
  - ✅ [N] comprehensive tests
  - ✅ Prometheus metrics integrated
  - ✅ Full documentation
  - Commit: [hash]
```

Update Build Status section with latest test results.

### Step 8.2: Generate Feature Completion Report

Provide comprehensive report to user:

```markdown
## Feature [N] Implementation Complete 🎉

### Feature: [Feature Name]

**Status**: ✅ 100% COMPLETE
**Duration**: [X] hours
**Commit**: [hash] (ready for commit, not yet committed)

### Implementation Summary

[2-3 paragraph summary of what was implemented and how]

### Files Created ([X] new files)
1. [file path] ([X] lines) - [Purpose]
2. [file path] ([X] lines) - [Purpose]

### Files Modified ([X] files)
1. [file path] - [Changes made]
2. [file path] - [Changes made]

### Dependencies Added ([X] crates)
1. [crate] = "[version]" - [Justification]
2. [crate] = "[version]" - [Justification]

### Testing Coverage
- Unit tests: [X] tests
- Integration tests: [X] tests
- Total new tests: +[X] (previous [Y] → now [Z])
- Test results: [Z]/[Z] passing ✅

### Performance Characteristics
- [Metric 1]: [Measurement]
- [Metric 2]: [Measurement]
- [Impact on system]: [Description]

### Documentation Updates
- README.md: ✅ Phase 2 features updated
- CHANGELOG.md: ✅ Comprehensive entry added
- Inline docs: ✅ All public APIs documented
- Feature guide: ✅ [Created/Not needed]

### Metrics Integration
- [metric_name_1]: [Description]
- [metric_name_2]: [Description]
- Total metrics: +[X]

### Memory Entities Created
- Phase2_Feature[N]_[FeatureName]_Implementation ([X] observations)
- Phase2_Feature[N]_TechnicalDecisions ([X] observations)
- Phase2_Feature[N]_EdgeCases ([X] observations)

### Validation Results
- cargo check: ✅ PASSES
- cargo build: ✅ SUCCESS
- cargo test: ✅ [X]/[X] passing
- cargo clippy: ✅ [<5] warnings
- cargo build --release: ✅ SUCCESS

### Phase 2 Progress
- Overall: [X]% complete ([Y]/6 features)
- Completed: [List completed features]
- Next: Feature [N+1] - [Name]

### Next Steps
1. Review implementation if needed
2. Commit with /phase-commit command
3. Continue with Feature [N+1] using /next-phase-feature (if Features 1-5)
4. Run /phase-report to document Phase 2 completion (if Feature 6)
```

---

## ✅ SUCCESS CRITERIA

Feature is considered complete when ALL of the following are true:

- ✅ Feature 100% implemented (no stubs, no TODOs)
- ✅ All identified edge cases handled
- ✅ 6+ comprehensive tests written and passing
- ✅ All integration tests passing
- ✅ cargo test shows all tests passing
- ✅ cargo clippy has <5 warnings
- ✅ cargo build --release successful
- ✅ README.md updated
- ✅ CHANGELOG.md updated
- ✅ Inline rustdoc complete
- ✅ 3+ Memory entities created with comprehensive observations
- ✅ CLAUDE.local.md updated with session summary
- ✅ Feature completion report provided

---

## 🚨 IMPORTANT REMINDERS

1. **NO STUBS**: Every function must be fully implemented
2. **NO SHORTCUTS**: Follow all phases systematically
3. **TEST FIRST**: Write tests alongside implementation, not after
4. **DOCUMENT CONTINUOUSLY**: Update docs during implementation, not after
5. **USE MCP SERVERS**: Sequential Thinking for design, Context7 for research, Memory for tracking
6. **QUALITY GATES**: All tests must pass before proceeding
7. **COMPREHENSIVE**: This is production code, not a prototype

---

## 🎯 ESTIMATED TIME BY FEATURE

- Feature 4 (Request Batching): 8-10 hours
- Feature 5 (TUI Interface): 12-16 hours
- Feature 6 (Performance Benchmarking): 6-8 hours

---

**Execute this command now to implement the next Phase 2 feature with full workflow automation.**
