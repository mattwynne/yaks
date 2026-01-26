# Cleanup and Improvements for sync-done-undo Implementation

Based on code review of commit 0e509b3, the following improvements should be made:

## Performance

1. **Optimize sync_yaks() diff check (line 356-370)**
   - Currently does recursive `diff -r` of entire `.yaks` directory on every sync
   - Replace with git hash comparison for better performance with large yak collections

## Error Handling

2. **Add error handling to log_command()**
   - Ensure it doesn't silently fail
   - Consider transaction pattern: log before state change or rollback on failure

3. **Add rollback mechanism to sync merge logic**
   - If git operations fail during merge (lines 396-403), user could end up with partial state
   - Need error handling and rollback capability

## Observability

4. **Improve prune_yaks() logging (line 247)**
   - Current: just logs "prune" with no details
   - Should: capture which yaks were pruned in commit message for audit trail

5. **Add yx log command**
   - View operation history from refs/notes/yaks
   - Useful for debugging and understanding what happened

6. **Add --dry-run flag to sync**
   - Preview what would change before syncing
   - Helps users understand conflicts and merges

## Documentation

7. **Document timestamp-based conflict resolution**
   - Add comments explaining last-write-wins strategy
   - Warn about clock skew between machines
   - Consider using logical clock or requiring time sync

## Testing

8. **Add integration tests for refs/notes/yaks**
   - Verify push/fetch work correctly with remote repositories
   - Test with actual git remotes, not just local operations
