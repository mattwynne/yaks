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
      yx list
    "
    The output should include "- [ ] Write docs"
    The output should not include "Fix the bug"
  End

  It 'handles prune when no yaks exist'
    When run sh -c "
      yx prune
      yx list
    "
    The output should equal "You have no yaks. Are you done?"
  End

  It 'keeps all yaks when none are done'
    When run sh -c "
      yx add 'Fix the bug'
      yx add 'Write docs'
      yx prune
      yx list
    "
    The output should include "- [ ] Fix the bug"
    The output should include "- [ ] Write docs"
  End

  It 'removes all yaks when all are done'
    When run sh -c "
      yx add 'Fix the bug'
      yx add 'Write docs'
      yx done 'Fix the bug'
      yx done 'Write docs'
      yx prune
      yx list
    "
    The output should equal "You have no yaks. Are you done?"
  End

  It 'removes done child yaks'
    When run sh -c "
      yx add 'parent'
      yx add 'parent/child1'
      yx add 'parent/child2'
      yx done 'parent/child1'
      yx prune
      yx list
    "
    The output should include "- [ ] parent"
    The output should not include "child1"
    The output should include "- [ ] child2"
  End

  Describe 'logging'
    It 'logs each yak removal individually'
      When run in_test_repo "
        yx add 'Fix the bug'
        yx add 'Write docs'
        yx done 'Fix the bug'
        yx done 'Write docs'
        yx prune
        git log refs/notes/yaks --oneline
      "
      The output should include "rm Fix the bug"
      The output should include "rm Write docs"
    End
  End
End
