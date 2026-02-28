Feature: yx sync - Collaborate on Yaks via Git
  Synchronizes yaks between team members using a hidden git ref
  (`refs/notes/yaks`). Idempotent and safe to run anytime.

  Yaks are stored in a hidden git ref (`refs/notes/yaks`) that does
  not appear in branch history. Sync fetches from origin, commits
  local yak state, merges remote changes (fast-forward when possible,
  true merge if both sides changed), pushes, and extracts the merged
  result. Conflict resolution uses last-write-wins. When there is no
  remote origin, sync succeeds silently as a no-op.

  Background:
    Given a bare git repository called origin

  @fullstack
  Rule: Syncing stores yaks on the remote

    Example: Syncing pushes the yaks ref to origin
      Given a git clone of origin called alice
      And alice has a yak called "make the tea"
      When alice syncs yaks
      Then origin has a "refs/notes/yaks" ref

  Rule: After syncing, all users have the same yaks

    Example: Bob gets alice's yak after sync
      Given a git clone of origin called alice
      And a git clone of origin called bob
      And alice has a yak called "make the tea"
      And alice has synced yaks
      When bob syncs yaks
      Then bob should have a yak called "make the tea"

    Example: Both users see the same yaks after syncing
      Given a git clone of origin called alice
      And a git clone of origin called bob
      And alice has a yak called "buy biscuits"
      And alice has synced yaks
      And bob has a yak called "make the tea"
      And bob has synced yaks
      When alice syncs yaks
      Then alice and bob both have the same yaks:
        """
        ○ buy biscuits
         ○ make the tea
        """

  @fullstack
  Rule: Worktrees of the same repository can sync independently

    Example: Yak created on a feature branch appears on main after sync
      Given a git clone of origin called main
      And a git worktree of main called feature
      And a git worktree of main called bugfix
      And feature has a yak called "make the tea"
      And feature has synced yaks
      When bugfix syncs yaks
      Then bugfix should have a yak called "make the tea"

  Rule: Removals propagate through sync

    Example: Alice's removal appears on bob's side
      Given a git clone of origin called alice
      And a git clone of origin called bob
      And alice has a yak called "make the tea"
      And alice has synced yaks
      And bob has synced yaks
      And alice has removed the yak "make the tea"
      And alice has synced yaks
      When bob syncs yaks
      Then bob should not have a yak called "make the tea"

    Example: Bob removes alice's yak
      Given a git clone of origin called alice
      And a git clone of origin called bob
      And alice has a yak called "make the tea"
      And alice has synced yaks
      And bob has synced yaks
      And bob has removed the yak "make the tea"
      And bob has synced yaks
      When alice syncs yaks
      Then alice should not have a yak called "make the tea"

  Rule: Moves propagate through sync

    Example: Alice's move appears on bob's side
      Given a git clone of origin called alice
      And a git clone of origin called bob
      And alice has a yak called "make the tea"
      And alice has a yak called "buy biscuits"
      And alice has synced yaks
      And bob has synced yaks
      And alice has moved the yak "buy biscuits" under "make the tea"
      And alice has synced yaks
      When bob syncs yaks
      Then bob should have these yaks:
        """
        ○ make the tea
         ╰─ ○ buy biscuits
        """

  Rule: Field changes propagate through sync

    Example: State change propagates
      Given a git clone of origin called alice
      And a git clone of origin called bob
      And alice has a yak called "make the tea"
      And alice has synced yaks
      And bob has synced yaks
      And alice has set the state of "make the tea" to "wip"
      And alice has synced yaks
      When bob syncs yaks
      Then bob yak "make the tea" should have state "wip"

    Example: Context change propagates
      Given a git clone of origin called alice
      And a git clone of origin called bob
      And alice has a yak called "make the tea"
      And alice has synced yaks
      And bob has synced yaks
      And alice has set the context of "make the tea" to "use the good teapot"
      And alice has synced yaks
      When bob syncs yaks
      Then bob yak "make the tea" should have context "use the good teapot"

  Rule: Changes to different fields on the same yak are merged

    Example: Alice changes state, bob changes context
      Given a git clone of origin called alice
      And a git clone of origin called bob
      And alice has a yak called "make the tea"
      And alice has set the state of "make the tea" to "wip"
      And alice has synced yaks
      And bob has synced yaks
      And bob has set the context of "make the tea" to "use the good teapot"
      When bob syncs yaks
      Then bob yak "make the tea" should have state "wip"
      And bob yak "make the tea" should have context "use the good teapot"

  Rule: When both users change the same field, the latest change wins

    Example: Bob's later state change wins
      Given a git clone of origin called alice
      And a git clone of origin called bob
      And alice has a yak called "make the tea"
      And alice has set the state of "make the tea" to "wip"
      And alice has synced yaks
      And bob has synced yaks
      And bob has set the state of "make the tea" to "done"
      And bob has synced yaks
      When alice syncs yaks
      Then alice yak "make the tea" should have state "done"

  Rule: Removal wins over other changes to the same yak

    Example: Bob's edit is lost when alice has removed the yak
      Given a git clone of origin called alice
      And a git clone of origin called bob
      And alice has a yak called "make the tea"
      And alice has synced yaks
      And bob has synced yaks
      And alice has removed the yak "make the tea"
      And alice has synced yaks
      And bob has set the context of "make the tea" to "too late"
      When bob syncs yaks
      Then bob should not have a yak called "make the tea"

  Rule: Removing a yak doesn't affect unrelated yaks during sync

    Example: Bob's changes to "wash the cups" survive syncing a removed "make the tea"
      Given a git clone of origin called alice
      And a git clone of origin called bob
      And alice has a yak called "make the tea"
      And alice has a yak called "wash the cups"
      And alice has synced yaks
      And bob has synced yaks
      And bob has removed the yak "make the tea"
      And bob has set the state of "wash the cups" to "done"
      And alice has set the state of "make the tea" to "wip"
      And alice has synced yaks
      When bob syncs yaks
      Then bob should not have a yak called "make the tea"
      And bob yak "wash the cups" should have state "done"

    Example: Bob's changes to "wash the cups" survive syncing a move under removed "make the tea"
      Given a git clone of origin called alice
      And a git clone of origin called bob
      And alice has a yak called "make the tea"
      And alice has a yak called "buy biscuits"
      And alice has a yak called "wash the cups"
      And alice has synced yaks
      And bob has synced yaks
      And bob has removed the yak "make the tea"
      And bob has set the state of "wash the cups" to "done"
      And alice has moved the yak "buy biscuits" under "make the tea"
      And alice has synced yaks
      When bob syncs yaks
      Then bob should not have a yak called "make the tea"
      And bob yak "wash the cups" should have state "done"

  Rule: A move to a previously-removed parent fails

    Example: Bob's move is discarded but the child survives
      Given a git clone of origin called alice
      And a git clone of origin called bob
      And alice has a yak called "make the tea"
      And alice has a yak called "buy biscuits"
      And alice has synced yaks
      And bob has synced yaks
      And alice has removed the yak "make the tea"
      And alice has synced yaks
      And bob has moved the yak "buy biscuits" under "make the tea"
      When bob syncs yaks
      Then bob should not have a yak called "make the tea"
      And bob should have a yak called "buy biscuits"

  Rule: Concurrent removals don't conflict

    Example: Both users remove the same yak
      Given a git clone of origin called alice
      And a git clone of origin called bob
      And alice has a yak called "make the tea"
      And alice has synced yaks
      And bob has synced yaks
      And alice has removed the yak "make the tea"
      And alice has synced yaks
      And bob has removed the yak "make the tea"
      When bob syncs yaks
      Then bob should not have a yak called "make the tea"

  @fullstack
  Rule: Sync works with non-local remote URLs

    Example: Sync works when origin uses a file:// URL
      Given a git clone of origin via file URL called alice
      And a git clone of origin via file URL called bob
      And alice has a yak called "make the tea"
      And alice has synced yaks
      When bob syncs yaks
      Then bob should have a yak called "make the tea"

  @fullstack
  Rule: Non-sync commands don't contact the remote

    Example: Adding a yak works when origin is unreachable
      Given a git clone of origin via file URL called alice
      And alice's origin remote is unreachable
      When alice adds a yak called "make the tea"
      Then alice should have a yak called "make the tea"

  Rule: Concurrent changes by different users converge to the same state

    Example: Alice and bob both make changes and converge after syncing
      Given a git clone of origin called alice
      And a git clone of origin called bob
      And alice has a yak called "make the tea"
      And alice has synced yaks
      And bob has synced yaks
      And alice has set the state of "make the tea" to "wip"
      And bob has set the context of "make the tea" to "use the good teapot"
      And bob has synced yaks
      And alice has synced yaks
      When bob syncs yaks
      Then alice yak "make the tea" should have state "wip"
      And alice yak "make the tea" should have context "use the good teapot"
      And bob yak "make the tea" should have state "wip"
      And bob yak "make the tea" should have context "use the good teapot"

    Example: Both users add different yaks concurrently
      Given a git clone of origin called alice
      And a git clone of origin called bob
      And alice has a yak called "make the tea"
      And bob has a yak called "buy biscuits"
      And alice has synced yaks
      And bob has synced yaks
      When alice syncs yaks
      Then alice and bob both have the same yaks:
        """
        ○ buy biscuits
         ○ make the tea
        """

  @fullstack
  Rule: Sync reports when a compaction is received

    Example: Bob sees compaction message after Alice compacts
      Given a git clone of origin called alice
      And a git clone of origin called bob
      And alice has a yak called "make the tea"
      And alice has synced yaks
      And alice has compacted yaks
      And alice has synced yaks
      When bob syncs yaks
      Then the output should include "compaction"

  Rule: Unsynced events survive compaction by another peer

    Example: Bob's unsynced yak survives Alice's compaction
      Given a git clone of origin called alice
      And a git clone of origin called bob
      And alice has a yak called "make the tea"
      And alice has synced yaks
      And bob has synced yaks
      And bob has a yak called "buy biscuits"
      And alice has compacted yaks
      And alice has synced yaks
      When bob syncs yaks
      Then bob should have a yak called "make the tea"
      And bob should have a yak called "buy biscuits"
      When alice syncs yaks
      Then alice should have a yak called "make the tea"
      And alice should have a yak called "buy biscuits"

  Rule: Sync refuses when the remote uses a newer schema version

    Example: Bob's outdated binary refuses to sync Alice's newer schema
      Given a git clone of origin called alice
      And a git clone of origin called bob
      And alice has a yak called "make the tea"
      And alice has synced yaks
      And bob has synced yaks
      And origin has been migrated beyond the current schema version
      When bob tries to sync yaks
      Then the command should fail
      And the error should contain "Please update yx"
      And bob should have a yak called "make the tea"

  @wip
  Rule: Sync tells you what changed

    Example: Bob sees what was pulled and what was discarded
      Given a git clone of origin called alice
      And a git clone of origin called bob
      And alice has a yak called "make the tea"
      And alice has synced yaks
      And bob has synced yaks
      And alice has removed the yak "make the tea"
      And alice has a yak called "wash the cups"
      And alice has synced yaks
      And bob has set the context of "make the tea" to "too late"
      When bob syncs yaks
      Then bob's sync output should be:
        """
        Replaying 3 events since last sync:
        ✓ Removed: make the tea (alice)
        ✓ Added: wash the cups (alice)
        ✗ SetField: make the tea (bob)
        """
