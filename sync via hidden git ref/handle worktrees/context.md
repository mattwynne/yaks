Verify that `yx sync` works correctly when using git worktrees, which is a common workflow for working on multiple yaks simultaneously.

## How git worktrees work:
- Multiple worktrees share the same `.git/` directory (object database and refs)
- Each worktree has its own working directory (including separate `.yaks/` directories)

## What sync does:
1. Fetches `refs/notes/yaks` from origin (updates shared ref)
2. Extracts remote yaks to temp dir
3. Copies local `.yaks/` over the temp dir (merging)
4. Creates new commit on `refs/notes/yaks` (shared ref)
5. Pushes to origin
6. Extracts back to local `.yaks/`

## Potential issues to test:

**1. Stale .yaks/ directory**
- If you sync in worktree A, worktree B's `.yaks/` won't automatically update
- When you sync in B, it should fetch and merge correctly
- Test: Add yak in A, sync in A, sync in B, verify B gets the yak

**2. Race condition / concurrent edits**
- If you add different yaks in two worktrees without syncing, then sync both:
  - Worktree A: adds "foo", syncs (pushes foo)
  - Worktree B: adds "bar", syncs (should fetch foo, merge with bar, push both)
- The merge logic (`cp -r` after extracting remote) should preserve both
- Test: Add different yaks in A and B, sync both, verify both yaks end up in origin

**3. Conflicting edits**
- What happens if both worktrees modify the same yak (e.g., both mark it done)?
- Last write wins with current implementation
- Should document this behavior

## Test to add:
Create spec/sync_worktrees.sh that:
1. Sets up main repo with origin
2. Creates two worktrees from main
3. Adds different yaks in each worktree
4. Syncs both
5. Verifies both yaks appear in origin and in each worktree after sync

This will verify the multi-agent/multi-worktree workflow that's a key use case for yaks.
