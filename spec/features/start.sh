# shellcheck shell=bash
# shellcheck disable=SC1010
Describe 'yx start'
  BeforeEach 'setup_isolated_repo'
  AfterEach 'teardown_isolated_repo'

  It 'sets a yak to wip state'
    When run sh -c "
      yx add 'Fix the bug'
      yx start 'Fix the bug'
      yx list --format markdown
    "
    The output should include "- [wip] Fix the bug"
  End
End
