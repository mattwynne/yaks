Rewrite log_command() to manipulate git trees directly without needing .yaks filesystem directory.

## Current Implementation

log_command() currently:
1. Uses GIT_WORK_TREE=$YAKS_PATH to point to .yaks
2. Stages all files from .yaks with git add
3. Writes tree
4. Creates commit on refs/notes/yaks

This requires .yaks to exist as a filesystem directory.

## New Implementation Needed

Should:
1. Read current tree from refs/notes/yaks (if exists)
2. Accept tree modifications as parameters (add/update/delete paths)
3. Use git mktree to create new tree
4. Create commit with git commit-tree
5. Update refs/notes/yaks

## Why This is Critical

ALL write operations (add, done, rm, move, context) call log_command().
Until log_command works without .yaks, none of the other write operations can be converted.

This is the critical blocker identified in Mikado Experiment 3.
