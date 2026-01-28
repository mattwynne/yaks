Make remove_yak() delete from git tree instead of filesystem.

## Current Implementation

```bash
remove_yak() {
  local yak_name="$*"
  local resolved_name
  resolved_name=$(require_yak "$yak_name") || exit 1
  local yak_path="$YAKS_PATH/$resolved_name"
  rm -rf "$yak_path"
  log_command "rm $resolved_name"
}
```

## New Implementation

Should:
- Read current tree from refs/notes/yaks
- Remove the yak entry (and all nested children if applicable)
- Build new tree without removed entries
- Call log_command --tree with new tree

## Why This is a Leaf Node

✅ No validation logic needed
✅ No children checking (unlike done_yak)
✅ Just tree manipulation
✅ Clear test expectations

## Test Strategy

Existing tests in spec/rm.sh:
- removes a yak by name
- shows error when yak not found
- handles removing the only yak
- removes multi-word yak names without quotes
- removes a nested yak

## Implementation Plan

1. Create helper: `remove_from_tree(yak_name)`
2. Update remove_yak() to use it
3. Call log_command --tree
4. Run tests
5. Fix any issues
