Describe 'yx sync push'
  It 'can push refs/notes/yaks to bare repo'
    ORIGIN=$(mktemp -d)
    REPO=$(mktemp -d)
    YX_BIN="$(pwd)/bin/yx"

    # Set up bare origin and clone
    git -C "$ORIGIN" init --bare --quiet
    git -C "$REPO" init --quiet
    git -C "$REPO" remote add origin "$ORIGIN"
    git -C "$REPO" config user.email "test@example.com"
    git -C "$REPO" config user.name "Test"
    echo "test" > "$REPO/README.md"
    git -C "$REPO" add README.md
    git -C "$REPO" commit -m "init" --quiet
    git -C "$REPO" push -u origin main --quiet

    # Add a yak and sync
    YAKS_PATH="$REPO/.yaks" "$YX_BIN" add "test yak"
    cd "$REPO"
    YAKS_PATH="$REPO/.yaks" "$YX_BIN" sync 2>&1

    # Check if refs/notes/yaks exists in origin
    When call git -C "$ORIGIN" show-ref refs/notes/yaks
    The status should be success
    The output should include "refs/notes/yaks"

    rm -rf "$ORIGIN" "$REPO"
  End
End
