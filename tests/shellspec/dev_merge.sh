# shellcheck shell=bash
# shellcheck disable=SC2034 # ShellSpec reads assertion variables dynamically.

Describe 'bin/dev merge'
  It 'marks the matching yak done after fast-forwarding and cleaning up the branch'
    When run bash -c '
      set -e
      repo=$(mktemp -d)
      trap "rm -rf \"$repo\"" EXIT
      git -C "$repo" init --initial-branch=main --quiet
      git -C "$repo" config user.email test@example.com
      git -C "$repo" config user.name "Test User"
      echo base > "$repo/file.txt"
      git -C "$repo" add file.txt
      git -C "$repo" commit --quiet -m base
      git -C "$repo" switch --quiet -c yak-123
      echo change >> "$repo/file.txt"
      git -C "$repo" commit --quiet -am change
      git -C "$repo" switch --quiet main

      cd "$repo"
      YX_DEV_SOURCE_ONLY=1 . "$TEST_PROJECT_DIR/bin/dev"
      check() { :; }
      yx() { echo "$*" >> "$repo/yx.log"; }
      argc_branch=yak-123
      merge

      git rev-parse --verify yak-123 >/dev/null 2>&1 && exit 1
      grep -Fx "done yak-123" "$repo/yx.log"
    '
    The status should be success
    The output should include '✅ Branch yak-123 merged to main'
    The output should include 'done yak-123'
    The error should include "Preparing worktree (checking out 'yak-123')"
  End

  It 'fails before checks when the branch worktree is dirty'
    When run bash -c '
      set -e
      repo=$(mktemp -d)
      wt="$repo-wt"
      trap "rm -rf \"$repo\" \"$wt\"" EXIT
      git -C "$repo" init --initial-branch=main --quiet
      git -C "$repo" config user.email test@example.com
      git -C "$repo" config user.name "Test User"
      echo base > "$repo/file.txt"
      git -C "$repo" add file.txt
      git -C "$repo" commit --quiet -m base
      git -C "$repo" branch yak-123
      git -C "$repo" worktree add --quiet "$wt" yak-123
      echo dirty >> "$wt/file.txt"

      cd "$repo"
      YX_DEV_SOURCE_ONLY=1 . "$TEST_PROJECT_DIR/bin/dev"
      check() { echo SHOULD_NOT_CHECK; return 1; }
      argc_branch=yak-123
      merge
    '
    The status should be failure
    The output should not include 'SHOULD_NOT_CHECK'
    The error should include 'Cannot merge: worktree has uncommitted changes'
  End

  It 'runs only changed .pi extension npm tests for .pi-only merges'
    When run bash -c '
      set -e
      repo=$(mktemp -d)
      trap "rm -rf \"$repo\"" EXIT
      git -C "$repo" init --initial-branch=main --quiet
      git -C "$repo" config user.email test@example.com
      git -C "$repo" config user.name "Test User"
      mkdir -p "$repo/.pi/extensions/protect-main"
      echo base > "$repo/README.md"
      git -C "$repo" add README.md
      git -C "$repo" commit --quiet -m base
      git -C "$repo" switch --quiet -c yak-123
      cat > "$repo/.pi/extensions/protect-main/package.json" <<JSON
{"scripts":{"test":"echo package test"}}
JSON
      echo change > "$repo/.pi/extensions/protect-main/index.ts"
      git -C "$repo" add .pi
      git -C "$repo" commit --quiet -m pi-change
      git -C "$repo" switch --quiet main

      cd "$repo"
      YX_DEV_SOURCE_ONLY=1 . "$TEST_PROJECT_DIR/bin/dev"
      check() { echo FULL_CHECK_SHOULD_NOT_RUN; return 1; }
      npm() { echo "npm $* in ${PWD#$repo/}"; }
      yx() { :; }
      argc_branch=yak-123
      merge
    '
    The status should be success
    The output should include 'Running .pi extension tests in .pi/extensions/protect-main'
    The output should include 'npm test in'
    The output should include '.pi/extensions/protect-main'
    The output should not include 'FULL_CHECK_SHOULD_NOT_RUN'
    The error should include "Preparing worktree (checking out 'yak-123')"
  End

  It 'marks the matching yak done after a successful merge cleanup'
    When run bash -c '
      YX_DEV_SOURCE_ONLY=1 . "$TEST_PROJECT_DIR/bin/dev"
      yx() { echo "yx $*"; }
      mark_merged_yak_done yak-123
    '
    The status should be success
    The output should include 'yx done yak-123'
    The output should include '✓ Marked yak yak-123 done'
  End

  It 'warns but succeeds when marking the yak done fails'
    When run bash -c '
      YX_DEV_SOURCE_ONLY=1 . "$TEST_PROJECT_DIR/bin/dev"
      yx() { echo "no matching yak" >&2; return 1; }
      mark_merged_yak_done feature-branch
    '
    The status should be success
    The error should include 'Warning: merged branch feature-branch, but could not mark matching yak done'
    The error should include 'no matching yak'
  End
End
