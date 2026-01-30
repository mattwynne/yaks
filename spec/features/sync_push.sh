# shellcheck shell=bash
Describe 'yx sync push'
  It 'can push refs/notes/yaks to bare repo'
    ORIGIN=$(mktemp -d)
    REPO=$(mktemp -d)

    # Set up bare origin and clone
    setup_bare_repo "$ORIGIN"
    setup_test_repo "$REPO" "test@example.com" "Test" "$ORIGIN"
    echo "test" > "$REPO/README.md"
    git -C "$REPO" add README.md
    git -C "$REPO" commit -m "init" --quiet
    git -C "$REPO" push -u origin main --quiet

    # Add a yak and sync
    GIT_WORK_TREE="$REPO" "yx" add "test yak"
    cd "$REPO" || return
    GIT_WORK_TREE="$REPO" "yx" sync 2>&1

    # Check if refs/notes/yaks exists in origin
    When call git -C "$ORIGIN" show-ref refs/notes/yaks
    The status should be success
    The output should include "refs/notes/yaks"

    rm -rf "$ORIGIN" "$REPO"
  End
End
