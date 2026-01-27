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

## Key Changes

1. Remove `YAKS_PATH` variable entirely
2. Use standard `GIT_DIR` for test isolation
3. Rewrite operations to use git plumbing:
   - `list_yaks()` - use `git ls-tree`
   - `add_yak()` - use `git mktree` + `git commit-tree`
   - `context_yak()` - read via `git show`, write via tree update
   - `sync_yaks()` - simplified (no extraction step)
   - `log_command()` - simplified (no filesystem snapshot)

## Benefits

- Single source of truth
- Simpler sync logic
- No duplication/consistency issues
- Still fast (git plumbing is efficient)
