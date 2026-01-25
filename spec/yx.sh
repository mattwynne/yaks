Describe 'yx'
  It 'shows help when run with --help'
    When run yx --help
    The output should include "Usage:"
    The status should be success
  End

  It 'shows help when run with no arguments'
    When run yx
    The output should include "Usage:"
    The status should be success
  End
End
