---
name: preparing-a-yak
description: Use when a yak needs requirements, examples, and a plan before implementation - prepares a yak so it's ready for subagent-driven development
---

# Preparing a Yak

## Overview

**Preparing a yak turns a vague idea into a buildable spec with examples and a plan.** Each phase stores its output on the yak itself using `yx` fields, so everything travels with the yak.

## When to Use

- Yak exists but has no context, or context is vague
- Before picking up a yak for implementation
- When a yak needs requirements fleshed out before coding

**Don't use when:**
- Yak already has context, examples, and plan
- Yak is a simple, obvious task (just pick it up)

## First: Mark the Yak as WIP

**Before doing anything else**, claim the yak:

```bash
yx state "yak name" wip
```

Preparation is active work. Other agents and the human need to
see that this yak is being worked on. Do this BEFORE Phase 1.

## The Three Phases

### Phase 1: Brainstorm the Spec

Use `/brainstorming` to explore the idea collaboratively with the user.

**Adaptation:** Instead of writing the spec to a file, store it as the yak's context:

```bash
# Pipe the spec into the yak's context
cat <<'EOF' | yx context "yak name"
# Goal
[What this yak accomplishes]

# Success Criteria
[Specific, testable criteria]

# Design Decisions
[Key decisions from brainstorming]

# See Also
- `yx field --show "yak name" examples` for detailed behaviour
- `yx field --show "yak name" plan` for implementation plan
EOF
```

**Done when:** The user approves the spec in context.

### Phase 2: Example Map the Behaviour

Use `/example-mapping` to discover rules, examples, and questions.

**CRITICAL: Go one rule at a time, not all at once.**

The process:

1. **Identify all the rules** from the spec. List them as a short numbered checklist (rule name only, one line each) so the user can see the full scope.

2. **Present one rule at a time.** For each rule:
   - State the rule clearly
   - Give 2-3 concrete examples (including edge cases)
   - Ask the user: does this rule look right? Any examples missing?
   - Wait for confirmation before moving to the next rule.

3. **After all rules are confirmed, go through questions one at a time.** For each question:
   - State the question and why it matters
   - Suggest options if you have them
   - Wait for the user's answer
   - Record the decision (update the relevant rule or note the deferral)

4. **Store the final example map on the yak:**

```bash
cat <<'EOF' | yx field "yak name" examples
Feature: [yak name]

  Rule: [first rule]
    Example: [concrete example]
    Example: [edge case]

  Rule: [second rule]
    Example: [concrete example]

  # Deferred:
  # - [deferred question or rule]
EOF
```

**Done when:** All rules confirmed, all questions answered or deferred, and the example map is stored on the yak.

### Phase 3: Write the Implementation Plan

Use `/writing-plans` to create a step-by-step implementation plan from the spec and examples.

**Adaptation:** Store the plan on the yak:

```bash
cat <<'EOF' | yx field "yak name" plan
# Implementation Plan

## Tasks
1. [First task - with specific files and test approach]
2. [Second task]
...

## Order
[Dependencies between tasks, what can be parallelised]
EOF
```

**Done when:** The user approves the plan.

## After Preparation: Create Sub-Yaks

If the plan has multiple tasks with dependencies, create
sub-yaks and arrange them hierarchically using the
`/yak-mapping` nesting pattern:

**Children are prerequisites — leaf nodes get done first.**

```bash
# Create sub-yaks from plan tasks
yx add task A --under "yak name" --context "Task N in plan doc"
yx add task B --under "yak name" --context "Task M in plan doc"

# Then nest them so dependencies are expressed through hierarchy
# If task B depends on task A, make A a child of B:
yx move "task A" --under "task B"
```

The tree enforces execution order: work leaves first, then
their parents. Each sub-yak's context references its task
in the plan document.

The yak now has everything needed for implementation:
- **context**: The spec (what and why)
- **examples**: The behaviour (rules and edge cases)
- **plan**: The how (ordered tasks)
- **sub-yaks**: The work breakdown (dependency hierarchy)

**Next step:** Use `/parallel-yak-implementation` for
independent leaf yaks, or `/subagent-driven-development`
to execute sequentially.

## Quick Reference

| Phase | Skill | Stored In | Command to Read |
|-------|-------|-----------|-----------------|
| Spec | `/brainstorming` | context | `yx context --show "name"` |
| Behaviour | `/example-mapping` | examples field | `yx field --show "name" examples` |
| Plan | `/writing-plans` | plan field | `yx field --show "name" plan` |
| Sub-yaks | `/yak-mapping` | yak hierarchy | `yx ls` |

## Common Mistakes

| Mistake | Fix |
|---------|-----|
| Jumping straight to planning without brainstorming | Phase 1 first - understand what before how |
| Writing examples without a spec | Brainstorm the spec first, then map examples against it |
| Skipping example mapping for "simple" yaks | If it has multiple rules or edge cases, map it |
| Storing outputs in files instead of on the yak | Always use `yx context` and `yx field` |
| Starting implementation without user approval at each phase | Each phase ends with user confirmation |
| Leaving plan tasks as flat siblings | Use `/yak-mapping` nesting to order by dependency |
