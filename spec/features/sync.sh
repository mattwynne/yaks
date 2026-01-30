# shellcheck shell=bash
# shellcheck disable=SC1010,SC2034
Describe 'yx sync'
  setup_repos() {
    # Create origin repo
    ORIGIN=$(mktemp -d)
    setup_bare_repo "$ORIGIN"

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
    GIT_WORK_TREE="$USER1" "yx" add "test yak"
    sh -c "cd '$USER1' && GIT_WORK_TREE='$USER1' yx sync" 2>&1

    # Check that refs/notes/yaks exists in origin
    When call git -C "$ORIGIN" show-ref refs/notes/yaks
    The status should be success
    The stdout should be present
  End

  It 'pulls yaks from origin'
    # User1 adds a yak and syncs
    GIT_WORK_TREE="$USER1" "yx" add "shared yak"
    sh -c "cd '$USER1' && GIT_WORK_TREE='$USER1' yx sync" 2>&1

    # User2 syncs and should get the yak
    sh -c "cd '$USER2' && GIT_WORK_TREE='$USER2' yx sync" 2>&1

    When call sh -c "GIT_WORK_TREE='$USER2' yx ls"
    The output should include "shared yak"
  End

  It 'merges yaks from multiple users'
    # User1 adds a yak
    GIT_WORK_TREE="$USER1" "yx" add "user1 yak"
    sh -c "cd '$USER1' && GIT_WORK_TREE='$USER1' yx sync" 2>&1

    # User2 adds a different yak
    GIT_WORK_TREE="$USER2" "yx" add "user2 yak"
    sh -c "cd '$USER2' && GIT_WORK_TREE='$USER2' yx sync" 2>&1

    # User1 syncs again and should have both
    sh -c "cd '$USER1' && GIT_WORK_TREE='$USER1' yx sync" 2>&1

    When call sh -c "GIT_WORK_TREE='$USER1' yx ls"
    The output should include "user1 yak"
    The output should include "user2 yak"
  End

  It 'syncs done --undo operations correctly'
    # User1 adds and marks a yak as done, then syncs
    GIT_WORK_TREE="$USER1" "yx" add "test yak"
    GIT_WORK_TREE="$USER1" "yx" done "test yak"
    sh -c "cd '$USER1' && GIT_WORK_TREE='$USER1' yx sync" 2>&1

    # User2 syncs and should see it as done
    sh -c "cd '$USER2' && GIT_WORK_TREE='$USER2' yx sync" 2>&1
    result1=$(sh -c "GIT_WORK_TREE='$USER2' yx ls")
    echo "$result1" | grep -q "\[x\] test yak" || exit 1

    # User1 undoes it and syncs again
    GIT_WORK_TREE="$USER1" "yx" done --undo "test yak"
    sh -c "cd '$USER1' && GIT_WORK_TREE='$USER1' yx sync" 2>&1

    # User2 syncs and should now see it as todo
    sh -c "cd '$USER2' && GIT_WORK_TREE='$USER2' yx sync" 2>&1

    When call sh -c "GIT_WORK_TREE='$USER2' yx ls"
    The output should include "[ ] test yak"
  End

  It 'handles concurrent modifications to same yak'
    # User1 adds a yak and syncs
    GIT_WORK_TREE="$USER1" "yx" add "shared yak"
    sh -c "cd '$USER1' && GIT_WORK_TREE='$USER1' yx sync" 2>&1

    # User2 syncs to get the yak
    sh -c "cd '$USER2' && GIT_WORK_TREE='$USER2' yx sync" 2>&1

    # User1 marks it done (without syncing yet)
    GIT_WORK_TREE="$USER1" "yx" done "shared yak"

    # User2 adds context to same yak (without syncing yet)
    echo "User2 context" | GIT_WORK_TREE="$USER2" "yx" context "shared yak"

    # User1 syncs first
    sh -c "cd '$USER1' && GIT_WORK_TREE='$USER1' yx sync" 2>&1

    # User2 syncs - should trigger merge
    When call sh -c "cd '$USER2' && GIT_WORK_TREE='$USER2' yx sync 2>&1"
    The status should be success
  End

  It 'resolves concurrent edits with last-write-wins'
    # User1 adds a yak and syncs
    GIT_WORK_TREE="$USER1" "yx" add "conflict yak"
    sh -c "cd '$USER1' && GIT_WORK_TREE='$USER1' yx sync" 2>&1

    # User2 syncs to get the yak
    sh -c "cd '$USER2' && GIT_WORK_TREE='$USER2' yx sync" 2>&1

    # User1 marks done and syncs
    GIT_WORK_TREE="$USER1" "yx" done "conflict yak"
    sh -c "cd '$USER1' && GIT_WORK_TREE='$USER1' yx sync" 2>&1

    # User2 also marks done (creating same state)
    GIT_WORK_TREE="$USER2" "yx" done "conflict yak"

    # User2 syncs - should handle duplicate done gracefully
    sh -c "cd '$USER2' && GIT_WORK_TREE='$USER2' yx sync" 2>&1

    # Both should show done
    result1=$(sh -c "GIT_WORK_TREE='$USER1' yx ls")
    result2=$(sh -c "GIT_WORK_TREE='$USER2' yx ls")

    When call echo "$result1"
    The output should include "[x] conflict yak"
  End

  It 'does not restore pruned yaks after sync with divergent remote'
    # Setup: Both users add yaks
    sh -c "cd '$USER1' && GIT_WORK_TREE='$USER1' yx add 'done yak'"
    sh -c "cd '$USER1' && GIT_WORK_TREE='$USER1' yx done 'done yak'"
    sh -c "cd '$USER1' && GIT_WORK_TREE='$USER1' yx sync" 2>&1

    # User2 syncs to get the done yak
    sh -c "cd '$USER2' && GIT_WORK_TREE='$USER2' yx sync" 2>&1

    # User1 prunes locally (removes done yak from working dir and creates new commit)
    sh -c "cd '$USER1' && GIT_WORK_TREE='$USER1' yx prune"

    # Meanwhile, User2 adds a different yak and syncs (creates divergence)
    sh -c "cd '$USER2' && GIT_WORK_TREE='$USER2' yx add 'user2 yak'"
    sh -c "cd '$USER2' && GIT_WORK_TREE='$USER2' yx sync" 2>&1

    # Now User1 syncs - this should merge but NOT bring back done yak
    sh -c "cd '$USER1' && GIT_WORK_TREE='$USER1' yx sync" 2>&1

    When call sh -c "cd '$USER1' && GIT_WORK_TREE='$USER1' yx ls"
    The output should not include "done yak"
    The output should include "user2 yak"
  End
End
