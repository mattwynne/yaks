Describe 'yx sync'
  setup_repos() {
    # Get absolute path to yx command (shellspec runs from project root)
    YX_BIN="$(pwd)/bin/yx"

    # Create origin repo
    ORIGIN=$(mktemp -d)
    git -C "$ORIGIN" init --bare --quiet

    # Create user1 repo
    USER1=$(mktemp -d)
    git -C "$USER1" init --quiet
    git -C "$USER1" remote add origin "$ORIGIN"
    git -C "$USER1" config user.email "user1@example.com"
    git -C "$USER1" config user.name "User 1"
    echo "# Test Repo" > "$USER1/README.md"
    git -C "$USER1" add README.md
    git -C "$USER1" commit -m "Initial commit" --quiet
    git -C "$USER1" push -u origin main --quiet

    # Create user2 repo (clone of origin)
    USER2=$(mktemp -d)
    git clone --quiet "$ORIGIN" "$USER2"
    git -C "$USER2" config user.email "user2@example.com"
    git -C "$USER2" config user.name "User 2"
  }

  cleanup_repos() {
    rm -rf "$ORIGIN" "$USER1" "$USER2"
  }

  BeforeEach 'setup_repos'
  AfterEach 'cleanup_repos'

  It 'pushes yaks to origin'
    YAK_PATH="$USER1/.yaks" "$YX_BIN" add "test yak"
    sh -c "cd '$USER1' && YAK_PATH='$USER1/.yaks' '$YX_BIN' sync" 2>&1

    # Check that refs/notes/yaks exists in origin
    When call git -C "$ORIGIN" show-ref refs/notes/yaks
    The status should be success
  End

  It 'pulls yaks from origin'
    # User1 adds a yak and syncs
    YAK_PATH="$USER1/.yaks" "$YX_BIN" add "shared yak"
    sh -c "cd '$USER1' && YAK_PATH='$USER1/.yaks' '$YX_BIN' sync" 2>&1

    # User2 syncs and should get the yak
    sh -c "cd '$USER2' && YAK_PATH='$USER2/.yaks' '$YX_BIN' sync" 2>&1

    When call sh -c "YAK_PATH='$USER2/.yaks' '$YX_BIN' ls"
    The output should include "shared yak"
  End

  It 'merges yaks from multiple users'
    # User1 adds a yak
    YAK_PATH="$USER1/.yaks" "$YX_BIN" add "user1 yak"
    sh -c "cd '$USER1' && YAK_PATH='$USER1/.yaks' '$YX_BIN' sync" 2>&1

    # User2 adds a different yak
    YAK_PATH="$USER2/.yaks" "$YX_BIN" add "user2 yak"
    sh -c "cd '$USER2' && YAK_PATH='$USER2/.yaks' '$YX_BIN' sync" 2>&1

    # User1 syncs again and should have both
    sh -c "cd '$USER1' && YAK_PATH='$USER1/.yaks' '$YX_BIN' sync" 2>&1

    When call sh -c "YAK_PATH='$USER1/.yaks' '$YX_BIN' ls"
    The output should include "user1 yak"
    The output should include "user2 yak"
  End

  It 'syncs done --undo operations correctly'
    # User1 adds and marks a yak as done, then syncs
    YAK_PATH="$USER1/.yaks" "$YX_BIN" add "test yak"
    YAK_PATH="$USER1/.yaks" "$YX_BIN" done "test yak"
    sh -c "cd '$USER1' && YAK_PATH='$USER1/.yaks' '$YX_BIN' sync" 2>&1

    # User2 syncs and should see it as done
    sh -c "cd '$USER2' && YAK_PATH='$USER2/.yaks' '$YX_BIN' sync" 2>&1
    result1=$(sh -c "YAK_PATH='$USER2/.yaks' '$YX_BIN' ls")
    echo "$result1" | grep -q "\[x\] test yak" || exit 1

    # User1 undoes it and syncs again
    YAK_PATH="$USER1/.yaks" "$YX_BIN" done --undo "test yak"
    sh -c "cd '$USER1' && YAK_PATH='$USER1/.yaks' '$YX_BIN' sync" 2>&1

    # User2 syncs and should now see it as todo
    sh -c "cd '$USER2' && YAK_PATH='$USER2/.yaks' '$YX_BIN' sync" 2>&1

    When call sh -c "YAK_PATH='$USER2/.yaks' '$YX_BIN' ls"
    The output should include "[ ] test yak"
  End
End
