Describe 'yx add'
  BeforeEach 'export YAK_PATH=$(mktemp -d)'
  AfterEach 'rm -rf "$YAK_PATH"'

  It 'adds a yak'
    When run yx add "Fix the bug"
    The status should be success
  End

  It 'enters interactive mode when run without arguments'
    Data
      #|Fix the bug
      #|Write the docs
      #|
    End
    When run yx add
    The status should be success
    The output should include "Enter yaks (empty line to finish)"
  End

  It 'adds multiple yaks in interactive mode'
    Data
      #|Fix the bug
      #|Write the docs
      #|
    End
    When run sh -c 'yx add && yx list'
    The output should include "- [ ] Fix the bug"
    The output should include "- [ ] Write the docs"
  End
End
