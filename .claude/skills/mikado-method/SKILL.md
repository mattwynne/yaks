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

### 2. Run Tests and Record Failures

Run the test suite. Count and categorize the failures:
- How many tests failed
- Which test files failed
- What the error messages indicate

### 3. Analyze and Identify Blockers

Look for patterns in the failures:
- Do they all touch the same module?
- Is there a common error message?
- What assumptions did your change break?

From the failures, identify what needs to change first.

### 4. Build the Mikado Graph

Draw the dependency tree as you discover it:

```
goal (BLOCKED - tried, 6 failures)
├─ prerequisite A (not yet attempted)
└─ prerequisite B (BLOCKED - tried, 4 failures)
   └─ prerequisite C (LEAF - should try next)
```

**Graph notation:**
- BLOCKED - tried this change, tests failed, reverted
- LEAF - no known blockers, ready to implement
- not yet attempted - discovered as blocker but not tried

### 5. Revert Your Changes

Always revert before working on blockers. Keep the tree clean.

### 6. Work on a Leaf Node

Find a blocker with no dependencies (leaf node) and implement it.
Use yaks to track the work:

```bash
yx add "goal/prerequisite B/prerequisite C"
```

The leaf node change might:
- Pass all tests (merge it)
- Fail tests (discover more blockers, update graph)

### 7. Try the Parent Again

After merging leaf nodes, retry the parent change. Fewer failures
means progress. Update the graph.

### 8. Repeat Until Goal Achieved

Continue discovering and working through dependencies until your
original goal passes all tests.

## Documentation Pattern

Store Mikado progress in the parent yak's context:

```markdown
## Mikado Method Progress

### Experiment 1: [describe the naive change]
**Result:** N test failures
**Discovery:** [what you learned]
**Blockers identified:** [list]

### Current Mikado Graph
[paste the graph]

### Next Steps
1. Try [leaf node]
2. If it succeeds, retry [parent]
```

## Key Insights

**Tests are your guide.** The test suite tells you what's broken,
what depends on what, and when you're done.

**Revert fearlessly.** Don't try to fix failures in place. Revert
and work on blockers in isolation.

**Leaf nodes may reveal more.** Implementing a leaf might reveal
new dependencies. Update the graph and continue.

**Progress is incremental.** "Down to 4 failures from 6" is real
progress. Each experiment teaches you something.

## When NOT to Use

- Small, well-understood changes
- You already know all dependencies
- No tests to guide you
- Single-function changes

Mikado shines for large refactorings with unknown dependencies.

## Integration with Other Skills

- **incremental-tdd**: Use TDD within each Mikado leaf node
- **yak-worktree-workflow**: Each experiment or leaf node gets its own worktree
- **structuring-yak-dependencies**: Model discovered blockers as child yaks

## References

- Mikado Method book: https://mikadomethod.info/
- Core idea: Make the change you want, let the tests tell you what needs to change first
