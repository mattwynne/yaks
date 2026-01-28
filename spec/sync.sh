Describe 'yx sync'
  setup_repos() {
    # Create origin repo
    ORIGIN=$(mktemp -d)
    git -C "$ORIGIN" init --bare --quiet

    # Create user1 repo
    USER1=$(mktemp -d)
    setup_test_repo "$USER1" "user1@example.com" "User 1" "$ORIGIN"
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
    YAKS_PATH="$USER1/.yaks" "yx" add "test yak"
    sh -c "cd '$USER1' && YAKS_PATH='$USER1/.yaks' yx sync" 2>&1

    # Check that refs/notes/yaks exists in origin
    When call git -C "$ORIGIN" show-ref refs/notes/yaks
    The status should be success
    The stdout should be present
  End

  It 'pulls yaks from origin'
    # User1 adds a yak and syncs
    YAKS_PATH="$USER1/.yaks" "yx" add "shared yak"
    sh -c "cd '$USER1' && YAKS_PATH='$USER1/.yaks' yx sync" 2>&1

    # User2 syncs and should get the yak
    sh -c "cd '$USER2' && YAKS_PATH='$USER2/.yaks' yx sync" 2>&1

    When call sh -c "YAKS_PATH='$USER2/.yaks' yx ls"
    The output should include "shared yak"
  End

  It 'merges yaks from multiple users'
    # User1 adds a yak
    YAKS_PATH="$USER1/.yaks" "yx" add "user1 yak"
    sh -c "cd '$USER1' && YAKS_PATH='$USER1/.yaks' yx sync" 2>&1

    # User2 adds a different yak
    YAKS_PATH="$USER2/.yaks" "yx" add "user2 yak"
    sh -c "cd '$USER2' && YAKS_PATH='$USER2/.yaks' yx sync" 2>&1

    # User1 syncs again and should have both
    sh -c "cd '$USER1' && YAKS_PATH='$USER1/.yaks' yx sync" 2>&1

    When call sh -c "YAKS_PATH='$USER1/.yaks' yx ls"
    The output should include "user1 yak"
    The output should include "user2 yak"
  End

  It 'syncs done --undo operations correctly'
    # User1 adds and marks a yak as done, then syncs
    YAKS_PATH="$USER1/.yaks" "yx" add "test yak"
    YAKS_PATH="$USER1/.yaks" "yx" done "test yak"
    sh -c "cd '$USER1' && YAKS_PATH='$USER1/.yaks' yx sync" 2>&1

    # User2 syncs and should see it as done
    sh -c "cd '$USER2' && YAKS_PATH='$USER2/.yaks' yx sync" 2>&1
    result1=$(sh -c "YAKS_PATH='$USER2/.yaks' yx ls")
    echo "$result1" | grep -q "\[x\] test yak" || exit 1

    # User1 undoes it and syncs again
    YAKS_PATH="$USER1/.yaks" "yx" done --undo "test yak"
    sh -c "cd '$USER1' && YAKS_PATH='$USER1/.yaks' yx sync" 2>&1

    # User2 syncs and should now see it as todo
    sh -c "cd '$USER2' && YAKS_PATH='$USER2/.yaks' yx sync" 2>&1

    When call sh -c "YAKS_PATH='$USER2/.yaks' yx ls"
    The output should include "[ ] test yak"
  End

  It 'handles concurrent modifications to same yak'
    # User1 adds a yak and syncs
    YAKS_PATH="$USER1/.yaks" "yx" add "shared yak"
    sh -c "cd '$USER1' && YAKS_PATH='$USER1/.yaks' yx sync" 2>&1

    # User2 syncs to get the yak
    sh -c "cd '$USER2' && YAKS_PATH='$USER2/.yaks' yx sync" 2>&1

    # User1 marks it done (without syncing yet)
    YAKS_PATH="$USER1/.yaks" "yx" done "shared yak"

    # User2 adds context to same yak (without syncing yet)
    echo "User2 context" | YAKS_PATH="$USER2/.yaks" "yx" context "shared yak"

    # User1 syncs first
    sh -c "cd '$USER1' && YAKS_PATH='$USER1/.yaks' yx sync" 2>&1

    # User2 syncs - should trigger merge
    When call sh -c "cd '$USER2' && YAKS_PATH='$USER2/.yaks' yx sync 2>&1"
    The status should be success
  End

  It 'resolves concurrent edits with last-write-wins'
    # User1 adds a yak and syncs
    YAKS_PATH="$USER1/.yaks" "yx" add "conflict yak"
    sh -c "cd '$USER1' && YAKS_PATH='$USER1/.yaks' yx sync" 2>&1

    # User2 syncs to get the yak
    sh -c "cd '$USER2' && YAKS_PATH='$USER2/.yaks' yx sync" 2>&1

    # User1 marks done and syncs
    YAKS_PATH="$USER1/.yaks" "yx" done "conflict yak"
    sh -c "cd '$USER1' && YAKS_PATH='$USER1/.yaks' yx sync" 2>&1

    # User2 also marks done (creating same state)
    YAKS_PATH="$USER2/.yaks" "yx" done "conflict yak"

    # User2 syncs - should handle duplicate done gracefully
    sh -c "cd '$USER2' && YAKS_PATH='$USER2/.yaks' yx sync" 2>&1

    # Both should show done
    result1=$(sh -c "YAKS_PATH='$USER1/.yaks' yx ls")
    result2=$(sh -c "YAKS_PATH='$USER2/.yaks' yx ls")

    When call echo "$result1"
    The output should include "[x] conflict yak"
  End

  It 'does not restore pruned yaks after sync'
    # User1 adds some yaks and marks them done
    sh -c "cd '$USER1' && YAKS_PATH='$USER1/.yaks' yx add 'done yak 1'"
    sh -c "cd '$USER1' && YAKS_PATH='$USER1/.yaks' yx add 'done yak 2'"
    sh -c "cd '$USER1' && YAKS_PATH='$USER1/.yaks' yx add 'active yak'"
    sh -c "cd '$USER1' && YAKS_PATH='$USER1/.yaks' yx done 'done yak 1'"
    sh -c "cd '$USER1' && YAKS_PATH='$USER1/.yaks' yx done 'done yak 2'"

    # User1 syncs to push to origin
    sh -c "cd '$USER1' && YAKS_PATH='$USER1/.yaks' yx sync" 2>&1

    # Verify done yaks are present
    result1=$(sh -c "cd '$USER1' && YAKS_PATH='$USER1/.yaks' yx ls")
    echo "$result1" | grep -q "done yak 1" || exit 1
    echo "$result1" | grep -q "done yak 2" || exit 1

    # User1 prunes done yaks
    sh -c "cd '$USER1' && YAKS_PATH='$USER1/.yaks' yx prune"

    # Verify done yaks are gone
    result2=$(sh -c "cd '$USER1' && YAKS_PATH='$USER1/.yaks' yx ls")
    echo "$result2" | grep -q "done yak 1" && exit 1
    echo "$result2" | grep -q "done yak 2" && exit 1
    echo "$result2" | grep -q "active yak" || exit 1

    # User1 syncs again - done yaks should NOT come back
    sh -c "cd '$USER1' && YAKS_PATH='$USER1/.yaks' yx sync" 2>&1

    When call sh -c "cd '$USER1' && YAKS_PATH='$USER1/.yaks' yx ls"
    The output should not include "done yak 1"
    The output should not include "done yak 2"
    The output should include "active yak"
  End
End
