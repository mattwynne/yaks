Make mark_yak_done() update state via git instead of writing to filesystem.

## Mikado Discovery: NOT A LEAF NODE ❌

Attempted conversion but discovered hidden blockers:

**Blockers Found:**
1. `has_incomplete_children()` - expects filesystem, needs git version
2. `mark_yak_done_recursively()` - expects filesystem paths  
3. Fuzzy matching (`find_yak`) - ambiguity with "parent" vs "parent/child"
4. List display issues with hybrid git/filesystem

**Artifacts Created:**
- `update_tree_with_state()` helper function (4 unit tests pass)
- spec/update_tree_with_state.sh (keep for future use)

## Mikado Method: Come Back Later

This yak has too many dependencies. Work on simpler leaf nodes first:
- remove_yak (simpler - just delete from tree)
- context_yak (simpler - just update one blob)

Mark this yak as BLOCKED until those are done.

## Dependencies

- Blocked by: has_incomplete_children needs git
- Blocked by: find_yak fuzzy matching with nested paths
- Blocked by: mark_yak_done_recursively needs git
