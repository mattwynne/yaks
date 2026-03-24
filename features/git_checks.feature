@fullstack
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

  Rule: yx auto-discovers the git repo root

    Example: Running yx from a subdirectory finds .yaks at repo root
      Given a git repository with .yaks gitignored and a yak called "shave-yak"
      When I list the yaks from a subdirectory of that repository
      Then the command should succeed
      And the output should include "shave-yak"

  Rule: YX_ROOT overrides git root detection

    Example: Running yx with YX_ROOT from any directory
      Given a git repository with .yaks gitignored and a yak called "root-yak"
      When I list the yaks with YX_ROOT pointing to that repository
      Then the command should succeed
      And the output should include "root-yak"

  Rule: YX_ROOT must point to a git repo

    Example: Running yx with YX_ROOT pointing to a non-git directory
      Given a directory that is not a git repository
      When I try to list the yaks with YX_ROOT pointing to that directory
      Then the command should fail
      And the error should contain "YX_ROOT does not point to a git repository"

  Rule: YX_SKIP_GIT_CHECKS bypasses all git requirements

    Example: YX_SKIP_GIT_CHECKS lets yx run outside a git repo
      Given a directory that is not a git repository
      And a .yaks directory exists in that directory
      When I list the yaks with YX_SKIP_GIT_CHECKS set
      Then the command should succeed
