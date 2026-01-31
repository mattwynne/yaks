# shellcheck shell=bash
Describe 'yx'
  It 'shows help when run with --help'
    When run yx --help
    The error should include "USAGE:"
    The status should be success
  End

  It 'shows help when run with no arguments'
    When run yx
    The error should include "USAGE:"
    The status should be success
  End

  It 'shows help for invalid subcommands'
    When run yx woop
    The error should include "error:"
    The status should be failure
  End
End
