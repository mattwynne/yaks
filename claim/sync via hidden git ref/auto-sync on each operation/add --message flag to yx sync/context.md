Add a --message flag to `yx sync` to allow custom commit messages instead of the generic "Sync yaks".

Usage:
```bash
yx sync --message "Add implement login yak"
yx sync -m "Mark login yak as done"
```

This is a prerequisite for auto-sync, where each operation (add, done, rm, etc.) would call sync with a descriptive message like:
- "Add yak: implement login"
- "Mark done: implement login"
- "Remove yak: old task"
- "Move yak: old name -> new name"

Implementation:
- Update sync_yaks() to accept message parameter
- Default to "Sync yaks" if no message provided
- Use the message in the git commit-tree command (currently line ~297)
- Update case statement to handle --message/-m flag

Tests should verify:
- Custom messages appear in git log for refs/notes/yaks
- Default message still works when flag not provided
