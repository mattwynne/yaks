Feature: Git worktree compatibility

  Background:
    Given a bare git repository called origin

  @fullstack
  Rule: Worktree projections are disposable

    Example: A worktree can be removed after yx creates its projection
      Given a git clone of origin called main
      And a git worktree of main called feature
      And feature has a yak called "make the tea"
      Then main can remove the worktree feature

    Example: A legacy projection is rebuilt before use
      Given a git clone of origin called main
      And a git worktree of main called feature
      And feature has a yak called "make the tea"
      And feature has a legacy read-only projection
      When feature lists its yaks
      Then main can remove the worktree feature
