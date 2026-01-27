Make context_yak() read and write context.md via git instead of filesystem.

## Current Implementation

- show_yak_context(): reads from "$YAKS_PATH/$yak_name/context.md"
- edit_context_yak(): writes to "$YAKS_PATH/$yak_name/context.md" via $EDITOR or stdin

## New Implementation

- show: Use git show "refs/notes/yaks:$yak_name/context.md"
- edit: Update tree with new context.md content, call log_command()

## Dependencies

Blocked by: rewrite log_command to not require .yaks
