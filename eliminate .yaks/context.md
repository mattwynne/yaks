eliminate .yaks

Eliminate the `.yaks/` filesystem directory and use git plumbing commands directly.

## Architecture Change

**Current:**
- `.yaks/` filesystem holds working copy
- `refs/notes/yaks` mirrors filesystem
- Duplication between filesystem and git

**New:**
- `refs/notes/yaks` is single source of truth
- Use `git ls-tree`, `git show`, `git mktree`, `git commit-tree` directly
- No extraction/duplication needed

## Key Discovery: List Yaks by Listing Trees

Every directory in the git tree IS a yak. We can list all yaks simply:

```bash
git ls-tree -r -t refs/notes/yaks | grep '^040000 tree' | cut -f2
```

## Mikado Method Progress

### Experiment 1: Replace find_all_yaks() directly

**Attempt:** Changed `find_all_yaks()` to use `git ls-tree -r -t` instead of filesystem `find`

**Result:** 6 test failures
- 2 in completions (filtering done/not-done)
- 4 in prune (removing done yaks)

**Discovery:** Downstream code expects paths but git returns names

**Blockers identified:**
- is_yak_done() expects paths
- prune_yaks() expects paths  
- completions() expects paths

**Resolution:** ✅ All blockers fixed and merged!

### Experiment 2: Fix the blockers

**Changes made:**
- ✅ is_yak_done() now reads from git and accepts names
- ✅ prune_yaks() converts paths to names
- ✅ completions() reads from git and works with names  
- ✅ find_all_yaks() uses git ls-tree

**Result:** All 105 tests pass!

### Experiment 3: Remove filesystem writes in add_yak()

**Attempt:** Commented out `mkdir`/`echo`/`touch` in `add_yak_single()`

**Result:** 68 failures out of 105 tests

**Discovery:** Many functions still depend on .yaks existing:
- log_command() requires .yaks to stage changes
- context_yak() reads/writes to .yaks files
- mark_yak_done() writes state files to .yaks
- remove_yak() removes .yaks directories
- move_yak() moves .yaks directories
- list_yaks() likely reads from .yaks
- sync functions extract/copy .yaks

**New blockers identified:**
- Need to rewrite log_command() to work without .yaks
- Need git-based context read/write
- Need git-based state updates
- Need to handle all write operations via git trees

## Current Mikado Graph

```
eliminate .yaks (BLOCKED - 68 test failures when filesystem writes removed)
├─ make find_all_yaks use git (DONE ✅)
├─ make is_yak_done work with git (DONE ✅)
├─ make prune_yaks work with names (DONE ✅)
├─ make completions work with git (DONE ✅)
└─ make write operations use git directly (BLOCKED - not attempted)
   ├─ rewrite log_command to not require .yaks (LEAF)
   ├─ make context_yak read/write via git (LEAF)
   ├─ make mark_yak_done update git tree (LEAF)
   ├─ make remove_yak update git tree (LEAF)
   ├─ make move_yak update git tree (LEAF)
   └─ remove extract_yaks_to_working_dir from sync (LEAF)
```

## Next Steps

The core blocker is **log_command()** - it currently:
1. Uses GIT_WORK_TREE=$YAKS_PATH  
2. Stages .yaks contents
3. Commits to refs/notes/yaks

Need to rewrite it to:
1. Read current tree from refs/notes/yaks
2. Modify tree using git mktree  
3. Create commit with new tree

Once log_command works without .yaks, we can update other write operations.

## Benefits

- Single source of truth
- Simpler sync logic (no extraction)
- No duplication/consistency issues  
- Still fast (git plumbing ~11ms)

## Trade-offs

- Can't `cat .yaks/claim/context.md`
- Must use CLI for all operations
- Follows git's model (working dir vs committed state)
