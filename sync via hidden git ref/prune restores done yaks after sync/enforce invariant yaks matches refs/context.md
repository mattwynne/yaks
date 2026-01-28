## Current (Broken) Strategy

1. Check if `.yaks` differs from `refs/notes/yaks` (has_local_changes)
2. If different, try to merge filesystem directories with `cp -r` 
3. Then merge git refs
4. Extract final result to `.yaks`

**Problems:**
- Git doesn't track empty directories, so `.yaks` can differ just from empty folders left after prune
- `cp -r` can't represent deletions - when remote deletes a yak folder and we overlay local on top, the deleted folder comes back
- Assumes `.yaks` might have uncommitted changes, but every operation calls `log_command()` which commits immediately
- When `has_local_changes=true`, calls buggy `merge_remote_into_local_yaks` which brings back pruned yaks

## Better Strategy

**Invariant:** `.yaks` is always just a working copy of `refs/notes/yaks`. It should NEVER have uncommitted changes.

**Sync algorithm:**
1. **Verify invariant** - Assert `.yaks` matches `refs/notes/yaks`. If not, that's a bug → error or auto-reset
2. **Merge at git ref level only** - Use git's merge capabilities (already in place: `merge_local_and_remote`)
3. **Extract final result** - `extract_yaks_to_working_dir` (already does clean `rm -rf` + `git archive`)

**This eliminates:**
- `has_local_changes` detection
- `merge_remote_into_local_yaks` function entirely  
- Empty directory problem (because we always do clean extractions from git)

**Benefits:**
- Simpler, more correct
- Git handles all the hard merging logic
- `.yaks` is always disposable/reconstructible

## Implementation

Delete lines 659-685 (detect_local_changes), 778-787 (merge_remote_into_local_yaks), and 797-809 in sync_yaks.

Replace with simple invariant check at start of sync:
```bash
sync_yaks() {
  check_git_setup || exit 1
  
  # Verify .yaks matches refs/notes/yaks
  # If not, error or auto-reset to refs/notes/yaks
  
  git fetch origin refs/notes/yaks:refs/remotes/origin/yaks 2>/dev/null || true
  
  local remote_ref=$(get_remote_ref)
  local local_ref=$(get_local_ref)
  
  merge_local_and_remote "$local_ref" "$remote_ref"
  
  if git rev-parse refs/notes/yaks >/dev/null 2>&1; then
    git push origin refs/notes/yaks:refs/notes/yaks 2>/dev/null || true
  fi
  
  extract_yaks_to_working_dir
  
  git update-ref -d refs/remotes/origin/yaks 2>/dev/null || true
}
```
