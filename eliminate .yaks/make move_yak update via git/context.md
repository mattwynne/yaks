Make move_yak() rename via git tree instead of filesystem.

## Current Implementation

- mv "$YAKS_PATH/$old_name" "$YAKS_PATH/$new_name"
- Calls log_command to commit

## New Implementation

- Copy old tree to new location in git tree
- Remove old location from git tree
- Call log_command() to commit the move

## Dependencies

Blocked by: rewrite log_command to not require .yaks
