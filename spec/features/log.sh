# shellcheck shell=bash
Describe 'yx log'
  BeforeEach 'setup_isolated_repo'
  AfterEach 'teardown_isolated_repo'

  It 'displays add events'
    When run sh -c "
      yx add 'test yak'
      yx log
    "
    The output should include 'Added: "test yak"'
    The status should be success
  End

  It 'displays events in chronological order'
    When run sh -c "
      yx add 'first yak'
      yx add 'second yak'
      yx log
    "
    The line 1 of output should include 'Added: "first yak"'
    The line 2 of output should include 'Added: "second yak"'
    The status should be success
  End
End
