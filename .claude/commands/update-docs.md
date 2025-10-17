# /update-docs

Synchronize all Only1MCP documentation files with actual project state.

---

## 🎯 PURPOSE

Ensure README.md, CHANGELOG.md, CLAUDE.local.md, and PHASE_2_MASTER_PLAN.md are accurate and synchronized.

---

## 📋 STEP 1: GATHER ACTUAL PROJECT STATE

```bash
# Get actual test count
cargo test --quiet 2>&1 | grep -E "test result:|running" | tail -5

# Get actual file/line counts
tokei src/

# Get Phase 2 completed features
git log --oneline --grep="Phase 2 Feature" | head -10

# Get current date
date "+%A, %B %d, %Y - %I:%M %p %Z"
```

---

## 📋 STEP 2: VERIFY README.md

### Check Badges
- [ ] Test count badge matches actual: `![Tests](https://img.shields.io/badge/tests-XX%2FYY%20passing-green)`
- [ ] Phase 2 progress badge matches: `![Phase 2](https://img.shields.io/badge/Phase%202-XX%25%20complete-blue)`
- [ ] Build status accurate

### Check Phase 2 Features Section
```markdown
#### Phase 2 Features

- ✅ **Configuration Hot-Reload** - Zero-downtime config updates
- ✅ **Active Health Checking** - Proactive backend monitoring
- ✅ **Response Caching (TTL/LRU)** - Automatic cache management
- [✅ or ⬜] **[Feature 4-6]** - [Description]
```

### Check Project Status Section
- [ ] All completed features marked ✅
- [ ] Pending features marked ⬜
- [ ] Test counts accurate

### Check Dependencies in Credits
- [ ] All Phase 2 crates listed (moka, notify, which, arc-swap, etc.)

### Check Code Examples
- [ ] Test count expectations match actual

### Update if Needed
If discrepancies found, update README.md immediately.

---

## 📋 STEP 3: VERIFY CHANGELOG.md

### Check [0.2.0-dev] Section
- [ ] All completed Phase 2 features documented
- [ ] Each feature has comprehensive entry with sub-features
- [ ] Version links at bottom work

### Check Entry Format
Each feature should have:
```markdown
### Added
- **[Feature Name]**: [Description]
  - [Sub-feature 1]
  - [Sub-feature 2]
  - [Performance characteristics]
  - [Configuration options]
```

### Update if Needed
Add missing feature entries with full details.

---

## 📋 STEP 4: VERIFY CLAUDE.local.md

### Check Header
- [ ] Last Updated timestamp current
- [ ] Project Status line accurate
- [ ] Build Status section has latest test results (XX/YY passing)

### Check Phase Progress Section
- [ ] Phase 2 percentage accurate (N/6 features)
- [ ] All completed features marked ✅ with dates and commit hashes
- [ ] Pending features marked ⬜

### Check Current Session Summary
- [ ] Latest session documented
- [ ] Major achievements listed
- [ ] Test count progression accurate

### Check Build Status
- [ ] cargo check status current
- [ ] cargo test results current (XX/YY passing)
- [ ] cargo clippy status current

### Update if Needed
Add latest session summary, update test counts, mark completed features.

---

## 📋 STEP 5: VERIFY PHASE_2_MASTER_PLAN.md

### Check Feature Status Table
```markdown
| Feature | Status | Tests | Docs | Sub-Agent |
|---------|--------|-------|------|-----------|
| 1. Config Hot-Reload | ✅ Complete | ✅ | ✅ | Complete |
| 2. Active Health | ✅ Complete | ✅ | ✅ | Complete |
| 3. Response Caching | ✅ Complete | ✅ | ✅ | Complete |
| 4. Request Batching | [Status] | [Status] | [Status] | [Status] |
| 5. TUI Interface | ⏸️ Pending | ⏸️ | ⏸️ | Not started |
| 6. Benchmarking | ⏸️ Pending | ⏸️ | ⏸️ | Not started |
```

### Check Last Updated Timestamp
- [ ] "Last Updated:" at bottom is current date

### Check Weekly Goals
- [ ] Completed features marked with ✅
- [ ] Test count progressions accurate

### Update if Needed
Mark completed features, update checkboxes, update timestamp.

---

## 📋 STEP 6: CROSS-REFERENCE VALIDATION

### Verify Consistency Across Files
- [ ] Test counts match across all files
- [ ] Phase 2 progress percentage consistent
- [ ] Feature completion dates match git log
- [ ] Commit hashes referenced are correct
- [ ] All links work (especially in README to docs/)

### Check for Orphaned References
- [ ] No references to non-existent files
- [ ] No broken cross-references between docs
- [ ] All example file paths valid

---

## 📋 STEP 7: GENERATE VERIFICATION REPORT

Provide comprehensive report:

```markdown
## Documentation Synchronization Report

**Verification Date**: [Date and Time]
**Project State**: Phase 2 [X]% complete ([N]/6 features)
**Test Status**: [XX]/[YY] passing ([%]%)

### README.md Status
- Test count badge: [✅ Accurate / ❌ Needs update: old → new]
- Phase 2 progress badge: [✅ Accurate / ❌ Needs update: old% → new%]
- Features list: [✅ Accurate / ❌ Missing: Feature X]
- Dependencies: [✅ Complete / ❌ Missing: crate-name]
- Code examples: [✅ Accurate / ❌ Outdated test counts]
- **Updates Made**: [List changes or "None - already accurate"]

### CHANGELOG.md Status
- [0.2.0-dev] section: [✅ Complete / ❌ Missing entries]
- Feature 1 entry: [✅ Present]
- Feature 2 entry: [✅ Present]
- Feature 3 entry: [✅ Present]
- Feature [N] entry: [✅ Present / ❌ Missing]
- **Updates Made**: [List changes or "None - already accurate"]

### CLAUDE.local.md Status
- Last Updated: [✅ Current / ❌ Updated to: new timestamp]
- Build Status: [✅ Accurate / ❌ Updated: old → new]
- Phase Progress: [✅ Accurate / ❌ Updated: old% → new%]
- Session Summary: [✅ Complete / ❌ Added Session N]
- **Updates Made**: [List changes or "None - already accurate"]

### PHASE_2_MASTER_PLAN.md Status
- Feature checkboxes: [✅ Accurate / ❌ Updated: Feature N marked complete]
- Last Updated timestamp: [✅ Current / ❌ Updated]
- Weekly goals: [✅ Accurate / ❌ Updated]
- **Updates Made**: [List changes or "None - already accurate"]

### Cross-Reference Validation
- Test count consistency: [✅ All match / ❌ Discrepancies found and fixed]
- Phase 2 progress consistency: [✅ All match / ❌ Updated to: X%]
- Commit hash references: [✅ All valid / ❌ Fixed invalid references]
- File path references: [✅ All valid / ❌ Fixed broken links]

### Overall Status
✅ **All documentation synchronized and accurate**

OR

⚠️ **Updates applied to [N] files, [M] discrepancies corrected**

### Summary of Changes
[If updates made, list all changes in detail]

OR

No changes needed - all documentation was already accurate and synchronized.
```

---

## ✅ VERIFICATION CHECKLIST

Documentation is synchronized when:

- ✅ Test counts match across all files and actual cargo test output
- ✅ Phase 2 progress percentage consistent everywhere
- ✅ All completed features marked ✅ in all files
- ✅ All pending features marked ⬜ consistently
- ✅ Timestamps current (Last Updated dates)
- ✅ Commit hashes referenced are valid
- ✅ All dependencies credited in README
- ✅ No broken cross-references or links
- ✅ Code examples reflect current state
- ✅ Session summaries up-to-date in CLAUDE.local.md

---

**Execute this command regularly to prevent documentation drift.**
