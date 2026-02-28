---
name: parallel-yak-implementation
description: Use when multiple independent leaf yaks are ready to implement and can be worked on concurrently with separate agents in worktrees
---

# Parallel Yak Implementation

## Overview

When multiple leaf yaks are independent and have plans, dispatch
one agent per yak in parallel worktrees. This skill wraps
`superpowers-dev:dispatching-parallel-agents` with yak-specific
lifecycle steps.

## When to Use

- 2+ leaf yaks are ready (have plans, no children)
- The yaks don't modify the same files
- You want to maximize throughput

## The Process

### 1. Identify Ready Leaves

```bash
yx ls
```

Leaf yaks (no children) with plans are candidates. Verify
independence: check their plans don't touch the same files.

### 2. Mark ALL as WIP Before Dispatching

**This is the FIRST thing you do after identifying yaks.**

```bash
yx state "<yak-name>" wip
```

Do this for EVERY yak BEFORE launching any agents. This signals
to the human (and future sessions) what's being worked on.

**Verify with `yx ls`** - all target yaks should show as wip
before you proceed to step 3.

### 3. Dispatch One Agent Per Yak

**REQUIRED:** Use `superpowers-dev:dispatching-parallel-agents`
for the dispatch pattern.

Each agent prompt should include:
- The yak's plan (from `yx field --show "<name>" plan`)
- Instruction to invoke `superpowers-dev:using-git-worktrees`
- Instruction to invoke `superpowers-dev:executing-plans`
- Project context (test commands, commit conventions)
- Clear scope: only work on this yak's files

### 4. When Agents Complete

For each returning agent:
1. Review the summary
2. Verify with `dev check` in the worktree
3. Mark yak done: `yx state "<name>" done`
4. Use `superpowers-dev:finishing-a-development-branch`

If an agent fails, keep the yak as wip and investigate.

## ⛔ The Main Branch Rule

**Neither you nor your subagents may modify main's working
tree.** Main is for merging finished branches only.

- **Subagents** must each work in their own worktree
- **The orchestrating agent** (you) must not stash, clean,
  or edit files on main
- If you see uncommitted changes on main, **stop and ask
  the user** — they belong to someone else

## Common Mistakes

**Working directly on main** - The most dangerous mistake.
Always dispatch subagents with worktree instructions. If a
subagent reports it's working on main, kill it immediately.

**Forgetting to mark wip** - Agents start working but yaks
still show as todo. Always mark wip BEFORE dispatching.

**Overlapping file changes** - If two yaks modify the same
files, work them sequentially instead.

**Not reviewing agent output** - Always verify before marking
done. Agents can make systematic errors.
