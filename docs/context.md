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

## Progress

- ✅ Created template by documenting `done` command (spec/features/done.md)
- 📋 22 total features in spec/features/
- 🎯 Next: Document remaining core user-facing commands (add, list, context, rm, prune, move)

## Acceptance Criteria

"Done" means: Core user-facing commands (add, done, list, context, rm, prune, move) have companion .md files following the done.md pattern.
