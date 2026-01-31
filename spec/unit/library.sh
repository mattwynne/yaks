# shellcheck shell=bash
Describe 'Library loading'
  It 'can source lib/yaks.sh without errors'
    When run bash -c "source $TEST_PROJECT_DIR/lib/yaks.sh && echo 'loaded'"
    The status should be success
    The output should include "loaded"
  End

  It 'exports validate_yak_name function'
    When run bash -c "source $TEST_PROJECT_DIR/lib/yaks.sh && type validate_yak_name"
    The status should be success
    The output should include "validate_yak_name is a function"
  End
End
