# /daily-log

Create or update daily development log for Only1MCP.

---

## 🎯 PURPOSE

Generate structured daily log entry documenting development progress, decisions, and metrics.

**Time Estimate**: ~10-15 minutes
**Output**: `dev-logs/YYYY-MM-DD.md`

---

## 📋 STEP 1: DETERMINE DATE AND CHECK EXISTING LOG

```bash
# Get current date
DATE=$(date +%Y-%m-%d)
echo "Creating daily log for: $DATE"

# Check if log exists
if [ -f "dev-logs/$DATE.md" ]; then
    echo "ℹ️  Log file exists, will append new session entry"
    SESSION_NUM=$(($(grep -c "^## Session" "dev-logs/$DATE.md") + 1))
else
    echo "Creating new log file"
    SESSION_NUM=1
fi
```

---

## 📋 STEP 2: GATHER PROJECT INFORMATION

```bash
# Get version from Cargo.toml
VERSION=$(grep "^version" Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')

# Get phase from CLAUDE.local.md
PHASE=$(grep "^**Phase:" CLAUDE.local.md | head -1 | sed 's/.*Phase \([0-9]\).*/\1/' || echo "Unknown")

# Get test results
TEST_RESULTS=$(grep "cargo test:" CLAUDE.local.md | head -1 || echo "Unknown")

# Get time range
START_TIME=$(date +"%H:%M")
```

**Note**: Prompt user for:
1. Session summary (brief overview of work done)
2. Key tasks completed (3-5 items)
3. Architectural decisions made (if any)
4. Blockers encountered (if any)
5. Next session plans

---

## 📋 STEP 3: CREATE OR APPEND LOG ENTRY

### If New File (SESSION_NUM == 1):

Create `dev-logs/YYYY-MM-DD.md` with:

```markdown
# Development Log - YYYY-MM-DD

**Project**: Only1MCP
**Phase**: Phase [N]
**Version**: vX.X.X

---

## Session 1 - [START_TIME] - [END_TIME]

### Summary
[User-provided summary of session goals and accomplishments]

### Tasks Completed
- [Task 1]
- [Task 2]
- [Task 3]

### Architectural Decisions
[If applicable]
- **Decision**: [What was decided]
  - **Rationale**: [Why this approach]
  - **Impact**: [How this affects the system]

### Code Changes
**Files Modified**: [X] files
**Key Files**:
- `path/to/file1` - [Description]
- `path/to/file2` - [Description]

### Testing
- **Tests Run**: [cargo test results]
- **New Tests**: [Number added]
- **Status**: [Pass/Fail summary]

### Blockers/Issues
[If applicable]
- [Issue description] - [Status]

### Next Session
- [ ] [Planned task 1]
- [ ] [Planned task 2]

### Metrics
- **Tests**: XXX/XXX passing (100%)
- **Build**: ✅ SUCCESS
- **Version**: vX.X.X
- **Commit**: [latest commit hash]

---
```

### If Appending (SESSION_NUM > 1):

Append to existing file:

```markdown
## Session N - [START_TIME] - [END_TIME]

### Summary
[User-provided summary]

### Tasks Completed
- [Task 1]
- [Task 2]

### Code Changes
**Files Modified**: [X] files

### Testing
- **Status**: [Pass/Fail summary]

### Next Session
- [ ] [Planned task 1]

### Metrics
- **Tests**: XXX/XXX passing
- **Build**: ✅/❌
- **Version**: vX.X.X

---
```

---

## 📋 STEP 4: COLLECT USER INPUT

**Ask user for each section**:

1. **Session Summary** (required):
   - "Briefly describe what you accomplished this session (1-2 sentences):"

2. **Tasks Completed** (required):
   - "List 3-5 tasks you completed (one per line):"

3. **Architectural Decisions** (optional):
   - "Were any architectural decisions made? (yes/no)"
   - If yes: "Describe the decision, rationale, and impact:"

4. **Code Changes** (auto-populate from git):
   ```bash
   # Get modified files since last commit
   git diff --name-only HEAD
   git diff --cached --name-only
   ```
   - Prompt user to describe key file changes

5. **Testing Status** (auto-populate):
   ```bash
   # Get latest test results
   grep "cargo test:" CLAUDE.local.md | head -1
   ```

6. **Blockers/Issues** (optional):
   - "Any blockers or issues encountered? (yes/no)"
   - If yes: "Describe the issue and current status:"

7. **Next Session Plans** (required):
   - "What do you plan to work on next? (2-3 items):"

8. **End Time**:
   - Auto-populate with current time: `date +"%H:%M"`

---

## 📋 STEP 5: POPULATE TEMPLATE

Replace placeholders with collected information:
- `YYYY-MM-DD`: Today's date
- `[N]`: Current phase number
- `vX.X.X`: Current version
- `[START_TIME]`: When session started (or estimate)
- `[END_TIME]`: Current time
- `[User-provided ...]`: User's input from Step 4
- `XXX/XXX`: Actual test counts
- `[latest commit hash]`: `git log -1 --format=%h`

---

## 📋 STEP 6: WRITE FILE

```bash
# If new file
cat > "dev-logs/$DATE.md" <<'EOF'
[Generated content]
EOF

# If appending
cat >> "dev-logs/$DATE.md" <<'EOF'
[Session entry]
EOF
```

---

## 📋 STEP 7: CONFIRMATION

```bash
echo ""
echo "✅ Daily log updated: dev-logs/$DATE.md"
echo "   Session $SESSION_NUM added"
echo "   Summary: [First 50 chars of summary]..."
echo ""
echo "To view: cat dev-logs/$DATE.md"
```

---

## 🎯 USAGE PATTERNS

**Start of Day**: `/daily-log` to create today's log
**End of Session**: `/daily-log` to add session entry
**Before Commit**: Update log with completed work
**Weekly Review**: Read logs from past week

---

## 📝 TEMPLATE VARIABLES

Auto-populated from:
- **Version**: `Cargo.toml` (version field)
- **Phase**: `CLAUDE.local.md` (current phase)
- **Tests**: `CLAUDE.local.md` (build status section)
- **Commit**: `git log -1 --format=%h`
- **Date/Time**: System date command
- **Files Modified**: `git diff --name-only`

User-provided:
- Session summary
- Tasks completed
- Architectural decisions (optional)
- Blockers/issues (optional)
- Next session plans

---

## 🔗 RELATED COMMANDS

- `/session-summary` - Update CLAUDE.local.md with session results
- `/phase-commit` - Create comprehensive commit after work
- `/memory-update` - Capture decisions in knowledge graph

**Recommended Workflow**:
1. Start session: `/daily-log` (create entry)
2. End session: `/daily-log` (update with results)
3. Update memory bank: `/session-summary`
4. Commit work: `/phase-commit`

---

## 💡 TIPS

**Daily Logs Are Private**:
- Stored in `dev-logs/` (gitignored)
- Safe to include personal notes
- Only README is tracked in git

**Multiple Sessions**:
- Same day → Append new session entry
- Different day → Create new file
- Each session numbered (Session 1, 2, 3...)

**Historical Reference**:
- Review past decisions
- Track progress over time
- Understand development velocity
- Reference blockers and solutions

---

**Execute this command to capture today's development session.**
