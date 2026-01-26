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
    When run sh -c "
      yx add
      yx list
    "
    The output should include "- [ ] Fix the bug"
    The output should include "- [ ] Write the docs"
  End

  It 'captures multi-word yak names without quotes'
    When run sh -c "
      yx add this is a test
      yx list
    "
    The output should include "- [ ] this is a test"
  End

  It 'allows nested yak names with forward slash'
    When run yx add "foo/bar"
    The status should be success
  End

  It 'rejects yak names with backslash'
    When run yx add "foo\\bar"
    The error should include "Invalid yak name"
    The status should be failure
  End

  It 'rejects yak names with colon'
    When run yx add "foo:bar"
    The error should include "Invalid yak name"
    The status should be failure
  End

  It 'rejects yak names with asterisk'
    When run yx add "foo*bar"
    The error should include "Invalid yak name"
    The status should be failure
  End

  It 'rejects yak names with question mark'
    When run yx add "foo?bar"
    The error should include "Invalid yak name"
    The status should be failure
  End

  It 'rejects yak names with pipe'
    When run yx add "foo|bar"
    The error should include "Invalid yak name"
    The status should be failure
  End

  It 'rejects yak names with less than'
    When run yx add "foo<bar"
    The error should include "Invalid yak name"
    The status should be failure
  End

  It 'rejects yak names with greater than'
    When run yx add "foo>bar"
    The error should include "Invalid yak name"
    The status should be failure
  End

  It 'rejects yak names with quotes'
    When run yx add 'foo"bar'
    The error should include "Invalid yak name"
    The status should be failure
  End
End
