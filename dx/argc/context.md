dx/argc

## Status: ✅ COMPLETE - All 109 Tests Passing

**Working Branch:** `.worktrees/dx-argc` (branch: `dx-argc`)

**Latest Progress:** All argc migration complete - 109/109 tests passing!

### Implementation Summary

Successfully migrated from bash case statement to argc CLI framework:

**Core Implementation:**
- ✅ All commands use argc annotations (@cmd, @alias, @arg, @flag, @option)
- ✅ Nested subcommands using completions install pattern
- ✅ Fuzzy matching for yak names with ambiguity detection
- ✅ Git availability and repository checks
- ✅ Full sync implementation with merge logic
- ✅ Prune logging for each removed yak
- ✅ RC_FILE env var for testable completions install

**Test Results:** 109 examples, 0 failures, 1 warning, 1 skip
- Skip: installer test (requires `dev release`)
- Warning: installer test (requires release artifact)

**Key Functions Ported from Main:**
- extract_yaks_to_working_dir()
- Sync merge logic (has_uncommitted_yak_changes, merge_local_and_remote, etc.)
- Fuzzy matching (find_yak, try_fuzzy_match, require_yak)
- Git checks (is_git_repository, check_git_setup)

**Ready for merge to main.**
