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

**Adaptation:** Instead of leaving the Gherkin skeleton in conversation, store it on the yak:

```bash
# Store the example map as a field
cat <<'EOF' | yx field "yak name" examples
Feature: [yak name]

  Rule: [first rule]
    Example: [concrete example]
    Example: [edge case]

  Rule: [second rule]
    Example: [concrete example]

  # Questions:
  # - [unanswered question]
EOF
```

**Done when:** The user confirms the rules and examples cover the scope. Questions are either answered or explicitly deferred.

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

## After Preparation

The yak now has everything needed for implementation:
- **context**: The spec (what and why)
- **examples**: The behaviour (rules and edge cases)
- **plan**: The how (ordered tasks)

**Next step:** Use `/subagent-driven-development` to execute the plan.

## Quick Reference

| Phase | Skill | Stored In | Command to Read |
|-------|-------|-----------|-----------------|
| Spec | `/brainstorming` | context | `yx context --show "name"` |
| Behaviour | `/example-mapping` | examples field | `yx field --show "name" examples` |
| Plan | `/writing-plans` | plan field | `yx field --show "name" plan` |

## Common Mistakes

| Mistake | Fix |
|---------|-----|
| Jumping straight to planning without brainstorming | Phase 1 first - understand what before how |
| Writing examples without a spec | Brainstorm the spec first, then map examples against it |
| Skipping example mapping for "simple" yaks | If it has multiple rules or edge cases, map it |
| Storing outputs in files instead of on the yak | Always use `yx context` and `yx field` |
| Starting implementation without user approval at each phase | Each phase ends with user confirmation |
