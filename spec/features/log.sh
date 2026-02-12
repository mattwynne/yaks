# shellcheck shell=bash
Describe 'yx log'
  BeforeEach 'setup_isolated_repo'
  AfterEach 'teardown_isolated_repo'

  It 'shows empty log when no events exist'
    When run yx log
    The output should equal ""
    The status should be success
  End

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

  It 'displays done events'
    When run sh -c "
      yx add 'test yak'
      yx done 'test yak'
      yx log
    "
    The line 1 of output should include 'Added: "test yak"'
    The line 2 of output should include 'StateUpdated: "test yak" "done"'
    The status should be success
  End

  It 'displays done --undo events'
    When run sh -c "
      yx add 'test yak'
      yx done 'test yak'
      yx done --undo 'test yak'
      yx log
    "
    The line 1 of output should include 'Added: "test yak"'
    The line 2 of output should include 'StateUpdated: "test yak" "done"'
    The line 3 of output should include 'StateUpdated: "test yak" "todo"'
    The status should be success
  End

  It 'displays remove events'
    When run sh -c "
      yx add 'test yak'
      yx rm 'test yak'
      yx log
    "
    The line 1 of output should include 'Added: "test yak"'
    The line 2 of output should include 'Removed: "test yak"'
    The status should be success
  End

  It 'displays context events'
    When run sh -c "
      yx add 'test yak'
      echo 'Some context' | yx context 'test yak'
      yx log
    "
    The line 1 of output should include 'Added: "test yak"'
    The line 2 of output should include 'ContextUpdated: "test yak"'
    The status should be success
  End
End
