# /session-summary

Update CLAUDE.local.md with current session summary and metrics.

---

## 🎯 PURPOSE

Systematically update the Only1MCP session memory bank (CLAUDE.local.md) with current session results, ensuring continuity between development sessions.

**Time Estimate**: ~5 minutes
**Impact**: Critical for session continuity and context preservation

---

## 📋 STEP 1: READ CURRENT STATE

```bash
# Read CLAUDE.local.md to understand current structure
cat CLAUDE.local.md | head -100

# Get current session number
LAST_SESSION=$(grep "#### Session" CLAUDE.local.md | tail -1 | sed 's/.*Session \([0-9]*\).*/\1/')
NEW_SESSION=$((LAST_SESSION + 1))

# Get current date
DATE=$(date "+%b %d, %Y")
TIME=$(date "+%-I:%M %p %Z")
```

---

## 📋 STEP 2: GATHER SESSION INFORMATION

### Ask User for Session Details

1. **Session Title** (required):
   - "What is the main accomplishment or focus of this session? (e.g., 'ALL 4 MCP SERVERS OPERATIONAL')"

2. **Session Emoji** (optional):
   - "Choose an emoji to represent this session (default: 🚀):"

3. **Major Tasks** (required):
   - "List major tasks completed (1-5 categories with ~X hours each):"
   - Format: "Category Name (~X hours)"

4. **Task Details** (required):
   - For each category: "List 2-4 bullet points of what was done:"

5. **Implementation Highlights** (required):
   - "List 3-5 key achievements or implementation highlights:"

6. **Technical Details** (optional):
   - "Any technical metrics or details to include? (lines changed, performance, etc.):"

7. **Files Modified** (auto-populate from git):
   ```bash
   # Get modified files
   git status --porcelain | wc -l
   git diff --name-only HEAD
   ```
   - Prompt user to describe changes for key files

### Collect Build Status

```bash
# Run quick validation to get current status
echo "Running quick build status checks..."

# Format check
cargo fmt --check &> /dev/null && FMT_STATUS="✅ FORMATTED" || FMT_STATUS="❌ NEEDS FORMAT"

# Build check
cargo check 2>&1 | grep -q "error" && BUILD_STATUS="❌ ERRORS" || BUILD_STATUS="✅ PASSES (0 errors)"

# Test count (from last successful run or CLAUDE.local.md)
TEST_COUNT=$(grep "cargo test:" CLAUDE.local.md | head -1 | grep -oP '\d+/\d+' || echo "XXX/XXX")

# Clippy status (don't run, use last known)
CLIPPY_STATUS=$(grep "cargo clippy:" CLAUDE.local.md | head -1 | sed 's/.*- \*\*cargo clippy:\*\* //')

# Get version
VERSION=$(grep "^version" Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
```

---

## 📋 STEP 3: CREATE SESSION ENTRY

Generate session entry in this format:

```markdown
#### Session [N] ([MMM DD, YYYY]) - **[SESSION TITLE]** [EMOJI]
1. **[Task Category 1]** (~X hours)
   - Task detail 1
   - Task detail 2
   - Result or metric

2. **[Task Category 2]** (~X hours)
   - Task details

**Implementation Highlights**:
- Key achievement 1
- Key achievement 2
- Key achievement 3

**Technical Details**:
- Metric 1
- Metric 2

**Files Modified**: X files
1. `path/file1` - Changes description
2. `path/file2` - Changes description
```

---

## 📋 STEP 4: UPDATE BUILD STATUS SECTION

Locate and update the "### Build Status" section:

```markdown
### Build Status (As of [MMM DD, YYYY HH:MM AM/PM TZ])
- **cargo check:** [✅ PASSES (0 errors) | ❌ ERRORS]
- **cargo build:** [✅ SUCCESS (0 errors, X warnings) | ❌ FAILED]
- **cargo test:** [✅ XXX/XXX PASSING (100%) | ❌ FAILURES]
- **cargo clippy:** [✅ CLEAN | ⚠️ WARNINGS]
- **cargo fmt:** [✅ FORMATTED | ❌ NEEDS FORMAT]
```

---

## 📋 STEP 5: UPDATE LAST UPDATED TIMESTAMP

Update the first line of CLAUDE.local.md:

```markdown
**Last Updated:** [Day], [Month] [DD], [YYYY] - [H]:MM AM/PM TZ
```

Example: `**Last Updated:** Sunday, October 20, 2025 - 12:45 AM EDT`

---

## 📋 STEP 6: UPDATE FILE

Use Edit tool to make precise updates:

### Update 1: Add Session Entry

Insert new session at top of "## 📋 Current Session Summary" section (after the section header, before previous sessions):

```bash
# Find line number of "## 📋 Current Session Summary"
# Insert new session entry after it
```

### Update 2: Replace Build Status

Find "### Build Status (As of " and replace entire section through the last bullet point.

### Update 3: Update Timestamp

Replace first line with new timestamp.

---

## 📋 STEP 7: VERIFY UPDATES

```bash
# Show what changed
echo "Updated sections:"
echo "  ✅ Last Updated timestamp"
echo "  ✅ Session $NEW_SESSION added"
echo "  ✅ Build status updated"
echo ""

# Show new session summary (first 10 lines)
grep -A 10 "#### Session $NEW_SESSION" CLAUDE.local.md
```

---

## 📋 STEP 8: CONFIRMATION

```bash
echo "================================================================================
  SESSION SUMMARY UPDATED
================================================================================

Session: $NEW_SESSION
Date: $DATE $TIME
Title: [SESSION_TITLE]
Tasks: [X] categories completed

✅ CLAUDE.local.md updated successfully
✅ Build status current
✅ Timestamp updated

Next Steps:
1. Review updated CLAUDE.local.md
2. Run /daily-log to record in daily logs
3. Run /phase-commit when ready to commit

================================================================================"
```

---

## 🎯 SESSION ENTRY TEMPLATE

```markdown
#### Session [N] ([Date]) - **[TITLE]** [EMOJI]
1. **[Category]** (~X hours)
   - Detail 1
   - Detail 2
   - Detail 3

2. **[Category]** (~X hours)
   - Detail 1
   - Detail 2

**Implementation Highlights**:
- Achievement 1
- Achievement 2
- Achievement 3

**Technical Details**:
- Lines Changed: +XXX/-YYY
- Performance: [metric]
- Memory: [metric]

**Files Modified**: X files
1. `path/file` - Description
2. `path/file` - Description
```

---

## 📝 IMPORTANT NOTES

**Preserve Existing Content**:
- NEVER delete previous sessions
- Only update Build Status and Last Updated
- Add new session at TOP of session list (chronological order)

**Use Edit Tool**:
- Use Edit, NOT Write
- Precise old_string → new_string replacements
- Verify existing content preserved

**Session Numbering**:
- Auto-increment from last session number
- Sequential: Session 1, Session 2, Session 3...
- Never skip or duplicate numbers

**Timestamp Format**:
- Full format: "Sunday, October 20, 2025 - 12:45 AM EDT"
- Include day of week, month name, time with AM/PM, timezone

---

## 🔗 RELATED COMMANDS

- `/daily-log` - Create daily log entry (more detailed)
- `/phase-commit` - Commit with comprehensive message
- `/memory-update` - Preserve architectural decisions
- `/rust-check` - Get current build status

**Recommended Workflow**:
1. Complete work session
2. Run `/session-summary` (update CLAUDE.local.md)
3. Run `/daily-log` (record in daily logs)
4. Run `/phase-commit` (commit changes)

---

## 💡 USAGE PATTERNS

**End of Session**: Always run to preserve context
**After Major Milestones**: Document significant achievements
**Before Commits**: Ensure memory bank is current
**Multi-Session Days**: Run after each major work block

---

## ⚠️ CRITICAL RULES

1. **ALWAYS** preserve existing session entries
2. **NEVER** delete or overwrite previous content
3. **ALWAYS** use Edit tool, not Write
4. **ALWAYS** update all 3 sections (session entry, build status, timestamp)
5. **ALWAYS** verify updates before confirming

---

**Execute this command at the end of every development session to maintain continuity.**
