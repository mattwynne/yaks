# shellcheck shell=bash
# shellcheck disable=SC1010,SC2034
Describe 'yx sync - handling divergence scenarios'
  setup_repos() {
    # Create origin repo
    ORIGIN=$(mktemp -d)
    setup_bare_repo "$ORIGIN"

    # Create clone1 repo
    CLONE1=$(mktemp -d)
    setup_test_repo "$CLONE1" "user1@example.com" "User 1" "$ORIGIN"
    echo "# Test Repo" > "$CLONE1/README.md"
    git -C "$CLONE1" add README.md
    git -C "$CLONE1" commit -m "Initial commit" --quiet
    git -C "$CLONE1" push -u origin main --quiet

    # Create clone2 repo (clone of origin)
    CLONE2=$(mktemp -d)
    git clone --quiet "$ORIGIN" "$CLONE2"
    git -C "$CLONE2" config user.email "user2@example.com"
    git -C "$CLONE2" config user.name "User 2"
  }

  cleanup_repos() {
    rm -rf "$ORIGIN" "$CLONE1" "$CLONE2"
  }

  BeforeEach 'setup_repos'
  AfterEach 'cleanup_repos'

  It 'preserves local yaks when syncing with remote that has different yaks'
    # Clone1 adds yak-a and syncs
    echo "" | GIT_WORK_TREE="$CLONE1" "yx" add "yak-a"
    sh -c "cd '$CLONE1' && GIT_WORK_TREE='$CLONE1' yx sync" 2>&1

    # Clone2 syncs to get yak-a
    sh -c "cd '$CLONE2' && GIT_WORK_TREE='$CLONE2' yx sync" 2>&1

    # Clone1 adds yak-b and syncs
    echo "" | GIT_WORK_TREE="$CLONE1" "yx" add "yak-b"
    sh -c "cd '$CLONE1' && GIT_WORK_TREE='$CLONE1' yx sync" 2>&1

    # Clone2 adds yak-c locally (before syncing)
    echo "" | GIT_WORK_TREE="$CLONE2" "yx" add "yak-c"

    # Clone2 syncs - should keep local yak-c AND get remote yak-b
    sh -c "cd '$CLONE2' && GIT_WORK_TREE='$CLONE2' yx sync" 2>&1

    result2=$(sh -c "GIT_WORK_TREE='$CLONE2' yx ls --format markdown")

    When call echo "$result2"
    The output should include "yak-a"
    The output should include "yak-b"
    The output should include "yak-c"
  End
End
