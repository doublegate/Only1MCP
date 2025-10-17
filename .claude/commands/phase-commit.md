# /phase-commit

Generate comprehensive conventional commit message for Phase 2 feature completion.

---

## 🎯 PURPOSE

Create detailed, structured commit messages following Only1MCP's established pattern for Phase 2 feature commits.

---

## 📋 EXECUTION

### Step 1: Analyze Changes

```bash
# Check git status
git status

# Review changes
git diff --stat
git diff --cached --stat

# View recent commits for pattern
git log --oneline -3
```

### Step 2: Read Context

```bash
# Understand feature details
cat CLAUDE.local.md | grep -A 20 "Session.*COMPLETE"

# Identify feature number
cat to-dos/Phase_2/PHASE_2_MASTER_PLAN.md | grep -E "Feature [0-9].*✅"
```

### Step 3: Generate Commit Message

Use this template, filling in actual details:

```
feat: Implement Phase 2 Feature [N] - [Feature Name]

🚀 [One-line summary] - Production Ready

This commit implements [feature description], completing Feature [N] of Phase 2
([X]% of Phase 2 complete, [N]/6 features).

## Feature Overview
[2-3 sentence description of what the feature does and why it's valuable]

## Core Implementation

### [Component 1 Name]
- [Detail 1]: [Technical description]
- [Detail 2]: [Technical description]
- [Detail 3]: [Technical description]

### [Component 2 Name]
- [Detail 1]: [Technical description]
- [Detail 2]: [Technical description]

### [Component 3 Name] (if applicable)
- [Detail 1]: [Technical description]

## Files Created ([X] new files)
1. `[file path]` ([X] lines) - [Purpose and contents]
2. `[file path]` ([X] lines) - [Purpose and contents]

## Files Modified ([X] existing files)
1. `[file path]`: [Specific changes made]
2. `[file path]`: [Specific changes made]
3. `Cargo.toml`: Added [N] dependencies for [feature]
4. `README.md`: Updated Phase 2 features and test count
5. `CHANGELOG.md`: Added [feature] entry to [0.2.0-dev]

## Implementation Details

### Architecture & Patterns
[Describe architectural approach, design patterns used, why chosen]

### Key Algorithms & Data Structures
[Describe core algorithms, data structures, complexity characteristics]

### Performance Characteristics
- [Metric 1]: [Measurement or target]
- [Metric 2]: [Measurement or target]
- [Impact]: [How this affects overall system performance]

### Error Handling
[Describe error handling strategy, recovery mechanisms, edge cases covered]

### Concurrency & Safety
[Describe thread safety, async patterns, lock-free structures if applicable]

### Integration Points
[How this integrates with existing components: ProxyServer, Config, Metrics, etc.]

## Testing Coverage

### Unit Tests ([X] tests)
- `test_[name]`: [What it validates]
- `test_[name]`: [What it validates]
- [List all unit tests with descriptions]

### Integration Tests ([X] tests)
- `test_[name]`: [What it validates]
- `test_[name]`: [What it validates]
- [List all integration tests with descriptions]

### Edge Case Tests
- [Edge case 1]: [How handled and tested]
- [Edge case 2]: [How handled and tested]

**Total New Tests**: +[X] (previous [Y] → now [Z])
**Test Results**: [Z]/[Z] passing ✅ 100%

## Performance Characteristics

### Latency
- [Operation]: [Measurement]
- Overhead: [Measurement]

### Throughput
- [Metric]: [Measurement]

### Memory
- Per-[unit]: [Measurement]
- Total footprint: [Measurement]

### Scalability
- [Characteristic]: [How it scales]

## Documentation Updates

### README.md
- Added [Feature Name] to Phase 2 Features section
- Updated test count badges: [old] → [new]
- Updated Phase 2 progress: [old]% → [new]%
- Added feature description and usage example

### CHANGELOG.md
- Comprehensive [Feature Name] entry in [0.2.0-dev]
- Detailed sub-features and capabilities
- Configuration options documented
- Performance characteristics noted

### Inline Documentation
- All public APIs have rustdoc comments
- Module-level documentation complete
- Examples provided for complex APIs
- Error conditions documented

### Feature-Specific Docs (if created)
- Created `docs/[feature_name].md` guide
- Configuration examples
- Usage patterns
- Troubleshooting section

## Dependencies Added/Updated

1. `[crate-name] = "[version]"` - [Justification: what it provides]
2. `[crate-name] = "[version]"` - [Justification: what it provides]

**Total New Dependencies**: [X]

## Metrics Integration

### New Prometheus Metrics
- `[metric_name]`: [Description of what it tracks]
- `[metric_name]`: [Description of what it tracks]

**Total New Metrics**: [X]

## Configuration Schema

### New Configuration Options
```yaml
[section]:
  [option]: [value]  # [Description]
  [option]: [value]  # [Description]
```

## Build & Validation Results

- **cargo check**: ✅ PASSES (0 errors, 0 warnings)
- **cargo build**: ✅ SUCCESS (0 errors, [X] non-critical warnings)
- **cargo test**: ✅ [Z]/[Z] passing (100%)
  - Unit tests (lib): [X]/[X] passing
  - Integration tests: [Y]/[Y] passing
- **cargo clippy**: ✅ CLEAN ([<5] warnings, all non-critical)
- **cargo build --release**: ✅ SUCCESS (optimized binary generated)
- **cargo fmt --check**: ✅ PASSES

## Memory Entities Created

1. **Phase2_Feature[N]_[FeatureName]_Implementation**
   - Entity Type: FeatureImplementation
   - Observations: [X] observations tracking implementation details

2. **Phase2_Feature[N]_TechnicalDecisions**
   - Entity Type: ArchitecturalDecision
   - Observations: [X] observations documenting design choices

3. **Phase2_Feature[N]_EdgeCases** (if applicable)
   - Entity Type: KnowledgeBase
   - Observations: [X] observations capturing edge cases and limitations

**Total Memory Observations**: [X]+ added to knowledge graph

## Phase 2 Progress

**Overall**: [X]% complete ([N]/6 features implemented)

### Completed Features
- ✅ Feature 1: Configuration Hot-Reload (d8e499b)
- ✅ Feature 2: Active Health Checking (64cd843)
- ✅ Feature 3: Response Caching TTL/LRU (6391c78)
- ✅ Feature [N]: [Feature Name] (this commit)

### Remaining Features
- ⬜ Feature [N+1]: [Name] (estimated [X]-[Y] hours)
- ⬜ Feature [N+2]: [Name] (estimated [X]-[Y] hours)

### Milestone Progress
- Sprint 1: [Status or percentage]
- Sprint 2: [Status or percentage]
- Phase 2 Target: [Status]

## Known Issues

[If any issues or limitations, list them. Otherwise:]
None. All tests passing, feature fully functional.

## Next Steps

1. [Next feature to implement] - Feature [N+1]
2. [Any follow-up work needed]
3. [Documentation or optimization opportunities]

---

🚀 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>
```

### Step 4: Stage Changes

```bash
git add -A
```

### Step 5: Commit with Message

```bash
git commit -m "$(cat <<'EOF'
[Paste generated commit message here]
EOF
)"
```

### Step 6: Verify Commit

```bash
git log -1 --stat
git status
```

---

## ✅ COMMIT MESSAGE CHECKLIST

Ensure your commit message includes ALL of these sections:

- [ ] Conventional commit type (feat:, docs:, fix:, etc.)
- [ ] One-line summary (concise, descriptive)
- [ ] Feature overview (what and why)
- [ ] Core implementation (detailed technical breakdown)
- [ ] Files created (with line counts and purposes)
- [ ] Files modified (with specific changes)
- [ ] Implementation details (architecture, algorithms, performance)
- [ ] Testing coverage (all tests listed with descriptions)
- [ ] Performance characteristics (latency, throughput, memory)
- [ ] Documentation updates (README, CHANGELOG, inline, guides)
- [ ] Dependencies added (with justifications)
- [ ] Metrics integration (Prometheus metrics)
- [ ] Configuration schema (new options)
- [ ] Build validation results (all checks passing)
- [ ] Memory entities created (with observation counts)
- [ ] Phase 2 progress (percentage, completed, remaining)
- [ ] Known issues (or explicit "None")
- [ ] Next steps (follow-up actions)
- [ ] Claude Code attribution

---

## 🎯 COMMIT MESSAGE PATTERNS

### Commit Types
- `feat`: New feature implementation
- `fix`: Bug fix
- `docs`: Documentation updates only
- `test`: Test additions or fixes
- `refactor`: Code restructuring without behavior change
- `perf`: Performance improvements
- `chore`: Maintenance (dependencies, build, etc.)

### Summary Line Guidelines
- Start with type: `feat: Implement ...`
- Use imperative mood: "Implement" not "Implemented"
- Max 72 characters
- Describe WHAT not HOW
- No period at end

### Body Guidelines
- Comprehensive technical details
- Use present tense: "Adds" not "Added"
- Include metrics and measurements
- List all significant files
- Document all architectural decisions
- Provide context for future developers

---

**Execute this command after feature implementation to create comprehensive commit message.**
