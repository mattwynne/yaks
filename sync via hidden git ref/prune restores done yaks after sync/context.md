## Bug: Done yaks come back after prune + sync

### Reproduction Steps
```bash
yx sync
yx ls         # Shows done yaks
yx prune      # Removes done yaks
yx ls         # Confirms they're gone
yx sync       # BUG: Done yaks come back!
yx ls         # Done yaks are back
```

### Investigation (branch: fix-prune-sync-bug)

**What I tried:**
1. Added `is_ancestor()` helper to detect fast-forward scenarios
2. Modified `merge_local_and_remote()` to skip merge when local is ahead of remote
3. Added test case that reproduces the scenario and PASSES

**Test vs Reality:**
- Test: `spec/sync.sh` - "does not restore pruned yaks after sync" PASSES ✓
- Reality: Running the commands in the actual repo still shows the bug ✗

**Key Insight:**
The test passes because it's in a clean two-user scenario, but the real repo has a more complex git ref history. The fast-forward detection logic I added may not be sufficient.

**Current State:**
- Branch `fix-prune-sync-bug` has the attempted fix
- All 107 tests pass
- Bug still reproduces in real repository
- Need to investigate why the test scenario differs from reality

**Next Steps:**
1. Debug why test passes but real repo fails
2. Check the actual git ref structure in the real repo
3. May need to examine the `extract_yaks_to_working_dir()` function - it always extracts from refs/notes/yaks regardless of merge outcome
4. Consider if the issue is in how prune updates the ref vs how sync merges

**Hypothesis:**
The problem might be that `extract_yaks_to_working_dir()` at the end of sync (line 797) always extracts from `refs/notes/yaks`, but after the merge logic, the ref might contain the merged result that brought back the done yaks.
