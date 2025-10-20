# /memory-update

Create or update entities in the MCP Memory knowledge graph for Only1MCP.

---

## 🎯 PURPOSE

Preserve architectural decisions, technical knowledge, and implementation details in the MCP Memory knowledge graph for future session reference.

**Time Estimate**: ~5-10 minutes
**Impact**: Long-term knowledge preservation across sessions

---

## 📋 STEP 1: DETERMINE ENTITY TYPE

Ask user: "What type of knowledge are you capturing?"

**Entity Types Available**:

1. **Architecture** - System design decisions and patterns
   - Example: "Only1MCP_CustomCommands_Architecture"
   - Use for: Component designs, integration patterns, architectural choices

2. **TechnicalDebt** - Known issues, limitations, or improvements needed
   - Example: "Only1MCP_TestCoverage_TechnicalDebt"
   - Use for: TODO items, known bugs, optimization opportunities

3. **KnowledgeBase** - Lessons learned, best practices, patterns
   - Example: "Only1MCP_DevWorkflow_BestPractices"
   - Use for: Development patterns, troubleshooting guides, best practices

4. **Implementation** - Specific implementation details and mechanics
   - Example: "Only1MCP_StreamableHTTP_Implementation"
   - Use for: How features work, key algorithms, data structures

5. **Testing** - Testing strategies, patterns, and coverage
   - Example: "Only1MCP_IntegrationTests_Strategy"
   - Use for: Test approaches, coverage goals, testing patterns

---

## 📋 STEP 2: GATHER ENTITY INFORMATION

### Ask User:

1. **Entity Name** (required):
   - "What should this entity be named?"
   - **Format**: `Only1MCP_[Topic]_[Type]`
   - **Examples**:
     - `Only1MCP_ProxyArchitecture_Architecture`
     - `Only1MCP_CachingStrategy_Implementation`
     - `Only1MCP_ErrorHandling_KnowledgeBase`

2. **Observations** (required, 3-8 items):
   - "List 3-8 key observations about this topic (one per line):"
   - **Format**: Clear, detailed technical statements
   - **Examples**:
     - "MCP proxy server uses Axum web framework on Tokio async runtime"
     - "Three transport protocols supported: STDIO, SSE, Streamable HTTP"
     - "Automatic session initialization required for Streamable HTTP transport"

3. **Related Entities** (optional):
   - "Should this entity relate to any existing entities? (yes/no)"
   - If yes: "Enter related entity names (comma-separated):"

---

## 📋 STEP 3: VALIDATE ENTITY NAME

```bash
# Check entity name format
if [[ ! "$ENTITY_NAME" =~ ^Only1MCP_[A-Za-z0-9]+_[A-Za-z]+$ ]]; then
    echo "⚠️  Entity name should follow format: Only1MCP_[Topic]_[Type]"
    echo "   Example: Only1MCP_ProxyServer_Architecture"
    # Suggest correction
fi

# Check for duplicates (search existing)
echo "Checking for existing entities with similar names..."
```

---

## 📋 STEP 4: CREATE ENTITY

Use MCP Memory `create_entities` tool:

```javascript
mcp__MCP_DOCKER__create_entities({
  entities: [
    {
      name: "[ENTITY_NAME]",
      entityType: "[TYPE]",
      observations: [
        "[Observation 1]",
        "[Observation 2]",
        "[Observation 3]",
        "[Observation 4]",
        "[Observation 5]",
        // ... up to 8 observations
      ]
    }
  ]
})
```

**Parameters**:
- `name`: Full entity name (Only1MCP_[Topic]_[Type])
- `entityType`: One of: Architecture, TechnicalDebt, KnowledgeBase, Implementation, Testing
- `observations`: Array of 3-8 detailed technical statements

---

## 📋 STEP 5: CREATE RELATIONS (Optional)

If user provided related entities, create relations:

### Ask for Relation Types:

For each related entity, ask:
- "What is the relationship from [NEW_ENTITY] to [RELATED_ENTITY]?"
- **Common Relation Types**:
  - `implements` - New entity implements related entity's design
  - `depends_on` - New entity requires related entity
  - `enhances` - New entity improves related entity
  - `uses` - New entity uses related entity
  - `replaces` - New entity supersedes related entity
  - `documents` - New entity documents related entity
  - `tests` - New entity tests related entity

### Create Relations:

```javascript
mcp__MCP_DOCKER__create_relations({
  relations: [
    {
      from: "[NEW_ENTITY_NAME]",
      to: "[RELATED_ENTITY_NAME]",
      relationType: "[RELATION_TYPE]"
    }
    // Additional relations if needed
  ]
})
```

---

## 📋 STEP 6: VERIFY CREATION

Search for the newly created entity to confirm:

```javascript
mcp__MCP_DOCKER__search_nodes({
  query: "[ENTITY_NAME or partial match]"
})
```

**Expected Result**:
- Entity appears in search results
- All observations are present
- Relations are established (if created)

---

## 📋 STEP 7: CONFIRMATION REPORT

```bash
echo "================================================================================
  MEMORY BANK UPDATED
================================================================================

Entity Created:
- Name: [ENTITY_NAME]
- Type: [TYPE]
- Observations: [X] recorded

Relations Created: [N]
- [ENTITY] --[relation_type]--> [OTHER_ENTITY]
- [ENTITY] --[relation_type]--> [OTHER_ENTITY]

✅ Knowledge captured in memory graph
✅ Available for future sessions via search
✅ Can be referenced in architectural decisions

To retrieve later:
  Use mcp__MCP_DOCKER__search_nodes with query: '[keyword]'
  Or use mcp__MCP_DOCKER__open_nodes with names: ['[ENTITY_NAME]']

================================================================================"
```

---

## 🎯 ENTITY NAME PATTERNS

### Architecture Entities
**Format**: `Only1MCP_[Component]_Architecture`

**Examples**:
- `Only1MCP_ProxyServer_Architecture`
- `Only1MCP_TransportLayer_Architecture`
- `Only1MCP_RoutingEngine_Architecture`
- `Only1MCP_AdminAPI_Architecture`

### Technical Debt Entities
**Format**: `Only1MCP_[Issue]_TechnicalDebt`

**Examples**:
- `Only1MCP_TestCoverage_TechnicalDebt`
- `Only1MCP_ErrorHandling_TechnicalDebt`
- `Only1MCP_Performance_TechnicalDebt`
- `Only1MCP_Documentation_TechnicalDebt`

### Knowledge Base Entities
**Format**: `Only1MCP_[Topic]_KnowledgeBase`

**Examples**:
- `Only1MCP_DevWorkflow_KnowledgeBase`
- `Only1MCP_TestingPatterns_KnowledgeBase`
- `Only1MCP_MCPProtocol_KnowledgeBase`
- `Only1MCP_ErrorDebugging_KnowledgeBase`

### Implementation Entities
**Format**: `Only1MCP_[Feature]_Implementation`

**Examples**:
- `Only1MCP_StreamableHTTP_Implementation`
- `Only1MCP_SessionManagement_Implementation`
- `Only1MCP_HealthChecking_Implementation`
- `Only1MCP_RequestBatching_Implementation`

### Testing Entities
**Format**: `Only1MCP_[Feature]_TestStrategy`

**Examples**:
- `Only1MCP_Integration_TestStrategy`
- `Only1MCP_UnitTests_TestStrategy`
- `Only1MCP_E2E_TestStrategy`
- `Only1MCP_Coverage_TestStrategy`

---

## 📝 OBSERVATION GUIDELINES

**Good Observations** (Detailed, Technical, Specific):
- ✅ "Axum web framework selected for HTTP server due to performance and tokio integration"
- ✅ "Streamable HTTP transport requires initialize request before other methods per MCP 2025-03-26 spec"
- ✅ "Session IDs stored in Arc<RwLock<Option<String>>> for thread-safe access across requests"
- ✅ "Response caching uses moka 0.12 with TinyLFU eviction policy to prevent cache pollution"

**Poor Observations** (Vague, Generic, Unhelpful):
- ❌ "Uses HTTP"
- ❌ "Has caching"
- ❌ "Works well"
- ❌ "Good performance"

**Observation Characteristics**:
- **Specific**: Names technologies, versions, patterns
- **Technical**: Includes implementation details
- **Contextual**: Explains why, not just what
- **Actionable**: Useful for future development
- **Quantitative**: Includes numbers, metrics when relevant

---

## 🔗 RELATED COMMANDS

- `/session-summary` - Update CLAUDE.local.md (session-specific memory)
- `/daily-log` - Record daily progress (local logs)
- `/phase-commit` - Include memory entities in commit message

**Memory Hierarchy**:
1. **MCP Memory** (permanent, searchable) → `/memory-update`
2. **CLAUDE.local.md** (session continuity) → `/session-summary`
3. **Daily Logs** (detailed history) → `/daily-log`
4. **Git Commits** (code history) → `/phase-commit`

---

## 💡 WHEN TO USE

**Create New Entities**:
- After major architectural decisions
- When implementing new features
- After discovering important patterns
- When solving complex technical problems
- After learning valuable lessons

**Update Existing Entities**:
- When adding observations to existing topics
- When refining previous decisions
- When discovering new edge cases

**Create Relations**:
- When connecting related concepts
- When one component depends on another
- When documenting system integration points

---

## 🎯 COMMON USE CASES

### Use Case 1: After Implementing New Feature

**Situation**: Just completed Phase 2 Feature (e.g., Request Batching)

**Entities to Create**:
1. `Only1MCP_RequestBatching_Implementation` (Implementation)
   - How batching works
   - Data structures used
   - Performance characteristics

2. `Only1MCP_RequestBatching_Architecture` (Architecture)
   - Design decisions
   - Integration points
   - Scalability considerations

3. `Only1MCP_BatchingOptimization_KnowledgeBase` (KnowledgeBase)
   - Lessons learned
   - Best practices
   - Edge cases discovered

**Relations**:
- `RequestBatching_Implementation` --implements--> `RequestBatching_Architecture`
- `BatchingOptimization_KnowledgeBase` --documents--> `RequestBatching_Implementation`

### Use Case 2: After Debugging Complex Issue

**Situation**: Fixed tricky bug with Streamable HTTP auto-initialization

**Entity to Create**:
`Only1MCP_StreamableHTTP_AutoInit_KnowledgeBase` (KnowledgeBase)
- Root cause of issue
- Solution implemented
- How to prevent in future
- Related MCP protocol requirements

**Relations**:
- `StreamableHTTP_AutoInit_KnowledgeBase` --enhances--> `StreamableHTTP_Implementation`

### Use Case 3: Recording Technical Debt

**Situation**: Identified area needing improvement (e.g., test coverage)

**Entity to Create**:
`Only1MCP_IntegrationTestCoverage_TechnicalDebt` (TechnicalDebt)
- Current coverage gaps
- Missing test scenarios
- Priority and impact
- Proposed solution approach

**Relations**:
- `IntegrationTestCoverage_TechnicalDebt` --documents--> `Integration_TestStrategy`

---

## ⚠️ IMPORTANT NOTES

**Entity Naming**:
- Always prefix with `Only1MCP_`
- Use PascalCase for topic/feature names
- End with entity type (Architecture, KnowledgeBase, etc.)
- Keep names concise but descriptive

**Observation Quality**:
- 5-8 observations optimal (not too few, not too many)
- Include technical specifics (crate versions, algorithms, metrics)
- Explain rationale, not just facts
- Future-proof (will make sense months later)

**Searchability**:
- Use consistent terminology
- Include keywords for future search
- Cross-reference related concepts
- Think about how you'll search for this later

---

**Execute this command to preserve architectural decisions and technical knowledge in the memory graph.**
