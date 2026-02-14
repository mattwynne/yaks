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
