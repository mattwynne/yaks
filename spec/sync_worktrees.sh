Describe 'yx sync with git worktrees'
  setup_repos() {
    # Create origin repo
    ORIGIN=$(mktemp -d)
    git -C "$ORIGIN" init --bare --quiet

    # Create main repo
    MAIN=$(mktemp -d)
    git -C "$MAIN" init --quiet
    git -C "$MAIN" remote add origin "$ORIGIN"
    git -C "$MAIN" config user.email "main@example.com"
    git -C "$MAIN" config user.name "Main User"
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
    YAKS_PATH="$WORKTREE_A/.yaks" "yx" add "yak from A"
    sh -c "cd '$WORKTREE_A' && YAKS_PATH='$WORKTREE_A/.yaks' 'yx' sync" 2>&1

    # Sync in worktree B should fetch the yak
    sh -c "cd '$WORKTREE_B' && YAKS_PATH='$WORKTREE_B/.yaks' 'yx' sync" 2>&1

    When call sh -c "YAKS_PATH='$WORKTREE_B/.yaks' 'yx' ls"
    The output should include "yak from A"
  End

  It 'merges yaks from different worktrees'
    # Add different yaks in each worktree
    YAKS_PATH="$WORKTREE_A/.yaks" "yx" add "yak A"
    YAKS_PATH="$WORKTREE_B/.yaks" "yx" add "yak B"

    # Sync worktree A first
    sh -c "cd '$WORKTREE_A' && YAKS_PATH='$WORKTREE_A/.yaks' 'yx' sync" 2>&1

    # Sync worktree B (should merge both yaks)
    sh -c "cd '$WORKTREE_B' && YAKS_PATH='$WORKTREE_B/.yaks' 'yx' sync" 2>&1

    # Both yaks should be in worktree B
    When call sh -c "YAKS_PATH='$WORKTREE_B/.yaks' 'yx' ls"
    The output should include "yak A"
    The output should include "yak B"
  End

  It 'handles concurrent edits with last-write-wins'
    # Add same yak name in both worktrees
    YAKS_PATH="$WORKTREE_A/.yaks" "yx" add "shared yak"
    YAKS_PATH="$WORKTREE_B/.yaks" "yx" add "shared yak"

    # Mark done in A, leave todo in B
    YAKS_PATH="$WORKTREE_A/.yaks" "yx" done "shared yak"

    # Sync A first
    sh -c "cd '$WORKTREE_A' && YAKS_PATH='$WORKTREE_A/.yaks' 'yx' sync" 2>&1

    # Sync B (will overwrite with todo state - last write wins)
    sh -c "cd '$WORKTREE_B' && YAKS_PATH='$WORKTREE_B/.yaks' 'yx' sync" 2>&1

    # Should be todo (B's state won)
    When call sh -c "YAKS_PATH='$WORKTREE_B/.yaks' 'yx' ls"
    The output should include "[ ] shared yak"
  End
End
