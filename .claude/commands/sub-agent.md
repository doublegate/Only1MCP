Generate a new sub-agent tool task (run separately), to: $*

---

## PROJECT CONTEXT (Only1MCP)
**Type:** Rust-based MCP server aggregator/proxy for AI applications
**Version:** v0.2.10 (Phase 2 complete, Phase 3 planning)
**Status:** Phase 1-2 complete (100%), 4 MCP servers operational (14 tools)
**Tests:** 127 passing (100% success rate)
**Architecture:** Modular proxy with transport layer (HTTP, STDIO, SSE, Streamable HTTP)

## QUALITY STANDARDS (Apply to ALL work)
✅ **Code:** cargo fmt + clippy passing, zero warnings, tests passing
✅ **Documentation:** Technical accuracy, consistent formatting, no broken links
✅ **Commits:** Comprehensive messages with context, impact, and file lists
✅ **Memory Banks:** Keep updated with decisions, progress, and metrics
✅ **File Organization:** Follow project conventions (lowercase with underscores)

## SYSTEMATIC APPROACH (Work Through Phases)
1. **ANALYZE** - Read relevant files, understand current state, identify scope
2. **PLAN** - Break down into subtasks, identify dependencies, estimate effort
3. **EXECUTE** - Implement systematically, test as you go, document decisions
4. **VERIFY** - Test thoroughly, check cross-references, validate no regressions
5. **DOCUMENT** - Update README/CHANGELOG/memory banks, create deliverables report
6. **REPORT** - Provide comprehensive summary with metrics, changes, and next steps

## COMMUNICATION GUIDELINES
- **Progress Updates:** Report after each major phase (not just at end)
- **Decisions:** Explain rationale for significant choices
- **Issues:** Report blockers immediately, suggest solutions
- **Questions:** Ask for clarification if requirements unclear
- **Metrics:** Include concrete numbers (files changed, tests added, performance gains)

## DOCUMENTATION REQUIREMENTS
- **Update CHANGELOG.md** if changes affect user/behavior/features
- **Update README.md** if changes affect statistics/features/usage
- **Update CLAUDE.local.md** with session summary, metrics, decisions
- **Create README files** for new directories (if introducing new structure)
- **Log major decisions** in appropriate memory bank

## VERIFICATION CHECKLIST (Before Completion)
✅ All subtasks completed successfully
✅ Tests passing (if code changed)
✅ Documentation updated (if user-facing changes)
✅ Memory banks updated with session info
✅ No TODO/FIXME/WIP markers left uncommitted
✅ Cross-references validated (links, paths, file references)
✅ Git history clean (proper git mv for renames)
✅ No temporary files left in /tmp/

## DELIVERABLES (Always Provide)
1. **Summary Report:** What was accomplished, metrics, key decisions
2. **File Changes Log:** Files created/modified/deleted with line counts
3. **Issues Encountered:** Problems found and how resolved
4. **Verification Results:** Test results, validation checks performed
5. **Next Steps:** Recommendations for follow-up work (if applicable)
6. **Git Status:** Ready to commit? Any conflicts? Staging status

## ERROR HANDLING
- **Compilation Errors:** Report exact error, affected file, suggested fix
- **Test Failures:** Report which tests, failure details, root cause analysis
- **Linting Issues:** Fix automatically if possible, else report with solutions
- **Broken References:** Fix or report with list of broken links
- **Unclear Requirements:** Ask for clarification before proceeding
- **Blockers:** Report immediately with impact assessment and suggested workarounds

## EFFICIENCY GUIDELINES
- **Read Before Write:** Always read files before editing (understand context)
- **Batch Operations:** Use git mv for multiple renames in single session
- **Parallel Work:** When independent tasks exist, document all, then execute
- **Avoid Redundancy:** Check if work already done (grep, read existing docs)
- **Preserve History:** Always use git mv (never delete + create for renames)
- **Smart Skipping:** If only docs changed, skip code quality checks

## MEMORY BANK UPDATES (Critical)
**Add to CLAUDE.local.md after completion:**
- **Date:** Current date (YYYY-MM-DD)
- **Task:** Brief description of work completed
- **Duration:** Approximate time spent
- **Key Decisions:** Important choices made and rationale
- **Metrics:** Files changed, tests added, performance improvements
- **Issues:** Problems encountered and resolutions
- **Status:** Current project state after changes
- **Next Actions:** Recommended follow-up tasks

## PROJECT-SPECIFIC NOTES
- **Transport Layer:** Support HTTP, STDIO, SSE, Streamable HTTP, WebSocket (stub)
- **Configuration:** YAML/TOML files in `config/templates/` (solo, team, enterprise)
- **Custom Commands:** Claude Code commands in `.claude/commands/` (9 total)
- **Development Logs:** Daily logs in `dev-logs/` (gitignored except README)
- **Testing Strategy:** 127 tests across unit, integration, and doc tests
- **Documentation:** README, CHANGELOG, ARCHITECTURE.md, API_REFERENCE.md
- **Phase Tracking:** Use `to-dos/master-tracker.md` for roadmap
- **MCP Servers:** 4 operational (Context7, Sequential Thinking, Memory, NWS Weather)

## SUCCESS CRITERIA (Must Meet ALL)
✅ Task completed as specified (100% of requirements)
✅ No regressions introduced (tests still passing)
✅ Documentation comprehensive and accurate
✅ Memory banks updated with session
✅ All deliverables provided
✅ Clean git state (no conflicts, proper staging)
✅ Ready for user review/commit

## FINAL REMINDERS
⚠️ **ALWAYS read files before editing** - understand context first
⚠️ **NEVER commit without user approval** - stage only, let user commit
⚠️ **ALWAYS preserve git history** - use git mv, never delete+create
⚠️ **ALWAYS update memory banks** - critical for continuity
⚠️ **ALWAYS provide comprehensive reports** - metrics, decisions, next steps
⚠️ **ALWAYS ask if unclear** - better to clarify than assume

---

**Now execute the task above with full context, systematic approach, and comprehensive deliverables.**
