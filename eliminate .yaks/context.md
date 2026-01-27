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

## Environment Variables

**Remove:**
- `YAKS_PATH` - no more filesystem directory to point to

**Use standard git:**
- `GIT_DIR` - standard git environment variable for test isolation
- Production: unset, git auto-discovers from `$PWD`
- Tests: `GIT_DIR="$test_repo/.git"` for isolation

## Key Changes

1. **Remove `YAKS_PATH` entirely**
2. **Rewrite operations to use git plumbing:**
   - `list_yaks()` - use `git ls-tree -r -t` to list directories
   - `add_yak()` - use `git mktree` + `git commit-tree`
   - `context_yak()` - read via `git show`, write via tree update
   - `is_yak_done()` - read `state` file via `git show`
   - `sync_yaks()` - simplified (no extraction step!)
   - `log_command()` - simplified (no filesystem snapshot needed)

3. **Remove filesystem operations:**
   - No more `extract_yaks_to_working_dir()`
   - No more `mkdir -p "$YAKS_PATH/$yak_name"`
   - No more `echo "todo" > "$YAKS_PATH/$yak_name/state"`

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
