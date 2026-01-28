Add a "wip" (work in progress) state for yaks to indicate work that has been started but not yet completed.

## Current States
- `todo` - not started
- `done` - completed

## Proposed Addition
- `wip` - actively being worked on

## Benefits
- Better visibility into what's currently being worked on
- Helps with team coordination (multiple agents/developers)
- Clearer status for partially completed work

## Implementation Considerations
- State file values: "todo", "wip", "done"
- List output: different styling/color for wip
- Filtering: `yx ls --only wip`
- Completions: filter appropriately for different commands
- Migration: existing yaks default to "todo"

## Related Commands to Update
- `yx done` - move from wip to done
- `yx list` - show wip with distinct styling
- `yx wip` (new command) - move from todo to wip
- Completions - filter for appropriate states
