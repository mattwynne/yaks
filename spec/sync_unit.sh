Describe 'yx sync - unit tests'
  setup_repo() {
    REPO=$(mktemp -d)
    git -C "$REPO" init --quiet
    git -C "$REPO" config user.email "test@example.com"
    git -C "$REPO" config user.name "Test User"
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
    YAKS_PATH="$REPO/.yaks" "yx" add "test yak"

    cd "$REPO"
    # Mock origin to avoid push/fetch errors
    git remote add origin "$REPO"
    YAKS_PATH="$REPO/.yaks" "yx" sync 2>&1

    When call git -C "$REPO" rev-parse refs/notes/yaks
    The status should be success
    The stdout should be present
  End

  It 'stores yak directory in refs/notes/yaks'
    YAKS_PATH="$REPO/.yaks" "yx" add "test yak"
    echo "some context" > "$REPO/.yaks/test yak/context.md"

    cd "$REPO"
    git remote add origin "$REPO"
    YAKS_PATH="$REPO/.yaks" "yx" sync 2>&1

    # Check that we can list files from the ref
    When call git -C "$REPO" ls-tree -r --name-only refs/notes/yaks
    The output should include "test yak"
  End

  It 'extracts yaks from refs/notes/yaks after sync'
    YAKS_PATH="$REPO/.yaks" "yx" add "original yak"

    cd "$REPO"
    git remote add origin "$REPO"
    YAKS_PATH="$REPO/.yaks" "yx" sync 2>&1

    # The yak should still exist after sync
    When call sh -c "YAKS_PATH='$REPO/.yaks' 'yx' ls"
    The output should include "original yak"
  End
End
