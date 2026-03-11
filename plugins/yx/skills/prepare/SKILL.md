---
name: prepare
description: Prepare a yak for implementation by establishing spec, acceptance criteria, and breaking into sub-yaks
---

# Prepare Skill

You're helping prepare a yak for implementation. This is the bridge between mapping (lightweight structure) and working (hands-on implementation).

**Arguments:** `$ARGUMENTS` — the yak name to prepare, or empty to prompt the user.

**Goal:** Transform a yak from an idea into a well-specified piece of work ready for zero-context implementation.

## When to Use This Skill

- User says "prepare this yak", "spec out X", "flesh out this work", "break down this feature"
- A yak needs more detail before implementation can start
- You need to establish acceptance criteria for a yak
- A yak is too big and needs breaking into sub-yaks

**Related skills:**
- `/yx:map` — for lightweight initial structure and exploration
- `/yx:work` — for implementation after preparation is complete

**Workflow reference:** See `../docs/yak-workflow.dot` for the overall yak lifecycle.

## The Preparation Flow

### 1. Read Existing State

**Before doing anything else**, understand what you're working with:

```bash
# Show the yak details
yx show "$ARGUMENTS"

# See the context around it
yx ls

# Check parent and sibling yaks for context
```

If you need to explore the codebase (understand existing code, find relevant files, assess complexity):
- **Delegate to a subagent** — keeps the main conversation clean for spec discussion with the user
- Use explorers or code-reading agents to gather information
- Bring back a summary, not a transcript

**Ask:** Does the user want to answer questions now, or should you gather more context first?

### 2. Check for @needs-human

If the yak has a `@needs-human` tag:

```bash
yx show "$ARGUMENTS" --format json | jq -r '.tags[]'
```

**Surface it:** "This yak has an open question in its context: [question]. Want to answer now, or should I prepare a different yak?"

If answered, remove the tag:
```bash
yx untag "$ARGUMENTS" @needs-human
```

### 3. Mark Work In Progress

Before making any changes or additions:

```bash
yx start "$ARGUMENTS"
```

This marks the yak as WIP and creates a worktree if you'll be editing code.

### 4. Brainstorm the Spec

**Collaborate with the user** to establish:
- **Goal:** What are we trying to achieve? (1-2 sentences)
- **Scope:** What's in, what's out?
- **Key decisions:** Architecture, approach, tradeoffs
- **Why:** What problem does this solve? What value does it create?

**For refactorings:** Capture the "why" — what motivated this change? What will it make easier?

**Store as yak context:**
```bash
yx context "$ARGUMENTS" <<'EOF'
# Spec: [yak name]

## Goal
[What we're achieving]

## Scope
- In scope: ...
- Out of scope: ...

## Key Decisions
- [Decision 1 and rationale]
- [Decision 2 and rationale]

## Why
[The problem this solves, the value it creates]
EOF
```

**Confirm with user:** "Does this spec capture what you want? Anything to add or change?"

### 5. Establish Acceptance Criteria

Help the team define **how they'll know it's done**.

**Encourage teams to use their own approach:**
- Automated tests (integration, unit, e2e)
- Manual test checklist
- Demo criteria
- Performance benchmarks
- Documentation requirements

**Suggest automated tests if appropriate**, but don't mandate a specific technique. The goal is clarity on "done", not prescribing implementation practices.

**Add to context:**
```markdown
## Acceptance Criteria
- [ ] [Criterion 1]
- [ ] [Criterion 2]
- [ ] [Criterion 3]
```

Or:
```markdown
## Definition of Done
- Tests pass for X, Y, Z scenarios
- Documentation updated
- Code review completed
```

**Confirm:** "Do these acceptance criteria capture what 'done' looks like?"

### 6. Break Into Sub-Yaks (If Needed)

**When to break down:**
- The yak feels too big for one piece of work
- Multiple distinct steps or components
- Dependencies between parts
- Different skills or knowledge areas involved

**When NOT to break down:**
- The work is naturally cohesive
- Sub-yaks would be trivial (< 1 hour each)
- Breaking it would create artificial dependencies

**If breaking down:**

#### The Iron Law
**Always** run `yx ls` after every `yx add` to verify the structure.

```bash
# Add sub-yak
yx add "[parent name]" "[sub-yak name]"

# IMMEDIATELY verify
yx ls

# Repeat for each sub-yak
```

#### Sub-Yak Context Pattern

Each sub-yak needs **enough detail for zero-context implementation**:

```markdown
# [Sub-yak name]

## Goal
[One sentence: what this sub-yak achieves]

## Files to Change
- `path/to/file.ts` — [what changes]
- `path/to/test.ts` — [what tests to add]

## Acceptance Criteria
- [ ] [Specific criterion 1]
- [ ] [Specific criterion 2]

## Implementation Notes
- [Key detail 1]
- [Key detail 2]
- [Any gotchas or edge cases]

## Dependencies
- [What needs to be done first, if anything]
```

**Write context for each sub-yak:**
```bash
yx context "[parent name] / [sub-yak name]" <<'EOF'
[Detailed context as above]
EOF
```

**Ordering sub-yaks:**
- Consider dependencies — what needs to be done first?
- Use `yx reorder "[parent]" [child-id] [position]` if needed
- Confirm order with user: "Does this order make sense?"

### 7. Surface Open Questions

If preparation reveals questions only the human can answer:

1. **Tag the yak:**
```bash
yx tag "$ARGUMENTS" @needs-human
```

2. **Add the question to context:**
```bash
yx context "$ARGUMENTS" <<'EOF'
[Existing context]

## Open Questions
- [ ] [Question for the human]
EOF
```

3. **Ask the user:** "I need your input on: [question]. Want to answer now, or should I move on?"

If they answer, update the context and remove the tag.

If they want to defer, that's fine — the tag ensures it won't be forgotten.

### 8. Final Confirmation

Before finishing:

1. **Review with user:**
   - "Here's the spec: [summary]"
   - "Acceptance criteria: [list]"
   - "I've broken this into N sub-yaks: [list names]" (if applicable)
   - "Does this look ready for implementation?"

2. **Mark ready if complete:**
```bash
yx tag "$ARGUMENTS" @ready
```

3. **Or surface what's still needed:**
   - "Still need: [X, Y, Z]"
   - "Blocked on: [dependency]"

## Example Interaction Patterns

### Pattern 1: Simple Yak (No Sub-Yaks Needed)

```
User: Prepare "fix password reset email"

You: 
1. Read the yak and surrounding context
2. Start the yak: `yx start "fix password reset email"`
3. "Let's brainstorm the spec. What's broken with the email currently?"
4. Discuss with user, write spec to context
5. "How will we know it's fixed? Should we add tests?"
6. Add acceptance criteria to context
7. "This feels like a single piece of work. Ready to implement?"
```

### Pattern 2: Complex Yak (Needs Breaking Down)

```
User: Prepare "add OAuth support"

You:
1. Read the yak: `yx show "add OAuth support"`
2. Delegate codebase exploration to subagent: "Understand current auth implementation"
3. Start: `yx start "add OAuth support"`
4. "Let's spec this out. Which OAuth providers? What's the user experience?"
5. Discuss, write spec to context
6. "This feels big. I'm thinking we break it into:
   - Configure OAuth providers
   - Add OAuth login UI
   - Handle OAuth callbacks
   - Link OAuth accounts to existing users
   Does that make sense?"
7. User confirms
8. Iron Law: `yx add` then `yx ls` for each sub-yak
9. Write detailed context for each sub-yak
10. "These sub-yaks should be ready for zero-context implementation. Look good?"
```

### Pattern 3: Has @needs-human

```
User: Prepare "refactor user model"

You:
1. Check: `yx show "refactor user model"`
2. Notice @needs-human tag
3. Read context, find question: "Should we migrate existing data or maintain backwards compatibility?"
4. "This yak has an open question: [question]. Want to answer now, or should I prepare a different yak?"
5. If answered: remove tag, continue preparation
6. If deferred: acknowledge, ask if they want to prepare something else
```

## Common Pitfalls

### ❌ Don't: Prescribe Specific Techniques

- Don't mandate TDD, example mapping, ADRs, or any specific practice
- Don't require a rigid type system (feature/refactoring/chore)
- Let teams use their own approach

### ❌ Don't: Create Planning Documents

- Don't write a separate planning doc
- The yak tree **is** the plan
- Context lives on yaks, not in separate files

### ❌ Don't: Make Sub-Yaks Too Vague

**Bad sub-yak context:**
```markdown
# Update API
- Change the API
```

**Good sub-yak context:**
```markdown
# Add pagination to user listing endpoint

## Goal
Add limit/offset pagination to GET /users endpoint

## Files to Change
- `src/api/users.ts` — add limit/offset params, modify query
- `src/api/users.test.ts` — test pagination behavior

## Acceptance Criteria
- [ ] Endpoint accepts `limit` and `offset` query params
- [ ] Returns paginated results
- [ ] Returns total count in response header
- [ ] Tests cover edge cases (invalid params, empty results)
```

### ❌ Don't: Forget the Iron Law

**Always** run `yx ls` after `yx add` to verify structure.

### ✅ Do: Keep the Conversation Clean

- Delegate codebase exploration to subagents
- Focus the main conversation on spec discussion with the human
- Bring back summaries, not transcripts

### ✅ Do: Confirm at Each Phase

- After spec: "Does this capture what you want?"
- After acceptance criteria: "Do these define 'done'?"
- After breaking down: "Does this structure make sense?"
- Before finishing: "Ready for implementation?"

## Success Criteria

A well-prepared yak has:
- ✅ Clear spec in context (goal, scope, key decisions, why)
- ✅ Defined acceptance criteria (however the team prefers)
- ✅ Either ready as-is, OR broken into sub-yaks with detailed context
- ✅ All sub-yaks have enough detail for zero-context implementation
- ✅ Open questions either answered or tagged @needs-human
- ✅ Tagged @ready if complete

## Next Steps

After preparation:
- **If ready:** Use `/yx:work` to implement
- **If blocked:** Surface blockers, tag @needs-human if needed
- **If needs mapping:** Use `/yx:map` to explore structure first
