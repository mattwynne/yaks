## ✅ RESOLVED: No Refactor Needed

**Outcome:** The hierarchical sorting issue was solved using an enhanced sort-based approach instead of a full recursive refactor.

### What Was The Problem?
The test "keeps hierarchy when child is done" (spec/features/list.sh:66) was failing because:
- When a parent had done children and a newer sibling appeared later
- The sort wasn't maintaining the parent-child grouping correctly
- Children weren't appearing immediately after their parent

### Solution: Enhanced Sort Key Strategy
Instead of refactoring to a recursive display function, we enhanced the sort key to encode hierarchy:

**Format:** `parent_key/depth/priority/mtime/base_name`

Where:
- `parent_key`: Parent path with `/` replaced by `~` (higher ASCII, sorts after `/`)
- `depth`: Nesting level (0 for root, 1 for first level, etc.)
- `priority`: 0 for done, 1 for not-done (done items sort first within siblings)
- `mtime`: Modification time for chronological ordering
- `base_name`: The yak's leaf name

### Why This Works
- The parent key groups all children with their parent
- Depth ensures proper level separation
- Priority and mtime sort siblings correctly
- Single sort pass handles all cases

### Result
✅ All 17 list tests passing, including the problematic hierarchical case
✅ No recursive function needed
✅ Clean, maintainable solution
✅ Handles all edge cases:
  - Multiple nesting levels
  - Done/not-done children
  - Mixed states at different levels
  - Chronological ordering preserved

**Lesson:** Sometimes a better data structure (sort key) is simpler than a different algorithm (recursion).
