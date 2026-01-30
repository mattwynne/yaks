# Automated Installer Tests

Goal: We want to be able to run automated tests of our users' install.sh script.

Those tests should be runnable locally or in CI.

The script needs some affordances for short-circuiting download of a release, and for skipping interactive inputs.

## Status: Implementation Plan Created

**Plan Location:** `docs/plans/2026-01-29-automated-installer-tests.md`

**Current State:**
- ✓ Brainstormed and validated design
- ✓ Created worktree at `.worktrees/automated-installer-tests`
- ✓ Built release artifact (release/yx.zip)
- ✓ Created test infrastructure (test in RED state)
- ✓ Created detailed implementation plan

**Implementation Plan Summary:**
1. Fix test expectation (expect success + smoke tests)
2. Add env var support for prompts (YX_SHELL_CHOICE, YX_AUTO_COMPLETE)
3. Add YX_SOURCE with zip handling (main refactor)
4. Verify with local release
5. Run full test suite
6. Clean up unused files
7. Update documentation

**Next Steps:**
Use superpowers:executing-plans or superpowers:subagent-driven-development to implement the plan task-by-task.

**Success Criteria:**
- shellspec spec/features/install.sh passes
- All existing tests pass
- install.sh works with local zip and GitHub download
- Documentation updated
