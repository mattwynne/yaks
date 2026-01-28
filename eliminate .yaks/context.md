Eliminate the `.yaks/` filesystem directory and use git plumbing commands directly.

## Current Status

**Phase 1: Infrastructure - COMPLETE ✅**
- ✅ log_command --tree mode merged to main

**Phase 2: WRITE Operations - IN PROGRESS ⬜**

## Updated Mikado Graph (After mark_yak_done Attempt)

```
eliminate .yaks
│
├─ READ operations ✅ COMPLETE
│
└─ WRITE operations (discovering dependencies...)
   │
   ├─ make add_yak write via git ✅ DONE
   │
   ├─ make remove_yak update via git (LEAF - READY)
   │  Status: Not started
   │  Complexity: Low - just delete tree entries
   │
   ├─ make context_yak read/write via git (LEAF - READY)
   │  Status: Not started  
   │  Complexity: Low - just update context blob
   │
   ├─ make mark_yak_done update via git (BLOCKED ❌)
   │  Status: Attempted, reverted
   │  Blockers discovered:
   │    - has_incomplete_children() needs git
   │    - mark_yak_done_recursively() needs git
   │    - find_yak fuzzy matching broken with nested paths
   │  Come back after simpler operations work
   │
   ├─ make move_yak update via git (LEAF - READY?)
   │  Status: Not started
   │  Complexity: Medium - tree manipulation + path changes
   │
   └─ rewrite log_command (PARTIALLY COMPLETE)
      ✅ Phase 1: --tree infrastructure
      ⬜ Phase 2: Migrate all callers
      ⬜ Phase 3: Simplify to tree-only
```

## Next Actions (Mikado Method)

Pick a TRUE leaf node - one with no hidden dependencies:

**Option 1: remove_yak (RECOMMENDED)**
- Simplest operation
- Just delete entries from tree
- No validation logic needed
- Tests are straightforward

**Option 2: context_yak**  
- Also simple - just update one blob
- Preserve existing tree structure
- Two modes: read (show) and write (edit)

**DO NOT work on:**
- mark_yak_done (has blockers)
- move_yak (until we understand tree manipulation better)

## Mikado Lessons Learned

1. ✅ Always check dependencies before claiming "leaf node"
2. ✅ Revert when blockers discovered
3. ✅ Update map with new information
4. ✅ Keep useful artifacts (unit tests) for future work

## Benefits

- Single source of truth in git
- No filesystem duplication  
- Simpler sync logic
- Faster operations
- Better for concurrent access
