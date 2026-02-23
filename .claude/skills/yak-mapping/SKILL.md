---
name: yak-mapping
description: Use when planning work by approaching goals and discovering blockers, before creating comprehensive plans
---

# Yak Mapping

## Overview

**Yak mapping is emergent planning through action.** You discover work structure by approaching goals and finding what blocks you, not by decomposing from the top down.

**Core principle:** "It is in the doing of the work that we discover the work that we must do." — Woody Zuill

## Announcement

**Always start yak mapping by saying:**

"I'm using yak-mapping to discover the work structure by approaching the goal. I'll add yaks one at a time and show the map after each addition."

This sets expectations that we're doing emergent discovery, not top-down planning.

## When to Use

Use when:
- User asks you to "plan" or "map out" work using yaks
- User says "adapt for yaks" or "plan with yaks" or "break down into yaks"
- **User wants to structure work using the yak tool (this project)**
- Starting to structure a new feature or goal
- Need to break down complex work

**NOT for writing plan documents** - this creates actual yaks, not markdown plans.

Don't use when:
- Just executing already-mapped work
- Single straightforward task
- User provides detailed step-by-step plan

## CRITICAL: Use yx CLI Only

**NEVER touch .yaks directory directly!**
- Use: `yx add`, `yx move`, `yx rm`, `yx context`
- Never: `rm -rf .yaks`, `mkdir .yaks/...`, `cat > .yaks/...`

This is dogfooding - we use yaks to build yaks.

## THE IRON LAW

**After EVERY `yx add`, immediately run `yx ls` to show what changed.**

No exceptions:
- Not "I'll show it at the end"
- Not "just adding a quick one"
- Not "the structure is obvious"

`yx add` then `yx ls` is non-negotiable. This keeps the human in sync with your thinking.

## The Approach Pattern

### Core Loop (ONE yak at a time)

```
1. Add ONE yak
2. Show map with `yx ls`  (THE IRON LAW)
3. Add context to that yak
4. Pick ONE child to explore next
5. Repeat
```

### Step-by-Step Process

**1. Start with the Goal**
```bash
yx add sync
yx ls              # Always show after adding
```

**2. Approach It (Don't Decompose)**

Ask yourself: "If we tried to implement this RIGHT NOW, what would we try first?"

Don't ask: "What are all the components?"

**3. Discover ONE Blocker**

When approaching reveals "we need X first", add X as a child:
```bash
yx add write events to git ref --under sync
yx ls              # Show the updated map
```

**The nesting means:** "sync is BLOCKED BY write events"

**4. Add Context Before Going Deeper**
```bash
yx context write events to git ref
# Add goal + done + known knowns/unknowns
```

**5. Approach This Blocker**

Now explore this one level deeper:
```bash
# Approaching "write events" reveals we need log
yx add implement log command --under "write events to git ref"
yx ls              # Always show after adding
```

**6. Continue ONE Level at a Time**

Keep approaching and discovering until you hit a leaf node (can implement without discovering new blockers).

### When to Stop Exploring a Branch

Stop when:
- You've reached a leaf (no new blockers discovered)
- You've identified enough structure to start work
- Going deeper requires actually doing the work (not just thinking)

Then explore other branches or let someone start implementing leaves.

## Why Nesting Works

Yaks enforces: **parent cannot be marked done if it has incomplete children**.

The nesting is an **artifact of discovery**, not a planning decision. You literally CAN'T complete the parent until you clear the blocker.

**Growth is bidirectional:**
- **Downward**: Approach a goal, discover blockers -> add them as children
- **Upward**: Working on a goal, realize it's part of something bigger -> create parent with `yx move`

**Multiple blockers** all become children:
```bash
yx add deploy to production
yx add deployment script --under "deploy to production"
yx add configure secrets --under "deploy to production"
yx add update documentation --under "deploy to production"
```

You must complete all three before the parent goal is achievable.

**Work deepest-first** (leaves before parents):
```
deploy to production
├─ ○ deployment script    <- Start here
├─ ○ configure secrets    <- Or here
╰─ ○ update documentation <- Or here
```

The structure naturally guides you to unblocked work (leaf nodes).

## Reorganizing Flat Yaks into a Dependency Hierarchy

Sometimes you have a flat set of sibling yaks and realize they have
phased dependencies. To restructure them:

**The rule: later phases are parents, earlier phases are children.**
Phase 1 items nest under the Phase 2 item they block, not the other
way around.

**Example:** You're planning a tea party. You start with flat yaks:

```
tea party
├─ ○ buy teapot
├─ ○ buy tea leaves
├─ ○ brew tea
├─ ○ invite friends
╰─ ○ serve tea
```

Then you realize: you can't serve tea until it's brewed and friends
are invited. You can't brew until you have a teapot and leaves.
Restructure so prerequisites nest under what they block:

```bash
yx move "brew tea" --under "serve tea"
yx move "invite friends" --under "serve tea"
yx move "buy teapot" --under "brew tea"
yx move "buy tea leaves" --under "brew tea"
```

Result:
```
tea party
╰─ ○ serve tea
   ├─ ○ brew tea
   │  ├─ ○ buy teapot
   │  ╰─ ○ buy tea leaves
   ╰─ ○ invite friends
```

Now the tree enforces the order: work leaves first (buy teapot, buy
tea leaves, invite friends), then brew, then serve.

## Yak Granularity

**Leaf yaks should be implementable in one TDD cycle (20-40 minutes):**
- Write failing test (5 min)
- Implement minimal code (10-20 min)
- Refactor if needed (5-10 min)
- Commit (1 min)

**Right-sized yaks:**
- Approaching reveals 2-4 blockers -> probably good size
- Approaching reveals 0 blockers and feels ready to implement -> perfect leaf
- Approaching reveals 0 blockers and feels tiny -> too granular
- Approaching reveals 6+ blockers -> too large, needs intermediate level

## Context Pattern

**Write contexts assuming someone else will implement - zero context assumption.**

Add context showing:
- **Goal**: What this accomplishes (1 sentence)
- **Definition of Done**: Specific, testable criteria (3-5 bullets)
- **Known Knowns**: Decisions already made, specific file paths, specific patterns
- **Known Unknowns**: Open questions (that will be answered during implementation)

### Definition of Done - Be SPECIFIC

- "InMemoryStorage implements StoragePort (save/load/list/delete/exists)"
- "File created: src/adapters/memory_storage.rs"
- "Unit tests pass: cargo test memory_storage"
- NOT: "storage works" (too vague)
- NOT: "add tests" (what tests? where?)

### Example Context

```bash
cat <<'EOF' | yx context write events to git ref
# Goal
Commands are logged as events in git for replay.

# Definition of Done
- First Gherkin scenario passes - commands appear in log
- Events written to refs/notes/yaks
- Can read log with `yx log` command
- Unit tests pass for write_event()

# Known Knowns
- Events write to refs/notes/yaks (git notes)
- Commit format: headline = command, body = stdin
- Need `yx log` command to verify
- Similar to git log implementation pattern
- Use git2-rs or shell out to git

# Known Unknowns
- Which commands are plumbing vs porcelain?
- git2-rs vs shell out - performance tradeoff?
- Do we need event schema versioning?
EOF
```

**Balance:** Specific enough for someone else to implement, but light enough that details emerge during work.

## After Mapping is Complete

Once you've discovered blockers and identified leaf nodes:

**Present the map and ask:**

```
Mapping complete! Ready-to-implement leaf yaks:
- [list leaf nodes with their parent context]

Next steps:
1. Pick up a leaf yak (I'll create worktree and start TDD)
2. You choose which yak to start with
3. Review the map for now

Which would you like?
```

**If user chooses to implement:**
- Use **superpowers:using-git-worktrees** to create isolated workspace
- Use **superpowers:test-driven-development** for implementation
- Follow the TDD cycle: test -> fail -> implement -> pass -> commit

## Common Mistakes

### Top-Down Decomposition
```bash
# WRONG: Planning all components upfront
yx add event logging --under sync
yx add git storage --under sync
yx add replay algorithm --under sync
# You haven't approached anything yet!
```

### Discovery Through Approach
```bash
# RIGHT: What would we try first?
yx add sync
# "If we approached sync, we'd need to write events"
yx add write events to git ref --under sync
# "If we approached that, we'd need log command"
yx add implement log --under "write events to git ref"
```

### Inverted Nesting
```bash
# WRONG: Makes CI look like it's part of local setup
yx add setup local dev lint --under "add ci workflow"

# RIGHT: CI is blocked BY local setup
yx add add ci workflow
yx add setup local dev lint --under "add ci workflow"
```

### Other Red Flags

- Adding multiple yaks without `yx ls` between them (Iron Law!)
- Showing markdown structure instead of actual `yx` commands
- Touching .yaks directory directly instead of using yx CLI
- Using writing-plans when user says "plan with yaks"
- Over-planning context before discovering blockers
- Nesting by "feels like subtask" instead of "blocks the parent"
- Exploring 3+ levels deep before adding context to parents
- Vague definitions of done ("make it work", "add tests")

**If you catch yourself doing these, stop and restart with approach-first.**

## Integration

Use with:
- **yak-worktree-workflow**: How to work on individual leaf yaks

## Quick Reference

| Action | Command |
|--------|---------|
| Add goal | `yx add goal name` |
| Add blocker | `yx add blocker --under "goal name"` |
| Show map | `yx ls` |
| Add context | `yx context yak name` (uses stdin) |
| Read context | `yx context --show yak name` |
| Move yak under parent | `yx move yak name --under "parent name"` |
| Move to root | `yx move yak name --to-root` |
