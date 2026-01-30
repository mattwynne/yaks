# Automated Installer Tests

Goal: We want to be able to run automated tests of our users' install.sh script.

Those tests should be runnable locally or in CI.

The script needs some affordances for short-circuiting download of a release, and for skipping interactive inputs.

## Status: Implementation In Progress

**Plan Location:** `.worktrees/automated-installer-tests/docs/plans/2026-01-29-automated-installer-tests.md`

**Current State:**
- ✓ Brainstormed and validated design
- ✓ Created worktree at `.worktrees/automated-installer-tests`
- ✓ Built release artifact (release/yx.zip)
- ✓ Created test infrastructure
- ✓ Created detailed implementation plan
- ✓ **Task 1 COMPLETE: Fixed test expectation (commit 890801b)**
  - Updated spec/features/install.sh to expect success
  - Added smoke tests (yx --help, yx add foo, yx ls)
  - Test runs and fails as expected (YX_SOURCE not supported yet)
- ✓ **Task 2 COMPLETE: Add Environment Variable Support (commit 8ff5b88)**
  - Added YX_SHELL_CHOICE env var to skip shell choice prompt
  - Added YX_AUTO_COMPLETE env var to skip auto-complete prompt
  - Install.sh now supports non-interactive mode for testing

**Implementation Progress:**
- [x] Task 1: Fix Test Expectation
- [x] Task 2: Add Environment Variable Support for Prompts
- [ ] Task 3: Add YX_SOURCE with Zip Handling
- [ ] Task 4: Test with Local Release (Verification)
- [ ] Task 5: Run Full Test Suite
- [ ] Task 6: Clean Up Unused Files
- [ ] Task 7: Update Documentation

**Next Steps:**
Task 3 is ready to implement - add YX_SOURCE support with zip handling to replace local/download branching.

**Success Criteria:**
- shellspec spec/features/install.sh passes
- All existing tests pass
- install.sh works with local zip and GitHub download
- Documentation updated
