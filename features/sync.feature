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
  Rule: Pushing yaks to origin

    Example: Syncing pushes the yaks ref to origin
      Given a git clone of origin called alice
      And alice has a yak called "test yak"
      When alice syncs yaks
      Then origin has a "refs/notes/yaks" ref

  @fullstack
  Rule: Pulling yaks from origin

    Example: Syncing pulls yaks added by another user
      Given a git clone of origin called alice
      And a git clone of origin called bob
      And alice has a yak called "shared yak"
      And alice has synced yaks
      When bob syncs yaks
      Then bob should have a yak called "shared yak"

  @fullstack
  Rule: Merging yaks from multiple users

    Example: Both users' yaks are present after syncing
      Given a git clone of origin called alice
      And a git clone of origin called bob
      And alice has a yak called "alice yak"
      And alice has synced yaks
      And bob has a yak called "bob yak"
      And bob has synced yaks
      When alice syncs yaks
      Then alice should have a yak called "alice yak"
      And alice should have a yak called "bob yak"

    Example: Local yaks are preserved when syncing with new remote yaks
      Given a git clone of origin called alice
      And a git clone of origin called bob
      And alice has a yak called "yak-a"
      And alice has synced yaks
      And bob has synced yaks
      And alice has a yak called "yak-b"
      And alice has synced yaks
      And bob has a yak called "yak-c"
      When bob syncs yaks
      Then bob should have a yak called "yak-a"
      And bob should have a yak called "yak-b"
      And bob should have a yak called "yak-c"

  @fullstack
  Rule: Sync does not pollute working tree or index

    Example: Git index remains clean after sync
      Given a git clone of origin called alice
      And alice has a yak called "test yak"
      When alice syncs yaks
      Then alice has nothing staged in the git index

  @fullstack
  Rule: Sync works across git worktrees

    Example: Yaks sync between worktrees of the same repository
      Given a git clone of origin called main-checkout
      And a git worktree of main-checkout called worktree-a
      And a git worktree of main-checkout called worktree-b
      And worktree-a has a yak called "shared yak"
      And worktree-a has synced yaks
      When worktree-b syncs yaks
      Then worktree-b should have a yak called "shared yak"

  # ================================================================
  # Scenarios below are from the example map for the EventStore-based
  # sync redesign. Tagged @wip until the implementation is complete.
  # See: yx field --show "root cause: migration creates duplicates on sync" examples
  # ================================================================

  @wip
  Rule: Non-conflicting changes on different yaks are merged

    Example: Local modified context, peer added new yak
      Given a git clone of origin called alice
      And a git clone of origin called bob
      And alice has a yak called "fix login bug"
      And alice has synced yaks
      And bob has synced yaks
      And alice has set the context of "fix login bug" to "root cause found in auth.rs"
      And alice has synced yaks
      And bob has a yak called "add dark mode"
      When bob syncs yaks
      Then bob should have a yak called "fix login bug"
      And bob should have a yak called "add dark mode"

    Example: Local removed yak, peer added new yak
      Given a git clone of origin called alice
      And a git clone of origin called bob
      And alice has a yak called "old migration"
      And alice has a yak called "database index"
      And alice has synced yaks
      And bob has synced yaks
      And alice has removed the yak "old migration"
      And alice has synced yaks
      And bob has a yak called "add caching"
      When bob syncs yaks
      Then bob should not have a yak called "old migration"
      And bob should have a yak called "database index"
      And bob should have a yak called "add caching"

    Example: Local moved yak under parent, peer added new yak
      Given a git clone of origin called alice
      And a git clone of origin called bob
      And alice has a yak called "API redesign"
      And alice has a yak called "rate limiting"
      And alice has synced yaks
      And bob has synced yaks
      And alice has moved the yak "rate limiting" under "API redesign"
      And alice has synced yaks
      And bob has a yak called "update docs"
      When bob syncs yaks
      Then bob should have a yak called "rate limiting"
      And bob should have a yak called "update docs"

    Example: Local changed state, peer changed different yak's context
      Given a git clone of origin called alice
      And a git clone of origin called bob
      And alice has a yak called "fix flaky test"
      And alice has a yak called "refactor parser"
      And alice has synced yaks
      And bob has synced yaks
      And alice has set the state of "fix flaky test" to "wip"
      And alice has synced yaks
      And bob has set the context of "refactor parser" to "split into tokenizer and evaluator"
      When bob syncs yaks
      Then bob yak "fix flaky test" should have state "wip"

  @wip
  Rule: Same yak modified on different fields - sync merges per-field

    Example: Local changed state, peer changed context on same yak
      Given a git clone of origin called alice
      And a git clone of origin called bob
      And alice has a yak called "upgrade dependencies"
      And alice has synced yaks
      And bob has synced yaks
      And alice has set the state of "upgrade dependencies" to "wip"
      And alice has synced yaks
      And bob has set the context of "upgrade dependencies" to "blocked on serde 2.0 release"
      When bob syncs yaks
      Then bob yak "upgrade dependencies" should have state "wip"
      And bob yak "upgrade dependencies" should have context "blocked on serde 2.0 release"

    Example: Local set custom field, peer changed state on same yak
      Given a git clone of origin called alice
      And a git clone of origin called bob
      And alice has a yak called "rewrite sync"
      And alice has synced yaks
      And bob has synced yaks
      And alice has set the "plan" field of "rewrite sync" to "step 1: add event_id"
      And alice has synced yaks
      And bob has set the state of "rewrite sync" to "wip"
      When bob syncs yaks
      Then bob yak "rewrite sync" should have state "wip"

  @wip
  Rule: Same field on same yak - last-write-wins by timestamp

    Example: Peer's more recent state change wins
      Given a git clone of origin called alice
      And a git clone of origin called bob
      And alice has a yak called "deploy pipeline"
      And alice has synced yaks
      And bob has synced yaks
      And alice has set the state of "deploy pipeline" to "wip"
      And alice has synced yaks
      And bob has set the state of "deploy pipeline" to "done"
      When bob syncs yaks
      Then bob yak "deploy pipeline" should have state "done"
      When alice syncs yaks
      Then alice yak "deploy pipeline" should have state "done"

    Example: Local's more recent context change wins
      Given a git clone of origin called alice
      And a git clone of origin called bob
      And alice has a yak called "security audit"
      And alice has synced yaks
      And bob has synced yaks
      And bob has set the context of "security audit" to "initial scan results"
      And bob has synced yaks
      And alice has set the context of "security audit" to "CVE-2026-1234 found in parser"
      When alice syncs yaks
      Then alice yak "security audit" should have context "CVE-2026-1234 found in parser"
      When bob syncs yaks
      Then bob yak "security audit" should have context "CVE-2026-1234 found in parser"

  @wip
  Rule: Events that target removed yaks are discarded

    Example: Remote removes yak, local modifies it - modify discarded
      Given a git clone of origin called alice
      And a git clone of origin called bob
      And alice has a yak called "legacy endpoint"
      And alice has synced yaks
      And bob has synced yaks
      And alice has removed the yak "legacy endpoint"
      And alice has synced yaks
      And bob has set the context of "legacy endpoint" to "too late"
      When bob syncs yaks
      Then bob should not have a yak called "legacy endpoint"

    Example: Remote removes yak, local moves another under it - move discarded
      Given a git clone of origin called alice
      And a git clone of origin called bob
      And alice has a yak called "old framework"
      And alice has a yak called "migration script"
      And alice has synced yaks
      And bob has synced yaks
      And alice has removed the yak "old framework"
      And alice has synced yaks
      And bob has moved the yak "migration script" under "old framework"
      When bob syncs yaks
      Then bob should not have a yak called "old framework"
      And bob should have a yak called "migration script"

    Example: Both sides remove the same yak
      Given a git clone of origin called alice
      And a git clone of origin called bob
      And alice has a yak called "dead code"
      And alice has synced yaks
      And bob has synced yaks
      And alice has removed the yak "dead code"
      And alice has synced yaks
      And bob has removed the yak "dead code"
      When bob syncs yaks
      Then bob should not have a yak called "dead code"

  @wip
  Rule: Sync logs each event through the output port

    Example: Pulled events are logged
      Given a git clone of origin called alice
      And a git clone of origin called bob
      And alice has a yak called "fix memory leak"
      And alice has synced yaks
      When bob syncs yaks
      Then the output should include "fix memory leak"

    Example: Discarded events are logged with reason
      Given a git clone of origin called alice
      And a git clone of origin called bob
      And alice has a yak called "sunset v1 API"
      And alice has synced yaks
      And bob has synced yaks
      And alice has removed the yak "sunset v1 API"
      And alice has synced yaks
      And bob has set the context of "sunset v1 API" to "too late"
      When bob syncs yaks
      Then the output should include "discarded"
