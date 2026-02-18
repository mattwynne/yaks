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

    Example: Running yx in a git repo without .yaks gitignored
      Given a git repository without .yaks in .gitignore
      When I try to list the yaks from this directory
      Then the command should fail
      And the error should contain ".yaks folder is not gitignored"

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
