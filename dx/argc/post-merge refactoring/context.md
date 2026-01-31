# Post-Merge Refactoring: Restore Code Quality

The argc integration works but regressed code quality. This yak tracks refactoring to restore maintainability while keeping argc benefits.

## Core Problems

1. **list() function is monolithic** (125 lines, lines 273-398)
   - Complex hierarchical sorting embedded inline (lines 330-370)
   - No separation of concerns
   - Hard to understand and modify

2. **Code duplication** - stat command logic repeated twice
   - Lines 309-315 and 358-363 are identical
   - Should be extracted to `get_file_mtime()` helper

3. **Missing helper functions** that old version had:
   - `validate_yak_name()` - validation now duplicated in 3 places
   - `is_yak_done()` - state checking now inlined
   - `get_sort_priority()` - priority logic now inlined
   - Helper for mtime extraction

4. **Migration runs in loops** instead of once at startup
   - `migrate_yak_state()` called inside list(), prune(), completions()
   - Performance issue with many yaks
   - Should run once at startup like old version

5. **Magic strings** - "done" and "todo" scattered throughout
   - Should be constants at top of file

## Refactoring Tasks (in order)

### Phase 1: Extract Helpers (Low Risk)
- [ ] Extract `get_file_mtime()` to eliminate duplication (lines 309-315, 358-363)
- [ ] Extract `validate_yak_name()` function (consolidate lines 206-209, 220-223, 434-436)
- [ ] Add state constants: `STATE_TODO="todo"` and `STATE_DONE="done"`
- [ ] Extract `is_yak_done()` helper
- [ ] Extract `get_sort_priority()` helper
- [ ] Run tests after each extraction

### Phase 2: Refactor list() Function (Medium Risk)
- [ ] Extract `should_display_yak()` - filter logic (lines 322-328)
- [ ] Extract `get_yak_metadata()` - state, mtime, depth for single yak
- [ ] Extract `build_sort_key()` - the complex sorting logic (lines 330-370)
- [ ] Extract `format_yak_output()` - markdown vs plain formatting (lines 385-394)
- [ ] Simplify main list() body to use these helpers
- [ ] Run tests to verify sorting still works correctly

### Phase 3: Optimize Migration (Low Risk)
- [ ] Move migration to startup (run once before argc eval)
- [ ] Remove migrate_yak_state() calls from loops
- [ ] Verify with tests

### Phase 4: Polish (Low Risk)
- [ ] Simplify `capture_output_and_status()` or document why temp file is needed
- [ ] Add comments to argc initialization section (lines 793-843)
- [ ] Document move() parent directory creation behavior

## Success Criteria

- All tests pass
- list() function under 50 lines
- No duplicated validation or mtime logic
- Migration runs once, not in loops
- Code is as readable as old version while keeping argc benefits

## Reference

Compare with bin/yx.backup which had better structure:
- list() used recursive `list_dir()` with inner helpers (lines 209-308)
- Clean helper functions for validation, state checking, sorting
- Migration at startup only (line 803)

## Notes

- Keep the improvements from new version:
  - Robust `check_gitignore()` with config isolation
  - `finish()` command name with `done` alias
  - Custom help() function
- Don't break argc integration
- Maintain test coverage throughout refactoring
