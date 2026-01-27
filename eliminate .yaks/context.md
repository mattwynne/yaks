eliminate .yaks

Eliminate the `.yaks/` filesystem directory and use git plumbing commands directly.

## Current Status

**Phase 1: READ Operations - COMPLETE ✅**
- ✅ find_all_yaks() reads from git using ls-tree
- ✅ is_yak_done() reads state from git
- ✅ prune_yaks() works with yak names
- ✅ completions() reads from git
- ✅ All 105 tests pass with these changes

**Phase 2: WRITE Operations - IN PROGRESS ⬜**
- ⬜ log_command() - still requires .yaks filesystem (CRITICAL BLOCKER)
- ⬜ add_yak() - writes to .yaks filesystem
- ⬜ context_yak() - writes to .yaks filesystem
- ⬜ mark_yak_done() - writes to .yaks filesystem
- ⬜ remove_yak() - writes to .yaks filesystem
- ⬜ move_yak() - writes to .yaks filesystem

## Mikado Method Progress

### Experiment 1: Replace find_all_yaks() directly (Initial)
**Result:** 6 test failures → Fixed all blockers → 0 failures ✅

### Experiment 2: Fix read operation blockers (Completed)
**Result:** All 105 tests pass ✅

### Experiment 3: Remove filesystem writes in add_yak() (Discovery)
**Attempt:** Commented out mkdir/echo/touch in add_yak_single()
**Result:** 68 failures out of 105 tests (65% failure rate)

**Critical Discovery:** log_command() is the central blocker. It currently:
1. Uses GIT_WORK_TREE=$YAKS_PATH to stage .yaks contents
2. Stages all files with git add
3. Writes tree and commits to refs/notes/yaks

Every write operation (add, done, rm, move, context) calls log_command(), so they all fail when .yaks doesn't exist.

## Current Mikado Graph

```
eliminate .yaks (BLOCKED - 68 failures when .yaks removed)
│
├─ READ operations (COMPLETE ✅)
│  ├─ find_all_yaks use git ✅
│  ├─ is_yak_done work with git ✅
│  ├─ prune_yaks work with names ✅
│  └─ completions work with git ✅
│
└─ WRITE operations (BLOCKED - 68 failures)
   │
   ├─ rewrite log_command() (LEAF - CRITICAL!)
   │  Status: Not started
   │  Priority: HIGH - blocks all other write operations
   │  Complexity: High - need to manipulate git trees directly
   │
   ├─ make add_yak write via git
   │  Status: Not started
   │  Blocked by: log_command rewrite
   │
   ├─ make context_yak read/write via git
   │  Status: Not started
   │  Blocked by: log_command rewrite
   │
   ├─ make mark_yak_done update via git
   │  Status: Not started
   │  Blocked by: log_command rewrite
   │
   ├─ make remove_yak update via git
   │  Status: Not started
   │  Blocked by: log_command rewrite
   │
   └─ make move_yak update via git
      Status: Not started
      Blocked by: log_command rewrite
```

## Next Action (Per Mikado Method)

**Work on the LEAF node:** "rewrite log_command to not require .yaks"

This is the critical path blocker. Once log_command works without .yaks:
1. Update add_yak to use new log_command
2. Update context_yak to use new log_command
3. Update mark_yak_done to use new log_command
4. Update remove_yak to use new log_command
5. Update move_yak to use new log_command
6. Re-run Experiment 3 to discover any remaining blockers

## Technical Approach for log_command Rewrite

Need to change from:
```bash
# Current: Uses filesystem
GIT_WORK_TREE="$YAKS_PATH" git add .
tree=$(git write-tree)
```

To:
```bash
# New: Direct tree manipulation
# 1. Read current tree from refs/notes/yaks
# 2. Accept tree modifications as parameters (paths to add/update/delete)
# 3. Use git mktree to create new tree
# 4. Create commit with git commit-tree
# 5. Update refs/notes/yaks
```

This requires learning git tree manipulation plumbing commands.

## Benefits

- Single source of truth in git
- No filesystem duplication
- Simpler sync logic (no extraction needed)
- Faster operations (no file I/O)
- Better for concurrent access

## Trade-offs

- Can't use standard Unix tools on yak data (cat, grep, etc.)
- Must use CLI for all operations
- Slightly more complex implementation
- Follows git's model (committed state, no working dir)

## Environment Variables

**Current:**
- YAKS_PATH - points to .yaks directory

**Future:**
- Remove YAKS_PATH entirely
- Use standard GIT_DIR for test isolation

## Test Strategy

After each write operation is converted:
1. Run full test suite
2. Count failures
3. Analyze new blockers
4. Update Mikado graph
5. Work on next leaf node

Keep iterating until all 105 tests pass without .yaks existing.
