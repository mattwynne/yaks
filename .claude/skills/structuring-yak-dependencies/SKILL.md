---
name: structuring-yak-dependencies
description: Use when approaching a goal and discovering blockers, to model emergent prerequisites through hierarchy
---

# Structuring Yak Dependencies

**Model emergent goal discovery through parent-child nesting**

## Core Philosophy

> "It is in the doing of the work that we discover the work that we must do." — Woody Zuill

Yaks implements the [Mikado Method](https://mikadomethod.info) and [Discovery Trees](https://www.fastagile.io/method/product-mapping-and-discovery-trees): you discover prerequisites by **approaching goals and finding blockers**.

**Related skills:** See `mikado-method` and `discovery-tree-workflow` for the foundational techniques this pattern applies to yaks.

## How Goals Emerge

**Work is fractal.** Each goal, when approached, may reveal smaller, more achievable goals that support the journey.

**Growth is bidirectional:**
- **Downward**: Approach a goal, discover blockers → add them as children
- **Upward**: Working on a goal, realize it's part of something bigger → create parent

**Bias for action:** Do this during actual work, not planning sessions. Write what you think you know, but prepare to be surprised.

## The Pattern

When you approach goal B and discover you need A first, make B the **parent** of A:

```bash
# You start here
yx add "add github actions workflow"

# Try to implement it, realize you need local tooling first
yx add "add github actions workflow/setup local dev lint"

# Later, realize both are part of broader linting initiative
yx move "add github actions workflow" "add shellcheck linting/add github actions workflow"
```

## Why Nesting Works

Yaks enforces: **parent cannot be marked done if it has incomplete children**.

The nesting is an **artifact of discovery**, not a planning decision. You literally CAN'T complete the parent until you clear the blocker.

## Multiple Blockers

If you discover 3 things blocking a goal, they ALL become children:

```bash
yx add "deploy to production"
yx add "deploy to production/add deployment script"
yx add "deploy to production/configure secrets"
yx add "deploy to production/update documentation"
```

You must complete all three before the parent goal is achievable.

## Work Order

Work **deepest-first** (leaves before parents):

```
goal/
└── dependent-work/
    ├── blocker-1    ← Start here
    ├── blocker-2    ← Or here
    └── blocker-3    ← Or here
```

The structure naturally guides you to unblocked work (leaf nodes).

## When to Use

**Use when:**
- Actively working on a goal and hit a blocker
- Realize prerequisite work exists
- Multiple blockers discovered for one goal
- Need structural enforcement of order (not just documentation)

**Don't use when:**
- Just brainstorming without attempting work
- Goals are independent (use siblings)
- Doing pure upfront planning (bias for action instead)

## Real Example

You're asked to "add CI for linting":

```bash
# 1. Start with what you know
yx add "add github actions workflow"

# 2. Approach it, read docs, realize you need local command first
yx add "add github actions workflow/setup local dev lint"

# 3. Start on local setup, discover two blockers
yx add "add github actions workflow/setup local dev lint/add shellcheck to devenv"
yx add "add github actions workflow/setup local dev lint/fix existing violations"

# 4. Work deepest-first: add shellcheck → fix violations → setup complete → CI works
```

The structure emerged from doing the work, not planning it upfront.

## Common Mistakes

**Mistake:** Planning the full structure before starting work

Don't create elaborate hierarchies upfront. Add structure as you discover blockers.

**Mistake:** Nesting by "feels like a sub-task" instead of "blocks the goal"

```bash
# ❌ WRONG: Inverted - makes CI look like it's part of local setup
yx add "setup local dev lint/add ci workflow"

# ✅ RIGHT: CI is blocked BY local setup
yx add "add ci workflow/setup local dev lint"
```

**Mistake:** Using siblings and relying on context docs

Context helps humans but doesn't enforce order. Nesting enforces order structurally.

## Working with Structured Yaks

Once you've structured dependencies, use the `yak-worktree-workflow` skill to work on individual yaks in isolation.
