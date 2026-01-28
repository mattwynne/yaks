Describe 'yx completions'
  BeforeEach 'setup_isolated_repo'
  AfterEach 'teardown_isolated_repo'

  It 'outputs nothing when no yaks exist'
    When run yx completions
    The output should equal ''
  End

  It 'lists all yak names (one per line)'
    When run sh -c "
      yx add 'Fix bug'
      yx add 'Add feature'
      yx completions
    "
    The output should equal "Add feature
Fix bug"
  End

  It 'filters to incomplete yaks for "done" command'
    When run sh -c "
      yx add 'Fix bug'
      yx add 'Add feature'
      yx done 'Fix bug'
      yx completions done
    "
    The output should equal "Add feature"
  End

  It 'filters to done yaks for "done --undo" command'
    When run sh -c "
      yx add 'Fix bug'
      yx add 'Add feature'
      yx done 'Fix bug'
      yx completions done --undo
    "
    The output should equal "Fix bug"
  End

  It 'includes nested yak paths in completions'
    When run sh -c "
      yx add 'parent'
      yx add 'parent/child'
      yx completions
    "
    The output should include "parent"
    The output should include "parent/child"
  End
End
