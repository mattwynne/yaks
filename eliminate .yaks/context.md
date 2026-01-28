Eliminate the `.yaks/` filesystem directory and use git plumbing commands directly.

## CRITICAL DISCOVERY: Context Command Blocking Everything! 🚨

After making add_yak write to git, discovered workflow is BROKEN:
- ✅ add_yak creates yaks in git
- ❌ context command expects .yaks filesystem
- ❌ Can't document work, add nested yaks, or collaborate

**IMMEDIATE ACTION REQUIRED: Fix context_yak FIRST**

## Updated Mikado Graph (After Multiple Discoveries)

```
eliminate .yaks
│
└─ WRITE operations
   │
   ├─ make add_yak ✅ DONE (but revealed blocker)
   │
   ├─ make context_yak (CRITICAL BLOCKER! 🔥)
   │  Status: Must do IMMEDIATELY
   │  Why: Workflow broken without it
   │  Blocks: Everything else
   │
   ├─ make remove_yak (BLOCKED by context)
   │  Can't document work without context
   │
   ├─ make mark_yak_done (BLOCKED by context + 4 others)
   │  Multiple blockers discovered
   │
   └─ make move_yak (BLOCKED by context)
```

## Mikado Lessons Learned (Updated)

1. ✅ Always check dependencies before claiming "leaf node"
2. ✅ Revert when blockers discovered
3. ✅ Update map with new information
4. ✅ **Integration matters more than individual functions**
5. ✅ **Workflow breakage is highest priority**

## Next Action

**Work on context_yak immediately - it's blocking everything**
