When auto-sync is enabled, handle offline scenarios gracefully by using the local refs/notes/yaks as the offline log, then push accumulated commits when back online.

## Better approach: Use local refs/notes/yaks

Instead of maintaining a separate log file, use git's native capabilities:

**When operation happens (online or offline):**
1. Perform the operation (add/done/rm/etc)
2. Create commit on local refs/notes/yaks with descriptive message
3. Try to push to origin
4. If push fails (offline):
   - Local commits stay in refs/notes/yaks
   - Continue silently (don't error to user)
5. If push succeeds:
   - Local and remote are in sync

**When coming back online:**
- Next sync automatically pushes all accumulated local commits
- Git handles the history/timestamps/messages natively
- No special replay logic needed!

## Benefits:
- No separate log file to maintain
- Git already tracks commit history and timestamps
- Push naturally batches multiple commits
- Handles merge conflicts with standard git mechanisms
- Can inspect offline commits with `git log refs/notes/yaks`

## Implementation:
- sync_yaks() already creates local commits
- Just make push failure non-fatal (it already does: `|| true`)
- When back online, push will send all accumulated commits
- Fetch before creating new commits to merge remote changes

## Edge cases to handle:

**Diverged histories:**
- If someone else pushed while you were offline
- Fetch will get their changes
- Need to merge their tree with yours before committing
- Current code does this: extracts remote, copies local over it

**Detection of offline:**
- `git push` returns non-zero when it fails
- Already silenced with `2>/dev/null || true`
- Could add logging to stderr if desired

**Testing:**
- Mock git push to fail (simulate offline)
- Make multiple operations while "offline"
- Mock git push to succeed (simulate back online)
- Verify all commits pushed in batch
