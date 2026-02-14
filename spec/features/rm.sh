# shellcheck shell=bash
Describe 'yx rm'
  BeforeEach 'setup_isolated_repo'
  AfterEach 'teardown_isolated_repo'

  It 'removes a yak by name'
    When run sh -c "
      yx add 'Fix the bug'
      yx add 'Write docs'
      yx rm 'Fix the bug'
      yx list --format markdown
    "
    The output should include "- [todo] Write docs"
    The output should not include "- [todo] Fix the bug"
  End
End
