Feature: yx CLI basics

  The yx command-line interface provides helpful feedback
  when users ask for help or make mistakes.

  Rule: Help output

    Example: Running yx with --help shows usage information
      When I run yx --help
      Then it should succeed
      And the output should include "Usage:"

  Rule: Invalid subcommand feedback

    Example: Running yx with an unknown subcommand shows an error
      When I run yx woop
      Then the command should fail
      And the error should contain "error:"

  Rule: Running yx without a subcommand shows usage and exits non-zero

    @fullstack
    Example: No subcommand exits with code 2
      When I invoke yx with no subcommand
      Then the command should fail
      And the error should contain "Usage"
