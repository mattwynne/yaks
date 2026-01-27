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

**Format breakdown:**
- `-r` = recursive
- `-t` = show tree entries (directories) when recursing
- Output format: `040000 tree <hash><TAB>path`
- `040000` = git file mode for directory (octal)
- `grep '^040000 tree'` = filter to only directories
- `cut -f2` = extract path (field 2, tab-delimited)
- Handles spaces in names correctly

**No need for:**
- `state` files for discovery (still needed for todo/done status)
- `.yak` marker files
- Complex awk/sed parsing

## Mikado Method Progress

Applied the Mikado Method to discover dependencies by trying naive implementations and reverting when they break.

### Experiment 1: Replace find_all_yaks() directly

**Attempt:** Changed `find_all_yaks()` to use `git ls-tree -r -t` instead of filesystem `find`

**Result:** 6 test failures
- 2 in completions (filtering done/not-done)
- 4 in prune (removing done yaks)

**Discovery:** `find_all_yaks()` returns yak names ("Fix bug") but downstream code expects full paths ("$YAKS_PATH/Fix bug")

**Blockers identified:**
- `is_yak_done()` expects paths, needs to work with names
- Other functions expect paths

### Experiment 2: Make is_yak_done work with names

**Attempt:** Changed `is_yak_done()` to read from git:
```bash
is_yak_done() {
  local yak_name="$1"
  local state=$(git show "refs/notes/yaks:$yak_name/state" 2>/dev/null)
  [ "$state" = "done" ]
}
```

**Result:** Down to 4 test failures (completions tests now pass!)
- All 4 failures in prune command

**Discovery:** `prune_yaks()` also expects paths:
```bash
prune_yaks() {
  while IFS= read -r yak_path; do
    if is_yak_done "$yak_path"; then
      local yak_name="${yak_path#$YAKS_PATH/}"  # Tries to strip path prefix
      remove_yak "$yak_name"
    fi
  done < <(find_all_yaks)
}
```

**New blocker:** `prune_yaks()` needs to work with names instead of paths

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
3. Continue discovering and working through dependencies

## Environment Variables

**Remove:**
- `YAKS_PATH` - no more filesystem directory to point to

**Use standard git:**
- `GIT_DIR` - standard git environment variable for test isolation
- Production: unset, git auto-discovers from `$PWD`
- Tests: `GIT_DIR="$test_repo/.git"` for isolation

## Benefits

- Single source of truth
- Simpler sync logic (no extraction after merge)
- No duplication/consistency issues
- Still fast (git plumbing is efficient ~11ms)
- Cleaner architecture

## Trade-offs

- Can't do `cat .yaks/claim/context.md` (must use `yx context --show claim`)
- Slightly less "Unix-y" (everything through CLI)
- But: follows git's own model (working dir vs committed state)
