# Argc Migration Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace the manual CLI argument parsing in yx with argc, incrementally migrating one command at a time while keeping yx functional throughout.

**Architecture:** Use argc's annotation-based approach (`@cmd`, `@arg`, `@flag`, etc.) to define CLI interface. Keep all library functions intact. Use hybrid approach where argc routes commands to existing functions, allowing incremental migration.

**Tech Stack:** argc (bash CLI framework), shellspec (testing), bash

**Key Constraints:**
- Tests must pass after each command migration
- Merge to main after each command (or small batch)
- Minimize time on feature branches
- Preserve all existing functionality
- Dogfooding rule: NEVER modify `.yaks` directly

---

## Current Status (2026-01-31)

**Progress:** Phase 2 complete - Infrastructure ready, 2 commands migrated

**Test Status:** ✅ 111/111 tests passing (100%)

**Completed:**
- ✅ Task 1: Extract library functions to lib/yaks.sh
- ✅ Task 2: Update nix flake to bundle argc
- ✅ Task 3: Add argc bootstrap to bin/yx
- ✅ Task 5: Migrate add command to argc
- ✅ Task 6: Migrate list command to argc
- ✅ Bug fix: Fixed symlink resolution for library path
- ✅ Refactor: Extracted `yaks_lib()` helper function

**Deferred:**
- ⏸️ Task 4: Migrate --help command (deferred until commands are defined)

**In Progress:**
- 🔄 Batch 3: Migrating remaining commands (done, rm, prune, move, context, sync, completions)

**Remaining:**
- Task 7: Migrate done command
- Task 8: Migrate remaining simple commands (rm, prune, move, context, sync, completions)
- Task 9: Remove case statement fallbacks
- Task 10: Update completions for argc
- Task 11: Update documentation

---

## Key Learnings & Changes

### 1. Symlink Resolution Issue
**Problem:** Test failures when yx invoked via symlink - couldn't find lib/yaks.sh
**Root Cause:** `${BASH_SOURCE[0]}` points to symlink location, not real file
**Solution:** Use `readlink -f` / `realpath` to resolve symlinks before calculating paths
**Commits:** `521744f`, `fabff26`

### 2. Helper Function Extraction
**Problem:** Duplicate symlink resolution logic in multiple places
**Solution:** Extracted `yaks_lib()` function for finding bundled resources
**Benefits:**
- Single source of truth for path resolution
- Reusable for any bundled resource
- Cleaner code (removed SHOUTY_CASE variables)

### 3. Test Fixes Required
**Issues Found:**
- install.sh didn't copy lib/ directory to installation
- completions return code not propagated
- Function definitions after case statement caused exit code issues

**Fixes Applied:**
- Updated install.sh to copy lib/ directory
- Fixed completions() wrapper to return proper exit codes
- Moved function definitions before case statement

### 4. Command Migration Pattern
**Pattern Established:**
1. Add `@cmd` annotation with `@arg`/`@flag`/`@option` declarations
2. Create wrapper function that calls library implementation
3. Update case statement: try argc first, fallback to old implementation
4. All tests must pass before committing

**Example:**
```bash
# @cmd Add a new yak
# @arg name* The yak name
add() {
  migrate_done_to_state
  check_git_requirements
  if [ ${#argc_name[@]} -eq 0 ]; then
    add_yak_interactive
  else
    add_yak_single "${argc_name[@]}"
  fi
}
```

---

## Phase 1: Preparation & Infrastructure ✅ COMPLETE

### Task 1: Extract Library Functions ✅ COMPLETE

**Status:** ✅ Completed (commit `0c61417`)

**Goal:** Create a separate library file with all reusable functions, leaving only command routing and argc integration in bin/yx.

**Files:**
- Create: `lib/yaks.sh`
- Modify: `bin/yx` (will be refactored incrementally)

**Step 1: Create lib directory**

```bash
mkdir -p lib
```

**Step 2: Create lib/yaks.sh with utility functions**

Extract all the helper functions from bin/yx into lib/yaks.sh. These are the functions that should be extracted (lines 5-851):

```bash
#!/usr/bin/env bash
# Yaks library functions

# Configuration
GIT_WORK_TREE="${GIT_WORK_TREE:-.}"

convert_to_absolute_path() {
  local path="$1"
  case "$path" in
    /*) echo "$path" ;;
    *) echo "$PWD/$path" ;;
  esac
}

GIT_WORK_TREE=$(convert_to_absolute_path "$GIT_WORK_TREE")
YAKS_PATH="$GIT_WORK_TREE/.yaks"

# ... (copy all function definitions from bin/yx lines 16-851)
# This includes:
# - convert_done_file_to_state
# - migrate_done_to_state
# - is_git_repository
# - yaks_path_exists
# - log_command
# - validate_yak_name
# - find_all_yaks
# - is_yak_done
# - try_exact_match
# - try_fuzzy_match
# - find_yak
# - capture_output_and_status
# - require_yak
# - is_macos_stat
# - get_mtime
# - get_sort_priority
# - sort_yaks
# - parse_format_option
# - parse_only_option
# - list_yaks
# - show_help
# - add_yak_interactive
# - add_yak_single
# - add_yak
# - has_incomplete_children
# - mark_yak_done_recursively
# - done_yak
# - remove_yak
# - prune_yaks
# - ensure_parent_yaks_exist
# - move_yak
# - show_yak_context
# - edit_yak_context
# - context_yak
# - detect_user_shell
# - get_shell_rc_file
# - get_completion_file
# - install_completions
# - completions
# - has_origin_remote
# - check_git_setup
# - yaks_path_has_content
# - has_uncommitted_yak_changes
# - use_remote_only
# - create_merge_commit
# - merge_with_git_merge_tree
# - is_ancestor
# - merge_local_and_remote
# - extract_yaks_to_working_dir
# - get_remote_ref
# - get_local_ref
# - merge_remote_into_local_yaks
# - sync_yaks
```

**Step 3: Write test to verify library can be sourced**

Create `spec/unit/library.sh`:

```bash
# shellcheck shell=bash
Describe 'Library loading'
  It 'can source lib/yaks.sh without errors'
    When run sh -c "source lib/yaks.sh && echo 'loaded'"
    The status should be success
    The output should include "loaded"
  End

  It 'exports validate_yak_name function'
    When run sh -c "source lib/yaks.sh && type validate_yak_name"
    The status should be success
  End
End
```

**Step 4: Run test to verify it fails**

```bash
shellspec spec/unit/library.sh
```

Expected: FAIL - lib/yaks.sh doesn't exist

**Step 5: Create lib/yaks.sh with all library functions**

Copy the entire content from bin/yx (functions only, not the case statement or main logic).

**Step 6: Run test to verify it passes**

```bash
shellspec spec/unit/library.sh
```

Expected: PASS

**Step 7: Commit library extraction**

```bash
git add lib/yaks.sh spec/unit/library.sh
git mit me
git commit -m "$(cat <<'EOF'
Extract library functions to lib/yaks.sh

Create separate library file with all reusable functions
to enable cleaner argc integration. This is preparation
for incremental argc migration.
EOF
)"
```

---

### Task 2: Update Nix Flake for Argc ✅ COMPLETE

**Status:** ✅ Completed (commit `1f99d55`)

**Goal:** Update flake.nix to include argc in the release bundle, stealing the approach from the argc branch.

**Files:**
- Modify: `flake.nix`

**Step 1: Read current flake.nix**

```bash
cat flake.nix
```

**Step 2: Modify buildPhase to bundle argc**

Change the buildPhase in flake.nix to:

```nix
            buildPhase = ''
              mkdir -p release-bundle/bin
              mkdir -p release-bundle/lib
              mkdir -p release-bundle/completions

              cp bin/yx release-bundle/bin/
              cp lib/yaks.sh release-bundle/lib/
              cp ${pkgs.argc}/bin/argc release-bundle/lib/
              cp -r completions/* release-bundle/completions/

              cd release-bundle
              zip -r ../yx.zip .
              cd ..
            '';
```

And add argc to nativeBuildInputs:

```nix
            nativeBuildInputs = [ pkgs.zip pkgs.argc ];
```

**Step 3: Test nix build**

```bash
nix build
```

Expected: Build succeeds and yx.zip includes lib/argc

**Step 4: Verify zip contents**

```bash
unzip -l result/yx.zip | grep -E "(argc|yaks.sh)"
```

Expected: Shows lib/argc and lib/yaks.sh

**Step 5: Commit nix changes**

```bash
git add flake.nix
git mit me
git commit -m "$(cat <<'EOF'
Add argc and library to nix build

Include argc binary and yaks library in release bundle
to support argc-based CLI. This enables argc to be
bundled with releases.
EOF
)"
```

---

## Phase 2: Hybrid Architecture Setup ✅ COMPLETE

### Task 3: Add Argc Bootstrap to bin/yx ✅ COMPLETE

**Status:** ✅ Completed (commit `afc906f`, later refactored in `fabff26`)

**Goal:** Add argc integration at the bottom of bin/yx while preserving the existing case statement for backward compatibility.

**Files:**
- Modify: `bin/yx`

**Step 1: Write test for argc detection**

Create `spec/unit/argc_bootstrap.sh`:

```bash
# shellcheck shell=bash
Describe 'Argc bootstrap'
  It 'has argc available in development environment'
    When run command -v argc
    The status should be success
  End

  It 'bin/yx can find argc'
    When run sh -c "
      YX_REAL_PATH='$(pwd)' source bin/yx 2>&1 | head -1
    "
    The status should not equal 1
  End
End
```

**Step 2: Run test**

```bash
shellspec spec/unit/argc_bootstrap.sh
```

Expected: May fail or pass, establishes baseline

**Step 3: Add argc bootstrap code to bin/yx**

At the very end of bin/yx (after the case statement), add:

```bash
# Argc integration bootstrap
# This allows gradual migration to argc-based commands

# Find argc relative to yx installation
find_argc() {
  local yx_real_path="$(cd "$(dirname "$(readlink -f "${BASH_SOURCE[0]}" 2>/dev/null || realpath "${BASH_SOURCE[0]}" 2>/dev/null || echo "${BASH_SOURCE[0]}")")" && pwd)"
  local argc_path="$yx_real_path/../lib/argc"

  # Use bundled argc if available, otherwise system argc
  if [ -x "$argc_path" ]; then
    echo "$argc_path"
  elif command -v argc >/dev/null 2>&1; then
    command -v argc
  else
    return 1
  fi
}

# Note: Argc evaluation will be added incrementally as commands migrate
# For now, this just sets up the infrastructure
```

**Step 4: Run all tests**

```bash
shellspec
```

Expected: All tests still pass (no behavior change yet)

**Step 5: Commit argc bootstrap**

```bash
git add bin/yx spec/unit/argc_bootstrap.sh
git mit me
git commit -m "$(cat <<'EOF'
Add argc bootstrap infrastructure

Add find_argc function to locate argc binary. This
prepares for incremental command migration without
changing any behavior yet.
EOF
)"
```

---

## Phase 3: First Command Migration 🔄 IN PROGRESS

### Task 4: Migrate --help Command to Argc ⏸️ DEFERRED

**Status:** ⏸️ Deferred until after Task 8 - argc needs @cmd annotations to generate useful help

**Goal:** Migrate the simplest command (--help) to argc as a proof of concept. This establishes the pattern for other commands.

**Note:** Argc generates help from @cmd annotations. Without commands defined, help output is minimal. Will revisit after remaining commands are migrated.

**Files:**
- Modify: `bin/yx`

**Step 1: Add argc annotations for help**

At the top of bin/yx, add argc annotations:

```bash
#!/usr/bin/env bash
# @describe A non-linear TODO list for humans and robots
# @version 0.1.0

# Source library functions
YX_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
if [ -f "$YX_DIR/lib/yaks.sh" ]; then
  # shellcheck source=../lib/yaks.sh
  source "$YX_DIR/lib/yaks.sh"
else
  echo "Error: Could not find lib/yaks.sh" >&2
  exit 1
fi

# Rest of the file...
```

**Step 2: Test --help still works**

```bash
./bin/yx --help
```

Expected: Shows help (using old case statement still)

**Step 3: Update case statement to use argc for empty/help**

Replace the case statement entry for help:

```bash
case "$1" in
  --help|"")
    # Check if argc is available
    argc_bin=$(find_argc)
    if [ $? -eq 0 ]; then
      # Use argc for help
      eval "$("$argc_bin" --argc-eval "$0" "$@")"
      exit $?
    else
      # Fallback to old help
      show_help
    fi
    ;;
  # ... rest of case statement unchanged ...
esac
```

**Step 4: Run help tests**

```bash
shellspec spec/features/yx.sh
```

Expected: Tests pass

**Step 5: Manual verification**

```bash
./bin/yx --help
./bin/yx
```

Expected: Both show help

**Step 6: Commit help migration**

```bash
git add bin/yx
git mit me
git commit -m "$(cat <<'EOF'
Migrate --help command to argc

First command migrated to argc. Falls back to original
implementation if argc not available. This establishes
the hybrid pattern for incremental migration.
EOF
)"
```

---

### Task 5: Migrate add Command to Argc ✅ COMPLETE

**Status:** ✅ Completed (commit `fee16bf`)

**Goal:** Migrate the `add` command to use argc's argument parsing while keeping existing add_yak functions.

**Files:**
- Modify: `bin/yx`

**Step 1: Write test for argc add command**

The existing tests in `spec/features/add.sh` should continue to work. Run them to establish baseline:

```bash
shellspec spec/features/add.sh
```

Expected: All pass

**Step 2: Add argc annotation for add command**

Add after the `@version` line:

```bash
# @cmd Add a new yak
# @arg name* The yak name (space-separated words)
add() {
  migrate_done_to_state
  check_git_requirements

  if [ ${#argc_name[@]} -eq 0 ]; then
    add_yak_interactive
  else
    add_yak_single "${argc_name[@]}"
  fi
}
```

**Step 3: Create check_git_requirements function**

Add to lib/yaks.sh:

```bash
check_git_requirements() {
  if ! command -v git >/dev/null 2>&1; then
    echo "Error: git command not found" >&2
    echo "yx requires git to be installed" >&2
    exit 1
  fi

  if ! is_git_repository; then
    echo "Error: not in a git repository" >&2
    echo "yx must be run from within a git repository" >&2
    exit 1
  fi

  if ! git -C "$GIT_WORK_TREE" check-ignore -q .yaks; then
    echo "Error: .yaks folder is not gitignored" >&2
    echo "Please add .yaks to your .gitignore file" >&2
    exit 1
  fi
}
```

**Step 4: Update case statement for add**

```bash
  add)
    # Try argc first
    argc_bin=$(find_argc)
    if [ $? -eq 0 ]; then
      eval "$("$argc_bin" --argc-eval "$0" "$@")"
      exit $?
    fi
    # Fallback
    shift
    add_yak "$@"
    ;;
```

**Step 5: Run add tests**

```bash
shellspec spec/features/add.sh
```

Expected: All 13 tests pass

**Step 6: Manual smoke test**

```bash
YAK_PATH=$(mktemp -d)
cd /tmp && git init test-add && cd test-add
echo ".yaks" > .gitignore
git add .gitignore && git commit -m "init"
GIT_WORK_TREE=/tmp/test-add yx add "test yak"
GIT_WORK_TREE=/tmp/test-add yx list
```

Expected: Shows "- [ ] test yak"

**Step 7: Cleanup test repo**

```bash
rm -rf /tmp/test-add
```

**Step 8: Commit add migration**

```bash
git add bin/yx lib/yaks.sh
git mit me
git commit -m "$(cat <<'EOF'
Migrate add command to argc

Convert add command to use argc argument parsing.
Preserves all existing functionality including
interactive mode and multi-word names.
EOF
)"
```

---

### Task 6: Migrate list Command to Argc ✅ COMPLETE

**Status:** ✅ Completed (commit `3209cdb`)

**Goal:** Migrate the `list` command with its options (--format, --only) to argc.

**Files:**
- Modify: `bin/yx`

**Step 1: Run existing list tests**

```bash
shellspec spec/features/list.sh
```

Expected: All pass (baseline)

**Step 2: Add argc annotation for list command**

```bash
# @cmd List all yaks
# @alias ls
# @option --format[markdown|md|plain|raw] Output format (default: markdown)
# @option --only[done|not-done] Filter by completion status
list() {
  migrate_done_to_state
  check_git_requirements

  # Parse format
  local format="${argc_format:-markdown}"

  # Parse only filter
  local only="${argc_only:-}"

  list_yaks_impl "$format" "$only"
}
```

**Step 3: Refactor list_yaks to list_yaks_impl**

In lib/yaks.sh, rename `list_yaks` to `list_yaks_impl` and change signature:

```bash
list_yaks_impl() {
  local format="$1"
  local only="$2"

  # ... rest of function unchanged, but use $format and $only directly
  # instead of parsing from arguments
}
```

**Step 4: Update case statement for list**

```bash
  list|ls)
    # Try argc first
    argc_bin=$(find_argc)
    if [ $? -eq 0 ]; then
      eval "$("$argc_bin" --argc-eval "$0" "$@")"
      exit $?
    fi
    # Fallback
    shift
    list_yaks "$@"
    ;;
```

**Step 5: Run list tests**

```bash
shellspec spec/features/list.sh
```

Expected: All tests pass

**Step 6: Manual smoke test**

```bash
yx list
yx list --format plain
yx list --only not-done
```

Expected: All work correctly

**Step 7: Commit list migration**

```bash
git add bin/yx lib/yaks.sh
git mit me
git commit -m "$(cat <<'EOF'
Migrate list command to argc

Convert list command to use argc with proper option
handling for --format and --only flags.
EOF
)"
```

---

### Task 7: Migrate done Command to Argc

**Goal:** Migrate the `done` command with its --undo and --recursive flags.

**Files:**
- Modify: `bin/yx`

**Step 1: Run existing done tests**

```bash
shellspec spec/features/done.sh
```

Expected: All pass

**Step 2: Add argc annotation for done command**

```bash
# @cmd Mark a yak as done
# @flag --undo Unmark a done yak
# @flag --recursive Mark all children as done too
# @arg name* The yak name
done() {
  migrate_done_to_state
  check_git_requirements

  local yak_name="${argc_name[*]}"

  if [ "${argc_undo:-0}" = "1" ]; then
    done_yak --undo "$yak_name"
  elif [ "${argc_recursive:-0}" = "1" ]; then
    done_yak --recursive "$yak_name"
  else
    done_yak "$yak_name"
  fi
}
```

**Step 3: Update case statement**

```bash
  done)
    # Try argc first
    argc_bin=$(find_argc)
    if [ $? -eq 0 ]; then
      eval "$("$argc_bin" --argc-eval "$0" "$@")"
      exit $?
    fi
    # Fallback
    shift
    done_yak "$@"
    ;;
```

**Step 4: Run done tests**

```bash
shellspec spec/features/done.sh
```

Expected: All tests pass

**Step 5: Commit done migration**

```bash
git add bin/yx
git mit me
git commit -m "$(cat <<'EOF'
Migrate done command to argc

Convert done command with --undo and --recursive flags
to argc-based parsing.
EOF
)"
```

---

### Task 8: Migrate Remaining Simple Commands

**Goal:** Migrate rm, prune, move, context, sync, completions to argc.

**Files:**
- Modify: `bin/yx`

**Step 1: Add argc annotations for all remaining commands**

```bash
# @cmd Remove a yak
# @alias rm
# @arg name* The yak name
remove() {
  migrate_done_to_state
  check_git_requirements
  remove_yak "${argc_name[*]}"
}

# @cmd Remove all done yaks
prune() {
  migrate_done_to_state
  check_git_requirements
  prune_yaks
}

# @cmd Rename a yak
# @alias mv
# @arg old! The old yak name
# @arg new! The new yak name
move() {
  migrate_done_to_state
  check_git_requirements
  move_yak "$argc_old" "$argc_new"
}

# @cmd Edit or show yak context
# @flag --show Display yak with context
# @flag --edit Edit context (default)
# @arg name* The yak name
context() {
  migrate_done_to_state
  check_git_requirements

  local yak_name="${argc_name[*]}"

  if [ "${argc_show:-0}" = "1" ]; then
    show_yak_context "$yak_name"
  else
    edit_yak_context "$yak_name"
  fi
}

# @cmd Push and pull yaks to/from origin via git ref
sync() {
  migrate_done_to_state
  check_git_requirements
  sync_yaks
}

# @cmd Output yak names for shell completion
# @arg cmd The command being completed
# @arg flag Optional flag being completed
completions() {
  migrate_done_to_state
  check_git_requirements
  completions_impl "$argc_cmd" "${argc_flag:-}"
}
```

**Step 2: Update all case statement entries**

Update each case entry to try argc first, then fall back.

**Step 3: Run all tests**

```bash
shellspec
```

Expected: All tests pass

**Step 4: Commit remaining migrations**

```bash
git add bin/yx
git mit me
git commit -m "$(cat <<'EOF'
Migrate remaining commands to argc

Complete argc migration for rm, prune, move, context,
sync, and completions commands.
EOF
)"
```

---

## Phase 4: Cleanup and Finalization

### Task 9: Remove Case Statement Fallbacks

**Goal:** Remove the old case statement since all commands now use argc.

**Files:**
- Modify: `bin/yx`

**Step 1: Run full test suite**

```bash
shellspec
```

Expected: All tests pass

**Step 2: Replace case statement with direct argc call**

At the end of bin/yx, replace the entire case statement with:

```bash
# Run migrations
migrate_done_to_state

# Find and execute argc
argc_bin=$(find_argc)
if [ $? -ne 0 ]; then
  echo "Error: argc not found" >&2
  echo "Please ensure argc is installed" >&2
  exit 1
fi

# Argc integration - evaluates commands defined above
eval "$("$argc_bin" --argc-eval "$0" "$@")"
```

**Step 3: Run all tests again**

```bash
shellspec
```

Expected: All tests still pass

**Step 4: Remove old functions from lib/yaks.sh**

Remove wrapper functions that are no longer needed (like show_help if it's been replaced).

**Step 5: Run tests one final time**

```bash
shellspec
```

Expected: All pass

**Step 6: Commit cleanup**

```bash
git add bin/yx lib/yaks.sh
git mit me
git commit -m "$(cat <<'EOF'
Remove case statement fallbacks

Complete argc migration by removing old argument
parsing. All commands now use argc exclusively.
EOF
)"
```

---

### Task 10: Update Completions for Argc

**Goal:** Regenerate shell completions using argc's built-in completion support.

**Files:**
- Modify: `completions/yx.bash`
- Modify: `completions/yx.zsh`

**Step 1: Generate argc completions**

```bash
argc_bin=$(command -v argc)
"$argc_bin" --argc-completions bash bin/yx > completions/yx.bash
"$argc_bin" --argc-completions zsh bin/yx > completions/yx.zsh
```

**Step 2: Test bash completions**

```bash
bash --norc --noprofile -c "
  source completions/yx.bash
  complete -p yx
"
```

Expected: Shows completion configuration

**Step 3: Test zsh completions**

```bash
zsh -c "
  source completions/yx.zsh
  which _yx
"
```

Expected: Shows function definition

**Step 4: Run completion tests**

```bash
shellspec spec/features/completions.sh
```

Expected: Tests pass (may need updates)

**Step 5: Commit updated completions**

```bash
git add completions/
git mit me
git commit -m "$(cat <<'EOF'
Regenerate shell completions using argc

Use argc's built-in completion generation for better
shell integration and automatic option completion.
EOF
)"
```

---

### Task 11: Update Documentation

**Goal:** Update README and help text to reflect argc-based CLI.

**Files:**
- Modify: `README.md`
- Modify: `.claude/CLAUDE.md`

**Step 1: Update README with new argc-based usage**

Update any relevant sections mentioning command parsing or internal architecture.

**Step 2: Update CLAUDE.md if needed**

Update project instructions to mention argc.

**Step 3: Test help output**

```bash
./bin/yx --help
```

Expected: Clean, argc-generated help

**Step 4: Commit documentation updates**

```bash
git add README.md .claude/CLAUDE.md
git mit me
git commit -m "$(cat <<'EOF'
Update documentation for argc migration

Reflect new argc-based architecture in project
documentation and development guides.
EOF
)"
```

---

## Success Criteria

- [ ] All tests pass after migration
- [ ] argc is bundled in nix releases
- [ ] All commands use argc annotations
- [ ] No behavior changes (backward compatible)
- [ ] Shell completions work with argc
- [ ] Can merge to main after each task or small batch
- [ ] Library functions cleanly separated from CLI routing

## Testing Strategy

After each task:
1. Run full test suite: `shellspec`
2. Manual smoke test for the migrated command
3. Verify help output: `yx --help`
4. Check that non-migrated commands still work

## Rollback Plan

If issues arise:
- Each commit is atomic and can be reverted
- Hybrid architecture allows falling back to old implementation
- Tests protect against regressions

## Notes

- Argc uses eval, so quoting and escaping must be careful
- The `argc_*` variables are set by argc's eval
- `@arg name*` means "zero or more arguments" stored in argc_name array
- `@arg name!` means "exactly one required argument"
- `@flag` creates argc_flagname variable (0 or 1)
- `@option` creates argc_optionname variable with value
