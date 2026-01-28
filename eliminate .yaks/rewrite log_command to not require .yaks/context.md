Rewrite log_command() to manipulate git trees directly without needing .yaks filesystem directory.

## Progress So Far

### ✅ Phase 1: Infrastructure (COMPLETE)

Added `--tree` parameter to log_command() that accepts pre-built tree hashes:

```bash
log_command --tree "$tree_hash" "commit message"
```

**Implementation:**
- Backward compatible: still reads from .yaks if --tree not provided
- Direct tree commit: bypasses filesystem entirely when using --tree
- All 109 tests pass (105 original + 4 new unit tests)

**Unit tests added:** spec/log_command_tree_mode.sh
- Commits tree to refs/notes/yaks
- Stores yak structure correctly
- Creates proper parent relationships
- Uses correct commit message

### ⬜ Phase 2: Migrate Callers (TODO)

Need to update write operations to build trees and use --tree mode:

1. **add_yak()** - create tree with state/context blobs, pass to log_command --tree
2. **context_yak()** - update tree with new context blob, pass to log_command --tree
3. **mark_yak_done()** - update tree with new state blob, pass to log_command --tree
4. **remove_yak()** - remove from tree, pass to log_command --tree
5. **move_yak()** - copy tree to new location, remove old, pass to log_command --tree

Each operation needs to:
- Read current tree from refs/notes/yaks (if exists)
- Create/modify blobs using git hash-object
- Build new tree using git mktree
- Call log_command --tree with new tree hash

### ⬜ Phase 3: Cleanup (TODO - see separate yak)

Once all callers migrated, simplify log_command() to only support --tree mode.
See: "eliminate .yaks/collapse log_command back to tree-only after migration"

## Current Architecture

Old (filesystem-based):
- add_yak creates directories and files in .yaks
- log_command stages those files and commits

New (git-based) - TODO:
- add_yak creates git blobs directly
- add_yak builds tree using git mktree
- add_yak calls log_command --tree with pre-built tree
- No filesystem operations required

## Why This is Critical

ALL write operations (add, done, rm, move, context) call log_command().
Until log_command works without .yaks, none of the other write operations can be converted.

This is the critical blocker identified in Mikado Experiment 3.

## Next Actions

1. Pick one write operation (suggest: add_yak as it's simplest)
2. Implement tree building for that operation
3. Use log_command --tree to commit
4. Run tests to verify
5. Repeat for remaining operations
