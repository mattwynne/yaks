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

Core user-facing commands:
- [x] done.md - Completed
- [x] add.md - Completed
- [ ] list.md - Todo
- [ ] context.md - Todo
- [ ] rm.md - Todo
- [ ] prune.md - Todo
- [ ] move.md - Todo
- [ ] sync.md - Todo

**Status: 2 of 8 core commands documented (25%)**

## Acceptance Criteria

"Done" means: All 8 core user-facing commands have companion .md files following the done.md pattern.
