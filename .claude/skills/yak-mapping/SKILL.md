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

## ⚠️ CRITICAL: Use yx CLI Only ⚠️

**NEVER touch .yaks directory directly!**
- ✅ Use: `yx add`, `yx move`, `yx rm`, `yx context`
- ❌ Never: `rm -rf .yaks`, `mkdir .yaks/...`, `cat > .yaks/...`

This is dogfooding - we use yaks to build yaks.

## ⚠️ THE IRON LAW ⚠️

**After EVERY `yx add`, immediately run `yx ls` to show what changed.**

No exceptions:
- Not "I'll show it at the end"
- Not "just adding a quick one"
- Not "the structure is obvious"

`yx add` → `yx ls` is non-negotiable. This keeps the human in sync with your thinking.

## The Approach Pattern

### Core Loop (ONE yak at a time)

```
1. Add ONE yak
2. Show map with `yx ls`
3. Add context to that yak
4. Pick ONE child to explore next
5. Repeat
```

**CRITICAL:** After EVERY `yx add`, run `yx ls` to show the human what changed.

### Step-by-Step Process

**1. Start with the Goal**
```bash
yx add "sync"
yx ls              # Always show after adding
```

**2. Approach It (Don't Decompose)**

Ask yourself: "If we tried to implement this RIGHT NOW, what would we try first?"

Don't ask: "What are all the components?"

**3. Discover ONE Blocker**

When approaching reveals "we need X first", add X:
```bash
yx add "sync/write events to git ref"
yx ls              # Show the updated map
```

**The nesting means:** "sync is BLOCKED BY write events"

**4. Add Context Before Going Deeper**
```bash
yx context "sync/write events to git ref"
# Add goal + done + known knowns/unknowns
```

**5. Approach This Blocker**

Now explore this one level deeper:
```bash
# Approaching "write events" reveals we need log
yx add "sync/write events to git ref/implement log command"
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

## Yak Granularity

**Leaf yaks should be implementable in one TDD cycle (20-40 minutes):**
- Write failing test (5 min)
- Implement minimal code (10-20 min)
- Refactor if needed (5-10 min)
- Commit (1 min)

**Right-sized yaks:**
- ✅ Approaching reveals 2-4 blockers → probably good size
- ✅ Approaching reveals 0 blockers and feels ready to implement → perfect leaf
- ❌ Approaching reveals 0 blockers and feels tiny → too granular
- ❌ Approaching reveals 6+ blockers → too large, needs intermediate level

**When to stop exploring a branch:**
- Hit a leaf (can implement without discovering new blockers)
- Going deeper requires actually doing the work (not just thinking)

## Context Pattern

**Write contexts assuming someone else will implement - zero context assumption.**

Add context showing:
- **Goal**: What this accomplishes (1 sentence)
- **Definition of Done**: Specific, testable criteria (3-5 bullets)
- **Known Knowns**: Decisions already made, specific file paths, specific patterns
- **Known Unknowns**: Open questions (that will be answered during implementation)

### Definition of Done - Be SPECIFIC

- ✅ "InMemoryStorage implements StoragePort (save/load/list/delete/exists)"
- ✅ "File created: src/adapters/memory_storage.rs"
- ✅ "Unit tests pass: cargo test memory_storage"
- ❌ "storage works" (too vague)
- ❌ "add tests" (what tests? where?)

### Known Knowns - Include specifics

- File paths: "Will live in src/adapters/memory_storage.rs"
- Patterns: "Use Arc<RwLock<HashMap<String, Yak>>> for thread-safety"
- Dependencies: "Implements StoragePort from src/ports/mod.rs"
- Similar work: "Follow pattern from DirectoryStorage"

### Known Unknowns - Specific questions

- ✅ "Does OutputPort trait exist in src/ports/mod.rs?"
- ❌ "how to do output?" (too vague)

### Example Context

```bash
cat <<'EOF' | yx context "sync/write events to git ref"
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

## Real-Time Updates

```bash
# After each add, show structure
yx ls

# Pattern:
yx add "parent/child"
yx ls                    # Verify it's there
yx context "parent"      # Add context
yx add "parent/child/grandchild"
yx ls                    # Show updated map
```

This keeps the human in sync with your thinking.

## TDD and Yak Granularity

Leaf yaks should align with TDD cycles. A well-scoped leaf yak suggests its first test:

**Example - Good leaf yak context:**
```markdown
# Goal
Implement in-memory storage using HashMap for testing.

# Definition of Done
- InMemoryStorage implements StoragePort
- Unit tests pass for save/load/list/delete/exists
- Exported from src/adapters/mod.rs

# Known Knowns
- Use Arc<RwLock<HashMap<String, Yak>>>
- First test: save and load a yak

# Known Unknowns
- Thread-safety concerns beyond basic RwLock?
```

**The first test is obvious:** Test that you can save and load a yak. This confirms the yak is properly scoped for one TDD session.

**If you can't envision the first test**, the yak might be too vague or too large - approach it to discover more structure.

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
- Follow the TDD cycle: test → fail → implement → pass → commit

## Common Mistakes

### ❌ Top-Down Decomposition
```bash
# WRONG: Planning all components upfront
yx add "sync/event logging"
yx add "sync/git storage"
yx add "sync/replay algorithm"
# You haven't approached anything yet!
```

### ✅ Discovery Through Approach
```bash
# RIGHT: What would we try first?
yx add "sync"
# "If we approached sync, we'd need to write events"
yx add "sync/write events to git ref"
# "If we approached that, we'd need log command"
yx add "sync/write events to git ref/implement log"
```

### ❌ Over-Planning Context
Writing detailed implementation plans in context before discovering blockers.

### ✅ Lightweight Context
Goal + done + key decisions. Details emerge when you work on it.

### ❌ Batch Creation
Adding 5 yaks without showing structure between each.

### ✅ Incremental Updates
Add one, show map, add context, add next, show map.

## Why This Works

- **Structure emerges from reality** - actual blockers, not guessed components
- **Parent enforces order** - can't mark parent done until children complete
- **Leaf nodes are ready** - no unknown blockers
- **Flexible** - structure changes as you learn

## Integration

Use with:
- **structuring-yak-dependencies**: Why parent/child nesting works
- **yak-worktree-workflow**: How to work on individual leaf yaks

## Quick Reference

| Action | Command |
|--------|---------|
| Add goal | `yx add "goal"` |
| Add blocker | `yx add "goal/blocker"` |
| Show map | `yx ls` |
| Add context | `yx context "name"` (uses stdin) |
| Read context | `yx context --show "name"` |

## Red Flags - Wrong Approach

- **Adding multiple yaks without `yx ls` between them**
- **Showing markdown structure instead of actual `yx` commands**
- **Touching .yaks directory directly instead of using yx CLI**
- **Using writing-plans or other planning skills when user says "plan with yaks"**
- Creating all yaks before showing any structure
- Planning "components" instead of discovering blockers
- Writing implementation details in context before approaching work
- Nesting by "feels like subtask" instead of "blocks the parent"
- **Exploring 3+ levels deep before adding context to parents**
- Definition of done too vague ("make it work", "add tests")
- Known knowns without specifics (no file paths, patterns, or examples)

**If you catch yourself doing these, stop and restart with approach-first.**
