Make mark_yak_done() update state via git instead of writing to filesystem.

## Current Implementation

- Writes to "$YAKS_PATH/$yak_name/state"
- Calls log_command to commit

## New Implementation

- Update tree with new state file content
- Call log_command() to commit the change

## Dependencies

Blocked by: rewrite log_command to not require .yaks
