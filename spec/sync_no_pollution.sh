Describe 'yx sync does not pollute working tree or index'
  It 'does not add files to git index'
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
    YAK_PATH="$REPO/.yaks" "$YX_BIN" add "test yak"
    cd "$REPO"
    YAK_PATH="$REPO/.yaks" "$YX_BIN" sync 2>&1

    # Check that nothing is staged (no files in index except what was already there)
    # .yaks/ will show as untracked, which is expected and correct
    When call git -C "$REPO" diff --cached --name-only
    The output should equal ""
    The status should be success

    rm -rf "$ORIGIN" "$REPO"
  End

  It 'does not leave yak files outside .yaks directory'
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
    YAK_PATH="$REPO/.yaks" "$YX_BIN" add "test yak"
    cd "$REPO"
    YAK_PATH="$REPO/.yaks" "$YX_BIN" sync 2>&1

    # Check that no yak directories appear at root (like claim/)
    # The bug we're preventing had directories like "claim/" at root instead of ".yaks/claim/"
    When call sh -c "cd '$REPO' && find . -maxdepth 1 -type d ! -name . ! -name .git ! -name .yaks ! -name '.*' | wc -l | tr -d ' '"
    The output should equal "0"

    rm -rf "$ORIGIN" "$REPO"
  End
End
