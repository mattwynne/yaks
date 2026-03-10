Feature: Git repository safety checks
  yx requires a properly configured git repository to operate.
  These checks run before any command and provide clear error
  messages when the environment is not set up correctly.

  Rule: Must be run inside a git repository

    Example: Running yx outside a git repository
      Given a directory that is not a git repository
      When I try to list the yaks from this directory
      Then the command should fail
      And the error should contain "not in a git repository"

  Rule: The .yaks folder must be gitignored

    @fullstack
    Example: Offer to add .yaks to .gitignore and commit
      Given a git repository without .yaks in .gitignore
      When I interactively run yx add "my-yak" from this directory
      And I accept the offer to add .yaks to .gitignore
      And I accept the offer to commit the change
      Then the command should succeed
      And there should be a yak called "my-yak"
      And .yaks should be in .gitignore
      And the last commit should include .gitignore

    @fullstack
    Example: Non-interactive environment without .yaks gitignored
      Given a git repository without .yaks in .gitignore
      When I non-interactively run yx from this directory
      Then the command should fail
      And the error should contain ".yaks folder is not gitignored"

    # TODO: Add tests for partial acceptance/decline scenarios
    # - Accept gitignore, decline commit (should leave uncommitted changes)
    # - Decline gitignore (should fail immediately)

  Rule: yx auto-discovers the git repo root

    Example: Running yx from a subdirectory finds .yaks at repo root
      Given a git repository with .yaks gitignored and a yak called "shave-yak"
      When I list the yaks from a subdirectory of that repository
      Then the command should succeed
      And the output should include "shave-yak"

  Rule: YAK_PATH takes precedence over git repo root

    Example: Running yx from a subdirectory with YAK_PATH uses YAK_PATH
      Given a git repository with YAK_PATH set and a yak called "explicit-path-yak"
      When I list the yaks from a subdirectory using YAK_PATH
      Then the command should succeed
      And the output should include "explicit-path-yak"

  Rule: Git is required even when YAK_PATH is set

    Example: Running yx with YAK_PATH outside a git repo errors
      Given a directory that is not a git repository
      And YAK_PATH is set to a directory
      When I try to list the yaks from this directory
      Then the command should fail
      And the error should contain "not in a git repository"

  Rule: YX_SKIP_GIT_CHECKS bypasses all git requirements

    Example: YX_SKIP_GIT_CHECKS lets yx run outside a git repo
      Given a directory that is not a git repository
      And YAK_PATH is set to a directory
      When I list the yaks with YX_SKIP_GIT_CHECKS set
      Then the command should succeed
