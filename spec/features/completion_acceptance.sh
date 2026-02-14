# shellcheck shell=bash
# shellcheck disable=SC1010
Describe 'yx completions (acceptance)'
  BeforeEach 'setup_isolated_repo'
  AfterEach 'teardown_isolated_repo'

  It 'lists commands'
    When run yx completions -- yx ""
    The status should be success
    The output should include "add"
    The output should include "done"
    The output should include "remove"
  End

  It 'filters done yaks for done command'
    yx add "todo-yak"
    yx add "done-yak"
    yx done "done-yak"
    When run yx completions -- yx done ""
    The output should include "todo-yak"
    The output should not include "done-yak"
  End
End
