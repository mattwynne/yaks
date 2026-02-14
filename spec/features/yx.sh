# shellcheck shell=bash
Describe 'yx'
  It 'shows help when run with --help'
    When run yx --help
    The output should include "Usage:"
    The status should be success
  End

  It 'shows error for invalid subcommands'
    When run yx woop
    The error should include "error:"
    The status should be failure
  End
End
