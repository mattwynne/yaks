# shellcheck shell=bash
# shellcheck disable=SC1010
Describe 'yx state'
  BeforeEach 'setup_isolated_repo'
  AfterEach 'teardown_isolated_repo'

  It 'sets a yak to wip state'
    When run sh -c "
      yx add 'get milk'
      yx state 'get milk' wip
      yx list --format markdown
    "
    The output should include "- [wip] get milk"
  End

  It 'shows error when setting invalid state'
    When run sh -c "
      yx add 'get milk'
      yx state 'get milk' invalid-state
    "
    The error should include "Error: Invalid state 'invalid-state'. Valid states are: todo, wip, done"
    The status should be failure
  End
End
