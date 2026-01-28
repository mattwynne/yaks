sync via hidden git ref/prune restores done yaks after sync/enforce invariant yaks matches refs/use GIT_PATH as primary variable

## Problem

Current architecture uses `YAKS_PATH` as the primary variable, but git commands operate on the current directory's repository. This causes issues when:
- Tests run `YAKS_PATH="$USER1/.yaks" yx add "test yak"` from a different directory
- Git commands update the wrong repository's `refs/notes/yaks`
- File operations go to the right place, but git operations don't

This violates the invariant that `.yaks` should always match `refs/notes/yaks`.

## Solution: GIT_PATH as Primary Variable

Make `GIT_PATH` the primary variable instead:

**New Architecture:**
- `GIT_PATH` = the git repository to operate on (defaults to current repo: `.`)
- `YAKS_PATH` = `$GIT_PATH/.yaks` (derived)

**Benefits:**
1. All git commands use `git -C "$GIT_PATH"` - no ambiguity
2. Clearer architecture - yaks are tied to a specific git repository
3. Fixes test issues where file and git operations target different repos
4. Maintains invariant naturally - both files and refs in same repo

**Implementation:**
```bash
GIT_PATH="${GIT_PATH:-.}"
YAKS_PATH="$GIT_PATH/.yaks"

# All git commands become:
git -C "$GIT_PATH" update-ref refs/notes/yaks "$new_commit"
git -C "$GIT_PATH" rev-parse refs/notes/yaks
# etc.
```

**Migration:**
- This is a breaking change for any existing usage that sets YAKS_PATH
- Since this is early development, acceptable to make the change now
- Update all git commands to use `git -C "$GIT_PATH"`
- Update tests to use `GIT_PATH` instead of `YAKS_PATH`

**Test Updates:**
```bash
# Old:
YAKS_PATH="$USER1/.yaks" yx add "test yak"

# New:
GIT_PATH="$USER1" yx add "test yak"
```
