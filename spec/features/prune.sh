# shellcheck shell=bash
Describe 'yx prune'
  BeforeEach 'setup_isolated_repo'
  AfterEach 'teardown_isolated_repo'

  It 'removes all done yaks'
    When run sh -c "
      yx add 'Fix the bug'
      yx add 'Write docs'
      yx done 'Fix the bug'
      yx prune
      yx list --format markdown
    "
    The output should include "- [todo] Write docs"
    The output should not include "Fix the bug"
  End

  It 'removes done child yaks'
    When run sh -c "
      yx add 'parent'
      yx add 'parent/child1'
      yx add 'parent/child2'
      yx done 'parent/child1'
      yx prune
      yx list --format markdown
    "
    The output should include "- [wip] parent"
    The output should not include "child1"
    The output should include "- [todo] child2"
  End
End
