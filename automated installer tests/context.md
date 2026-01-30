# Automated Installer Tests

Goal: We want to be able to run automated tests of our users' install.sh script.

Those tests should be runnable locally or in CI.

The script needs some affordances for short-circuiting download of a release, and for skipping interactive inputs.

## Status: COMPLETE

**Plan Location:** `.worktrees/automated-installer-tests/docs/plans/2026-01-29-automated-installer-tests.md`

**All Tasks Completed:**
- ✓ Task 1: Fixed test expectation (commit 890801b)
- ✓ Task 2: Added environment variable support (commit 8ff5b88)
- ✓ Task 3: Added YX_SOURCE with zip handling (commits 0b9c4e3, 332923b)
- ✓ Task 4: Tested with local release (verified via automated test)
- ✓ Task 5: Ran full test suite (install.sh test passes, 2 pre-existing failures unrelated)
- ✓ Task 6: Cleaned up unused files (commit d5533de)
- ✓ Task 7: Updated documentation (commit 3b53755)

**Success Criteria Met:**
- ✓ shellspec spec/features/install.sh passes
- ✓ Install.sh works with local zip and GitHub download
- ✓ Documentation updated with environment variables

**Ready to merge to main.**
