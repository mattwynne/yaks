Describe 'yx sync with git worktrees'
  setup_repos() {
    # Create origin repo
    ORIGIN=$(mktemp -d)
    git -C "$ORIGIN" init --bare --quiet

    # Create main repo
    MAIN=$(mktemp -d)
    setup_test_repo "$MAIN" "main@example.com" "Main User" "$ORIGIN"
    echo "# Test Repo" > "$MAIN/README.md"
    git -C "$MAIN" add README.md
    git -C "$MAIN" commit -m "Initial commit" --quiet
    git -C "$MAIN" push -u origin main --quiet

    # Create two worktrees
    WORKTREE_A="$MAIN/.worktrees/worktree-a"
    WORKTREE_B="$MAIN/.worktrees/worktree-b"
    git -C "$MAIN" worktree add "$WORKTREE_A" -b worktree-a --quiet 2>&1
    git -C "$MAIN" worktree add "$WORKTREE_B" -b worktree-b --quiet 2>&1
  }

  cleanup_repos() {
    # Clean up worktrees first
    if [ -d "$MAIN" ]; then
      git -C "$MAIN" worktree remove --force "$WORKTREE_A" 2>/dev/null || true
      git -C "$MAIN" worktree remove --force "$WORKTREE_B" 2>/dev/null || true
    fi
    rm -rf "$ORIGIN" "$MAIN"
  }

  BeforeEach 'setup_repos'
  AfterEach 'cleanup_repos'

  It 'syncs yaks from worktree A to worktree B'
    # Add yak in worktree A and sync
    GIT_PATH="$WORKTREE_A" "yx" add "yak from A"
    sh -c "cd '$WORKTREE_A' && GIT_PATH='$WORKTREE_A' 'yx' sync" 2>&1

    # Sync in worktree B should fetch the yak
    sh -c "cd '$WORKTREE_B' && GIT_PATH='$WORKTREE_B' 'yx' sync" 2>&1

    When call sh -c "GIT_PATH='$WORKTREE_B' 'yx' ls"
    The output should include "yak from A"
  End

  It 'merges yaks from different worktrees'
    # Add different yaks in each worktree
    GIT_PATH="$WORKTREE_A" "yx" add "yak A"
    GIT_PATH="$WORKTREE_B" "yx" add "yak B"

    # Sync worktree A first
    sh -c "cd '$WORKTREE_A' && GIT_PATH='$WORKTREE_A' 'yx' sync" 2>&1

    # Sync worktree B (should merge both yaks)
    sh -c "cd '$WORKTREE_B' && GIT_PATH='$WORKTREE_B' 'yx' sync" 2>&1

    # Both yaks should be in worktree B
    When call sh -c "GIT_PATH='$WORKTREE_B' 'yx' ls"
    The output should include "yak A"
    The output should include "yak B"
  End

  It 'handles concurrent edits with last-write-wins'
    # Add same yak name in both worktrees
    GIT_PATH="$WORKTREE_A" "yx" add "shared yak"
    GIT_PATH="$WORKTREE_B" "yx" add "shared yak"

    # Mark done in A, leave todo in B
    GIT_PATH="$WORKTREE_A" "yx" done "shared yak"

    # Sync A first
    sh -c "cd '$WORKTREE_A' && GIT_PATH='$WORKTREE_A' 'yx' sync" 2>&1

    # Sync B (will overwrite with todo state - last write wins)
    sh -c "cd '$WORKTREE_B' && GIT_PATH='$WORKTREE_B' 'yx' sync" 2>&1

    # Should be todo (B's state won)
    When call sh -c "GIT_PATH='$WORKTREE_B' 'yx' ls"
    The output should include "[ ] shared yak"
  End
End
