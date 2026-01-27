Make remove_yak() delete from git tree instead of filesystem.

## Current Implementation

- rm -rf "$YAKS_PATH/$yak_name"
- Calls log_command to commit

## New Implementation

- Remove yak from git tree
- Call log_command() to commit the deletion

## Dependencies

Blocked by: rewrite log_command to not require .yaks
