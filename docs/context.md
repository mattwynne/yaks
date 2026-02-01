# Documentation Goal

Add detailed feature documentation alongside the acceptance tests to help both humans and AI agents understand:
- What each feature does
- Why design decisions were made
- Edge cases and constraints
- Implementation details

## Approach

Co-locate markdown documentation with test files:
- Pattern: `spec/features/done.md` lives alongside `spec/features/done.sh`
- Keeps tests clean and focused on behavior
- Makes documentation easy to discover
- Separates WHAT (tests) from WHY (docs)

## Progress Checklist

Core user-facing commands (all commands from `yx --help`):
- [x] add.md - Completed
- [ ] list.md - Todo
- [x] done.md - Completed (command name is "finish", alias "done")
- [ ] rm.md - Todo (command name is "remove", alias "rm")
- [ ] prune.md - Todo
- [ ] move.md - Todo (command name is "move", alias "mv")
- [ ] context.md - Todo
- [ ] sync.md - Todo
- [ ] completions.md - Todo

**Status: 2 of 9 commands documented (22%)**

## Acceptance Criteria

"Done" means: All 9 user-facing commands have companion .md files following the done.md pattern.
