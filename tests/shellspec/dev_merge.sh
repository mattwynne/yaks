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
