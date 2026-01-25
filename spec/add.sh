Describe 'yx add'
  BeforeEach 'export YAK_PATH=$(mktemp -d)'
  AfterEach 'rm -rf "$YAK_PATH"'

  It 'adds a yak'
    When run yx add "Fix the bug"
    The status should be success
  End

  It 'rejects empty yak names'
    When run yx add ""
    The status should be failure
    The error should include "name cannot be empty"
  End
End
