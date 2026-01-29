Describe 'yx sync - unit tests'
  setup_repo() {
    REPO=$(mktemp -d)
    setup_test_repo "$REPO"
    echo "# Test" > "$REPO/README.md"
    git -C "$REPO" add README.md
    git -C "$REPO" commit -m "Initial commit" --quiet
  }

  cleanup_repo() {
    rm -rf "$REPO"
  }

  BeforeEach 'setup_repo'
  AfterEach 'cleanup_repo'

  It 'creates refs/notes/yaks when yak exists'
    GIT_WORK_TREE="$REPO" "yx" add "test yak"

    cd "$REPO"
    # Mock origin to avoid push/fetch errors
    git remote add origin "$REPO"
    GIT_WORK_TREE="$REPO" "yx" sync 2>&1

    When call git rev-parse refs/notes/yaks
    The status should be success
    The stdout should be present
  End

  It 'stores yak directory in refs/notes/yaks'
    GIT_WORK_TREE="$REPO" "yx" add "test yak"
    echo "some context" | GIT_WORK_TREE="$REPO" "yx" context "test yak"

    cd "$REPO"
    git remote add origin "$REPO"
    GIT_WORK_TREE="$REPO" "yx" sync 2>&1

    # Check that we can list files from the ref
    When call git ls-tree -r --name-only refs/notes/yaks
    The output should include "test yak"
  End

  It 'extracts yaks from refs/notes/yaks after sync'
    GIT_WORK_TREE="$REPO" "yx" add "original yak"

    cd "$REPO"
    git remote add origin "$REPO"
    GIT_WORK_TREE="$REPO" "yx" sync 2>&1

    # The yak should still exist after sync
    When call sh -c "GIT_WORK_TREE='$REPO' 'yx' ls"
    The output should include "original yak"
  End
End
