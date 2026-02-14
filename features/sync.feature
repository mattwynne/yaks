@fullstack
Feature: yx sync - Collaborate on Yaks via Git

  Synchronizes yaks between team members using a hidden git ref
  (`refs/notes/yaks`).

  Yaks are stored in a hidden git ref (`refs/notes/yaks`) that does
  not appear in branch history. Sync fetches from origin, commits
  local yak state, merges remote changes (fast-forward when possible,
  true merge if both sides changed), pushes, and extracts the merged
  result. Conflict resolution uses last-write-wins.

  Background:
    Given a bare git repository called "origin"

  Rule: Pushing yaks to origin

    Example: Syncing pushes the yaks ref to origin
      Given a git clone of "origin" called "user1"
      And "user1" has a yak called "test yak"
      When "user1" syncs yaks
      Then "origin" has a "refs/notes/yaks" ref

  Rule: Pulling yaks from origin

    Example: Syncing pulls yaks added by another user
      Given a git clone of "origin" called "user1"
      And a git clone of "origin" called "user2"
      And "user1" has a yak called "shared yak"
      And "user1" has synced yaks
      When "user2" syncs yaks
      Then "user2" should have a yak called "shared yak"

  Rule: Merging yaks from multiple users

    Example: Both users' yaks are present after syncing
      Given a git clone of "origin" called "user1"
      And a git clone of "origin" called "user2"
      And "user1" has a yak called "user1 yak"
      And "user1" has synced yaks
      And "user2" has a yak called "user2 yak"
      And "user2" has synced yaks
      When "user1" syncs yaks
      Then "user1" should have a yak called "user1 yak"
      And "user1" should have a yak called "user2 yak"

    Example: Local yaks are preserved when syncing with new remote yaks
      Given a git clone of "origin" called "user1"
      And a git clone of "origin" called "user2"
      And "user1" has a yak called "yak-a"
      And "user1" has synced yaks
      And "user2" has synced yaks
      And "user1" has a yak called "yak-b"
      And "user1" has synced yaks
      And "user2" has a yak called "yak-c"
      When "user2" syncs yaks
      Then "user2" should have a yak called "yak-a"
      And "user2" should have a yak called "yak-b"
      And "user2" should have a yak called "yak-c"

  Rule: Sync does not pollute working tree or index

    Example: Git index remains clean after sync
      Given a git clone of "origin" called "user1"
      And "user1" has a yak called "test yak"
      When "user1" syncs yaks
      Then "user1" has nothing staged in the git index

  Rule: Sync works across git worktrees

    Example: Yaks sync between worktrees of the same repository
      Given a git clone of "origin" called "main-checkout"
      And a git worktree of "main-checkout" called "worktree-a"
      And a git worktree of "main-checkout" called "worktree-b"
      And "worktree-a" has a yak called "shared yak"
      And "worktree-a" has synced yaks
      When "worktree-b" syncs yaks
      Then "worktree-b" should have a yak called "shared yak"
