Auto-sync yak changes to origin after each operation (add, done, rm, etc.) so that each operation creates a new commit in refs/notes/yaks.

This would make the yak history more granular and allow better collaboration - team members would see individual operations rather than bulk "Sync yaks" commits.

Benefits:
- Each add/done/rm/move gets its own commit
- Better git history and understanding of what changed when
- More real-time collaboration
- Can see who did what operation

Implementation:
- Call `yx sync` automatically at the end of add_yak(), done_yak(), remove_yak(), move_yak(), prune_yaks()
- Make sync silent unless there's an error
- Handle offline gracefully (sync can fail silently if no origin)

Requires:
- Adding a --message flag to yx sync so each operation can create a descriptive commit message
