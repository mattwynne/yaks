# shellcheck shell=bash
Describe 'yx sync does not pollute working tree or index'
  It 'does not add files to git index'
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

    # Check that nothing is staged (no files in index except what was already there)
    # .yaks/ will show as untracked, which is expected and correct
    When call git -C "$REPO" diff --cached --name-only
    The output should equal ""
    The status should be success

    rm -rf "$ORIGIN" "$REPO"
  End
End
