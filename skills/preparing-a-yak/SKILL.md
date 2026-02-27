---
name: preparing-a-yak
description: Use when a yak needs requirements, examples, and a plan before implementation - prepares a yak so it's ready for subagent-driven development
---

# Preparing a Yak

## Overview

**Preparing a yak turns a vague idea into a buildable spec with a
plan and sub-yaks.** Each phase stores its output on the yak itself
using `yx` fields, so everything travels with the yak.

Yaks come in different flavours. A feature adds or changes observable
behaviour. A refactoring changes structure while preserving behaviour.
A chore is everything else (dependency upgrades, CI fixes, tooling).
The preparation flow adapts to the type.

## When to Use

- Yak exists but has no context, or context is vague
- Before picking up a yak for implementation
- When a yak needs requirements fleshed out before coding

**Don't use when:**
- Yak already has context, examples, and plan
- Yak is a simple, obvious task (just pick it up)

## Phase 0: Read Existing State

Before doing anything, understand what you're working with:

1. **Read the yak's current state:**
   ```bash
   yx context --show "yak name"
   yx field --show "yak name" examples
   yx field --show "yak name" plan
   ```

2. **Read parent and sibling yaks** for scope and constraints:
   ```bash
   yx ls
   ```

3. **Explore relevant code** — read the files and tests that
   this yak will touch. Ground yourself in the codebase before
   brainstorming.

Skip phases that already have approved content. If the yak
already has a solid spec in context, go straight to Phase 2.

## Phase 1: Mark the Yak as WIP

**Before doing any visible work**, claim the yak:

```bash
yx state "yak name" wip
```

Preparation is active work. Other agents and the human need to
see that this yak is being worked on.

## Phase 2: Brainstorm the Spec

Use `/brainstorming` to explore the idea collaboratively with
the user.

**Adaptation:** Instead of writing the spec to a file, store it
as the yak's context:

```bash
cat <<'EOF' | yx context "yak name"
# Goal
[What this yak accomplishes]

# Type
[Feature | Refactoring | Chore]

# Success Criteria
[Specific, testable criteria]

# Design Decisions
[Key decisions from brainstorming]

# See Also
- `yx field --show "yak name" examples` for detailed behaviour
- `yx field --show "yak name" plan` for implementation plan
EOF
```

**Done when:** The user approves the spec in context, and the
yak's type (feature, refactoring, or chore) is clear.

## Phase 3: Deepen Understanding

This phase adapts based on the yak's type.

### Feature — Example Map the Behaviour

Use when the yak **adds or changes observable behaviour**: new
commands, changed output, new CLI flags, modified user-facing
semantics.

Use `/example-mapping` to discover rules, examples, and
questions.

**CRITICAL: Go one rule at a time, not all at once.**

The process:

1. **Identify all the rules** from the spec. List them as a
   short numbered checklist (rule name only, one line each) so
   the user can see the full scope.

2. **Present one rule at a time.** For each rule:
   - State the rule clearly
   - Give 2-3 concrete examples (including edge cases)
   - Ask the user: does this rule look right? Any examples
     missing?
   - Wait for confirmation before moving to the next rule.

3. **After all rules are confirmed, go through questions one at
   a time.** For each question:
   - State the question and why it matters
   - Suggest options if you have them
   - Wait for the user's answer
   - Record the decision (update the relevant rule or note the
     deferral)

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

**Done when:** All rules confirmed, all questions answered or
deferred, and the example map is stored on the yak.

### Refactoring — Write an ADR

Use when the yak **changes code structure while preserving
behaviour**: extracting modules, renaming concepts, changing
internal APIs, reorganising files.

The key artifact is an Architecture Decision Record capturing
**why** the change is being made and **what** the target
structure looks like.

1. **Discuss the motivation and trade-offs** with the user.
   What's wrong with the current structure? What does the
   target look like? What are the risks?

2. **Write the ADR** following the project's template:

```bash
cat <<'EOF' > docs/adr/NNNN-short-title.md
# Short Title

Date: YYYY-MM-DD

## Status

Proposed

## Context

[What is motivating this change? What problems does the
current structure cause?]

## Decision

[What structural change are we making? Describe the target
architecture clearly enough that someone could implement it.]

## Consequences

[What becomes easier? What becomes harder? What risks exist
during the migration?]
EOF
```

3. **Store a reference on the yak:**

```bash
cat <<'EOF' | yx field "yak name" adr
docs/adr/NNNN-short-title.md
EOF
```

**Done when:** The user approves the ADR.

### Chore — Skip This Phase

For dependency upgrades, CI fixes, tooling changes, and other
work that doesn't change behaviour or architecture: the spec
from Phase 2 is sufficient. Proceed to Phase 4.

## Phase 4: Write the Implementation Plan

Use `/writing-plans` to create a step-by-step implementation
plan.

**Ground the plan in the codebase.** Before writing tasks:
- Map which files will be created or modified
- Identify exact file paths and line ranges
- Follow existing patterns in the codebase
- Design each task to produce self-contained, testable changes

**Adapt based on yak type:**
- **Feature:** Plan tasks should follow TDD — failing test,
  implementation, verification, commit. Use the example map
  to drive which tests to write.
- **Refactoring:** Plan tasks should preserve behaviour at
  every step. Each task should leave tests passing. Consider
  whether existing tests are sufficient or need strengthening
  first.
- **Chore:** Plan tasks can be more mechanical. Still commit
  after each logical step.

**Store the plan on the yak:**

```bash
cat <<'EOF' | yx field "yak name" plan
# Implementation Plan

## File Structure
[Which files will be created/modified and why]

## Tasks
1. [First task — specific files, test approach, commands]
2. [Second task]
...

## Dependencies
[Which tasks depend on others, what can be parallelised]
EOF
```

**Done when:** The user approves the plan.

## Phase 5: Break Down into Sub-Yaks

Create sub-yaks from the plan's tasks and arrange them using
the `/yak-mapping` nesting pattern.

**Children are prerequisites — leaf nodes get done first.**

```bash
# Create sub-yaks from plan tasks
yx add task A --under "yak name"
yx ls
yx add task B --under "yak name"
yx ls

# Nest so dependencies are expressed through hierarchy
# If task B depends on task A, make A a child of B:
yx move "task A" --under "task B"
yx ls
```

**Add context to each sub-yak** with enough detail for an
agent to implement it with zero prior context. Include:
- Goal (1 sentence)
- Specific files to create/modify (exact paths)
- Test approach (what to test, how)
- Definition of done (specific, testable)

```bash
cat <<'EOF' | yx context "task A"
# Goal
[What this sub-yak accomplishes]

# Files
- Create: src/path/to/new_file.rs
- Modify: src/path/to/existing.rs
- Test: features/something.feature (or tests/...)

# Definition of Done
- [Specific criterion]
- [Another criterion]
- All tests pass: `dev check`
EOF
```

**Follow the Iron Law:** run `yx ls` after every `yx add` or
`yx move` to keep the human in sync.

The tree enforces execution order: work leaves first, then
their parents.

## After Preparation

The yak now has everything needed for implementation:
- **context**: The spec (what and why)
- **examples** (features only): The behaviour (rules and
  edge cases)
- **adr** (refactorings only): The architectural decision
- **plan**: The how (ordered tasks with file paths)
- **sub-yaks**: The work breakdown (dependency hierarchy)

**Next step:** Use `/parallel-yak-implementation` for
independent leaf yaks, or `/subagent-driven-development`
to execute sequentially.

## Quick Reference

| Phase | What | Skill | Stored In | Read With |
|-------|------|-------|-----------|-----------|
| 0 | Existing state | — | — | `yx context --show`, `yx field --show`, `yx ls` |
| 1 | Mark WIP | — | state | `yx ls` |
| 2 | Spec | `/brainstorming` | context | `yx context --show "name"` |
| 3a | Behaviour (feature) | `/example-mapping` | examples field | `yx field --show "name" examples` |
| 3b | Decision (refactoring) | — | adr field + file | `yx field --show "name" adr` |
| 3c | — (chore) | skip | — | — |
| 4 | Plan | `/writing-plans` | plan field | `yx field --show "name" plan` |
| 5 | Sub-yaks | `/yak-mapping` | yak hierarchy | `yx ls` |

## Common Mistakes

| Mistake | Fix |
|---------|-----|
| Jumping straight to planning without brainstorming | Phase 2 first — understand what before how |
| Not reading existing state before starting | Phase 0 — check what's already there |
| Not exploring the codebase before planning | Read the relevant files in Phase 0 and before Phase 4 |
| Example mapping a chore or refactoring | Only features need example mapping |
| Skipping ADR for a significant refactoring | If you're changing structure, capture the decision |
| Writing examples without a spec | Brainstorm the spec first, then map examples against it |
| Skipping example mapping for a feature with multiple rules | If it has rules and edge cases, map it |
| Storing outputs in files instead of on the yak | Always use `yx context` and `yx field` |
| Starting implementation without user approval at each phase | Each phase ends with user confirmation |
| Leaving plan tasks as flat siblings | Use `/yak-mapping` nesting to order by dependency |
| Writing plan tasks without exact file paths | Ground every task in specific files and commands |
| Adding sub-yaks without running `yx ls` after each | Iron Law — `yx add` then `yx ls`, always |
