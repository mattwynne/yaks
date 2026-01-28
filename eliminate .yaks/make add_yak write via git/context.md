Make add_yak_single() and add_yak_interactive() write directly to git instead of creating filesystem directories.

## Current Implementation

Currently creates:
- mkdir -p "$YAKS_PATH/$yak_name"
- echo "todo" > "$YAKS_PATH/$yak_name/state"
- touch "$YAKS_PATH/$yak_name/context.md"
- Calls log_command to commit

## New Implementation

Should:
- Create tree objects for new yak (with state and context.md)
- Call updated log_command() to commit the new tree
- No filesystem operations

## Dependencies

Blocked by: rewrite log_command to not require .yaks
