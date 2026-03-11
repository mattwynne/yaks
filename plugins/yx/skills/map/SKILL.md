---
name: map
description: Discover work structure by approaching goals and finding blockers — emergent planning through action, not top-down decomposition
---

# /yx:map — Yak Mapping

## Overview

**Yak mapping is emergent planning through action.** You discover work structure by approaching goals and finding what blocks you, not by decomposing from the top down.

**Core principle:** "It is in the doing of the work that we discover the work that we must do." — Woody Zuill

When invoked with arguments (e.g., `/yx:map build a REST API`), use those as the initial goal.

## Announcement

**Always start yak mapping by saying:**

"I'm using `/yx:map` to discover the work structure by approaching the goal. I'll add yaks one at a time and show the map after each addition."

This sets expectations that we're doing emergent discovery, not top-down planning.

## When to Use

Use when:
- User asks to "plan" or "map out" work using yaks
- User says "adapt for yaks" or "plan with yaks" or "break down into yaks"
- Starting to structure a new feature or goal
- Need to discover work structure for complex work
- User provides a goal and you need to find blockers

Don't use when:
- Just executing already-mapped work (use `/yx:work`)
- Single straightforward task (just do it)
- User wants detailed specs (use `/yx:prepare` for fleshing out individual yaks)

## THE IRON LAW

**After EVERY `yx add`, immediately run `yx ls` to show what changed.**

No exceptions:
- Not "I'll show it at the end"
- Not "just adding a quick one"
- Not "the structure is obvious"

`yx add` then `yx ls` is non-negotiable. This keeps the human in sync with your thinking.

## The Approach Pattern

### Phase 1: Scope the Goal (BEFORE creating child yaks)

After creating the top-level goal yak, **have a conversation to agree scope before going deeper.** Don't start adding children yet.

1. **Add the goal yak** and show the map
2. **Summarise your understanding** of the goal back to the human
3. **Present candidate areas** you've identified (if any), but as a conversation — NOT as yaks yet
4. **Ask the human to pick** what's in scope for now
5. **Only after agreement**, start the discovery loop on the agreed scope

This prevents runaway mapping where you create yaks for everything you can think of. The human decides what matters now.

**Example:**
```
I've created the goal yak. Before I start mapping blockers, let me check scope with you.

From the research, I can see several areas where this could apply:
- Area A (quick win)
- Area B (medium effort, high impact)
- Area C (ambitious stretch goal)

Which of these do you want to map out now? We can always add the others later.
```

### Phase 2: Discovery Loop (ONE yak at a time)

```
1. Add ONE yak
2. Show map with `yx ls`  (THE IRON LAW)
3. Add brief context to that yak (a sentence or two)
4. Pick ONE child to explore next
5. Repeat
```

### Step-by-Step Process

**1. Start with the Agreed Scope**

Only create child yaks for areas the human has agreed to explore.

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

**4. Add Brief Context**

During mapping, context is lightweight — just enough to remember what this is:
```bash
yx context write events to git ref <<'EOF'
Commands are logged as events in git refs/notes/yaks for replay.
Need to implement `yx log` command to verify.
EOF
```

Detailed specs, acceptance criteria, and comprehensive context come later in `/yx:prepare`.

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
- You hit a question only the human can answer (use `@needs-human`)

Then explore other branches or let someone start implementing leaves.

## @needs-human Convention

When mapping surfaces a question only the human can answer:

1. Tag the yak: `yx tag "yak name" @needs-human`
2. Add the question to the yak's context
3. Move on to another branch

**When encountering a `@needs-human` tag:**

Surface it: "This yak has an open question: [question]. Want to answer now or should I continue mapping elsewhere?"

## Why Nesting Works

Yaks enforces: **parent cannot be marked done if it has incomplete children**.

The nesting is an **artifact of discovery**, not a planning decision. You literally CAN'T complete the parent until you clear the blocker.

**Growth is bidirectional:**
- **Downward**: Approach a goal, discover blockers → add them as children
- **Upward**: Working on a goal, realize it's part of something bigger → create parent with `yx move`

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

Sometimes you have a flat set of sibling yaks and realize they have phased dependencies. To restructure them:

**The rule: later phases are parents, earlier phases are children.** Phase 1 items nest under the Phase 2 item they block, not the other way around.

**Example:** You're planning a tea party. You start with flat yaks:

```
tea party
├─ ○ buy teapot
├─ ○ buy tea leaves
├─ ○ brew tea
├─ ○ invite friends
╰─ ○ serve tea
```

Then you realize: you can't serve tea until it's brewed and friends are invited. You can't brew until you have a teapot and leaves. Restructure so prerequisites nest under what they block:

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

Now the tree enforces the order: work leaves first (buy teapot, buy tea leaves, invite friends), then brew, then serve.

## Yak Granularity

**Leaf yaks should be small enough to just do (20-40 minutes):**
- Could implement in one TDD cycle
- Single focused change
- Can describe "done" in 3-5 bullets

**Right-sized yaks:**
- Approaching reveals 2-4 blockers → probably good size
- Approaching reveals 0 blockers and feels ready to implement → perfect leaf
- Approaching reveals 0 blockers and feels tiny → too granular
- Approaching reveals 6+ blockers → too large, needs intermediate level

**If a yak feels too big:**
- Don't try to spec it out in detail during mapping
- Add a brief context note
- Continue approaching to discover blockers
- The tree will naturally decompose it through discovery

## After Mapping is Complete

Once you've discovered blockers and identified leaf nodes, present the map:

```
Mapping complete! The yak tree:
[show yx ls output]

Ready-to-implement leaf yaks:
- yak name (context: what it does)
- another yak (context: what it does)

Next steps:
1. Use /yx:prepare on any yak that needs detailed specs or acceptance criteria
2. Use /yx:work to pick up a leaf yak and implement it
3. Continue mapping other areas if needed

Which would you like to do?
```

## Common Mistakes

### Top-Down Decomposition
```bash
# WRONG: Planning all components upfront
yx add event logging --under sync
yx add git storage --under sync
yx add replay algorithm --under sync
# You haven't approached anything yet!
```

**Discovery Through Approach:**
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

### Skipping the Scope Conversation
```bash
# WRONG: Immediately creating yaks for every idea
yx add feature A --under goal
yx add feature B --under goal
yx add feature C --under goal
yx add feature D --under goal
# You never asked the human what they actually want!

# RIGHT: Create goal, discuss scope, then map agreed areas
yx add goal
# "I can see areas A, B, C, D. Which should we focus on?"
# Human: "Let's start with B"
yx add feature B --under goal
```

### Writing Full Specs During Mapping
```bash
# WRONG: Detailed spec with acceptance criteria, edge cases, etc.
yx context new feature <<'EOF'
Goal: Implement feature X
Definition of Done:
- Criterion 1
- Criterion 2
- Criterion 3
Edge cases:
- Case A
- Case B
Implementation notes:
...15 more lines...
EOF

# RIGHT: Brief context during mapping, details in /yx:prepare
yx context new feature <<'EOF'
Add feature X to support use case Y.
EOF
```

### Other Red Flags

- Adding multiple yaks without `yx ls` between them (Iron Law!)
- Showing markdown structure instead of actual `yx` commands
- Over-planning context before discovering blockers
- Nesting by "feels like subtask" instead of "blocks the parent"
- Exploring 3+ levels deep before adding context to parents
- Creating child yaks without agreeing scope first (scope creep!)

**If you catch yourself doing these, stop and restart with approach-first.**

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
| Tag yak | `yx tag yak name @needs-human` |
| List tags | `yx ls --tag @needs-human` |

## See Also

- **`mapping-flow.dot`** — Visual diagram of the mapping process
- **`/yx:prepare`** — Flesh out a yak with detailed specs and acceptance criteria
- **`/yx:work`** — Pick up ready yaks, implement in worktrees, merge
- **`../docs/yak-workflow.dot`** — High-level workflow showing when to use each skill
