We've added migration logic (migrate_done_to_state) that runs on every yx invocation. This is fine for now, but we don't want to accumulate these migrations forever.

Solution:
**Database-style migrations**: Track schema version, maintain ordered migration list, apply only needed ones

Considerations:
We're an event-based store (using git's refs/notes/yaks), so migrations can also be tracked as git commits 
