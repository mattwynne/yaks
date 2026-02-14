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
    echo "" | GIT_WORK_TREE="$USER1" "yx" add "test yak"
    sh -c "cd '$USER1' && GIT_WORK_TREE='$USER1' yx sync" 2>&1

    # Check that refs/notes/yaks exists in origin
    When call git -C "$ORIGIN" show-ref refs/notes/yaks
    The status should be success
    The stdout should be present
  End

  It 'pulls yaks from origin'
    # User1 adds a yak and syncs
    echo "" | GIT_WORK_TREE="$USER1" "yx" add "shared yak"
    sh -c "cd '$USER1' && GIT_WORK_TREE='$USER1' yx sync" 2>&1

    # User2 syncs and should get the yak
    sh -c "cd '$USER2' && GIT_WORK_TREE='$USER2' yx sync" 2>&1

    When call sh -c "GIT_WORK_TREE='$USER2' yx ls --format markdown"
    The output should include "shared yak"
  End

  It 'merges yaks from multiple users'
    # User1 adds a yak
    echo "" | GIT_WORK_TREE="$USER1" "yx" add "user1 yak"
    sh -c "cd '$USER1' && GIT_WORK_TREE='$USER1' yx sync" 2>&1

    # User2 adds a different yak
    echo "" | GIT_WORK_TREE="$USER2" "yx" add "user2 yak"
    sh -c "cd '$USER2' && GIT_WORK_TREE='$USER2' yx sync" 2>&1

    # User1 syncs again and should have both
    sh -c "cd '$USER1' && GIT_WORK_TREE='$USER1' yx sync" 2>&1

    When call sh -c "GIT_WORK_TREE='$USER1' yx ls --format markdown"
    The output should include "user1 yak"
    The output should include "user2 yak"
  End
End
