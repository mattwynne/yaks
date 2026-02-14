# shellcheck shell=bash
Describe 'yx field'
  BeforeEach 'setup_isolated_repo'
  AfterEach 'teardown_isolated_repo'

  It 'writes a field from stdin and shows it'
    When run sh -c "
      yx add 'my yak'
      echo 'field content' | yx field 'my yak' notes
      yx field 'my yak' notes --show
    "
    The output should equal "my yak

field content"
  End

  It 'shows error for reserved field name'
    When run sh -c "
      yx add 'my yak'
      echo 'content' | yx field 'my yak' context.md
    "
    The status should be failure
    The error should include "Error: Field name 'context.md' is reserved"
  End
End
