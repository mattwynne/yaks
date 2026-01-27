# Mikado Method

**Discovering refactoring dependencies through experiments**

## Purpose

Use the Mikado Method when you need to make a large refactoring but don't know all the dependencies upfront. Instead of planning everything, discover dependencies by trying changes and seeing what breaks.

## Core Principle

**Try, fail, learn, revert, repeat**

1. Try a naive implementation of your goal
2. Run tests to see what breaks
3. Identify the blockers (dependencies you need first)
4. Revert your changes
5. Work on a blocker (which might reveal more blockers)
6. Once blockers are done, try your original change again

## The Workflow

### 1. Make the Naive Change

Implement what you think the final code should look like, ignoring dependencies.

```bash
# Example: Replace filesystem with git
find_all_yaks() {
  git ls-tree -r -t refs/notes/yaks | grep '^040000 tree' | cut -f2
}
```

### 2. Run Tests and Record Failures

```bash
shellspec
```

Count and categorize the failures. Record:
- How many tests failed
- Which test files failed
- What the error messages indicate

### 3. Analyze the Failures

Look for patterns in the failures:
- Do they all touch the same function?
- Is there a common error message?
- What assumptions did your change break?

### 4. Identify Blockers

From the failures, identify what needs to change first:

```
Experiment 1: Replace find_all_yaks()
Result: 6 failures
- 2 in completions (filtering done/not-done)
- 4 in prune (removing done yaks)

Discovery: find_all_yaks() returns names but downstream
code expects paths.

Blockers:
- is_yak_done() expects paths, needs to work with names
- prune_yaks() expects paths, needs to work with names
```

### 5. Build the Mikado Graph

Draw the dependency tree as you discover it:

```
eliminate .yaks (BLOCKED - tried, 6 failures)
├─ make functions use yak names not paths (not yet attempted)
└─ make is_yak_done work with git (BLOCKED - tried, 4 failures)
   └─ make prune_yaks work with yak names (LEAF - should try next)
```

**Graph notation:**
- BLOCKED - tried this change, tests failed, reverted
- LEAF - no known blockers, ready to implement
- not yet attempted - discovered as blocker but not tried

### 6. Revert Your Changes

```bash
git reset --hard HEAD
```

Important: Always revert before working on blockers. Keep the tree clean.

### 7. Work on a Leaf Node

Find a blocker with no dependencies (leaf node) and implement it:

```bash
# Create yak for the leaf node work
yx add "eliminate .yaks/make is_yak_done work with git/make prune_yaks work with yak names"

# Work on it in isolation
# ... implement and test ...
```

The leaf node change might:
- Pass all tests (success! merge it)
- Fail tests (discover more blockers, update graph)

### 8. Try the Parent Again

After merging leaf nodes, try their parent change again:

```bash
# Reapply "make is_yak_done work with git"
# Run tests
# Fewer failures? Update graph with progress
```

### 9. Repeat Until Goal Achieved

Continue discovering and working through dependencies until your original goal passes all tests.

## Documentation Pattern

### In Yak Context

Document your experiments in the parent yak's context file:

```markdown
## Mikado Method Progress

### Experiment 1: Replace find_all_yaks() directly

**Attempt:** Changed find_all_yaks() to use git ls-tree

**Result:** 6 test failures
- 2 in completions (filtering done/not-done)
- 4 in prune (removing done yaks)

**Discovery:** find_all_yaks() returns names but downstream
expects paths

**Blockers identified:**
- is_yak_done() expects paths, needs to work with names
- Other functions expect paths

### Experiment 2: Make is_yak_done work with names

**Attempt:** Changed is_yak_done() to read from git
**Result:** Down to 4 failures (completions now pass!)
**Discovery:** prune_yaks() also expects paths
**New blocker:** prune_yaks() needs to work with names

## Current Mikado Graph

```
eliminate .yaks (BLOCKED - tried, 6 failures)
├─ make functions use yak names not paths (not yet attempted)
└─ make is_yak_done work with git (BLOCKED - tried, 4 failures)
   └─ make prune_yaks work with yak names (LEAF - should try next)
```

## Next Steps

1. Try "make prune_yaks work with yak names" (leaf node)
2. If it succeeds, try "make is_yak_done work with git" again
3. Continue discovering dependencies
```

### In Leaf Node Commits

When committing leaf node changes, note they're part of Mikado:

```
Make prune_yaks work with yak names

Change prune_yaks() to work when find_all_yaks()
returns names instead of paths.

This is a Mikado leaf node - tests will pass once
parent changes (is_yak_done accepting names) are
merged.
```

## Key Insights

### Tests Are Your Guide

The test suite tells you:
- What's broken (failure messages)
- What depends on what (failure patterns)
- When you're done (all green)

Trust the tests to reveal dependencies.

### Revert Fearlessly

Don't try to fix failures in place. Revert and work on blockers in isolation. This keeps each change small and focused.

### Update the Graph

As you discover new blockers, add them to the graph. As you complete nodes, mark them done. The graph is your roadmap.

### Leaf Nodes May Still Fail

When you implement a leaf node, it might reveal more dependencies and fail tests. That's fine! Update the graph and continue.

### Some Leaf Nodes Can Merge With Failing Tests

If a leaf node:
- Makes logical sense in isolation
- Will only work once parent changes merge
- Has failing tests that will pass after parent merges

You can merge it with failing tests. Document this clearly in the commit message.

**However:** If tests must be green before merge (project policy), then work on multiple nodes together in one worktree until tests pass.

## Example: eliminate .yaks Refactoring

### Goal
Replace `.yaks/` filesystem directory with git plumbing commands.

### Experiment 1
```bash
# Changed find_all_yaks() to use git ls-tree
# Result: 6 failures
# Discovered: downstream expects paths, not names
```

### Experiment 2
```bash
# Changed is_yak_done() to work with names
# Result: 4 failures (2 tests now pass!)
# Discovered: prune_yaks() also needs update
```

### Experiment 3
```bash
# Changed prune_yaks() to work with names
# Result: Should reduce failures further
# If still failing: more blockers to discover
```

### The Graph Evolution

After Experiment 1:
```
eliminate .yaks (BLOCKED)
└─ is_yak_done needs names (LEAF)
```

After Experiment 2:
```
eliminate .yaks (BLOCKED)
└─ is_yak_done needs names (BLOCKED)
   └─ prune_yaks needs names (LEAF)
```

## When NOT to Use Mikado

- Small, well-understood changes
- You already know all dependencies
- No tests to guide you
- Single-function changes

Mikado shines for large refactorings with unknown dependencies.

## Integration with Other Skills

- **incremental-tdd**: Use TDD within each Mikado leaf node
- **yak-worktree-workflow**: Each Mikado experiment or leaf node gets its own worktree
- **discovery-tree-workflow**: Create sub-yaks as you discover blockers

## Tips

### Name Your Experiments

Number them (Experiment 1, 2, 3...) so you can refer back.

### Commit the Graph

Keep the Mikado graph in the parent yak's context file and update it as you learn.

### Work One Leaf at a Time

Don't try to fix multiple blockers simultaneously. Stay focused.

### Celebrate Progress

"Down to 4 failures" is progress! Even discovering what *won't* work is valuable.

### Trust the Process

It feels slow at first (try, revert, try again) but it prevents you from getting lost in a huge half-working change.

## References

- Mikado Method book: https://mikadomethod.info/
- Core idea: Make the change you want, let the tests tell you what needs to change first
