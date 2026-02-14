# shellcheck shell=bash
# shellcheck disable=SC1010
Describe 'yx done'
  BeforeEach 'setup_isolated_repo'
  AfterEach 'teardown_isolated_repo'

  It 'marks a yak as done'
    When run sh -c "
      yx add 'Fix the bug'
      yx done 'Fix the bug'
      yx list --format markdown
    "
    The output should include $'\e[90m- [done] Fix the bug\e[0m'
  End

  It 'marks a nested yak as done'
    When run sh -c "
      yx add 'parent'
      yx add 'parent/child'
      yx done 'parent/child'
      yx list --format markdown
    "
    The line 1 should equal "- [wip] parent"
    The line 2 should equal $'\e[90m  - [done] child\e[0m'
  End

  It 'errors when marking a parent yak as done with incomplete children'
    When run sh -c "
      yx add 'parent'
      yx add 'parent/child'
      yx done 'parent'
    "
    The error should include "Error: cannot mark 'parent' as done - it has incomplete children"
    The status should be failure
  End

  It 'marks parent and all children as done with --recursive flag'
    When run sh -c "
      yx add 'parent'
      yx add 'parent/child1'
      yx add 'parent/child2'
      yx add 'parent/child1/grandchild'
      yx done --recursive 'parent'
      yx list --format markdown
    "
    The output should include $'\e[90m- [done] parent\e[0m'
    The output should include $'\e[90m  - [done] child1\e[0m'
    The output should include $'\e[90m  - [done] child2\e[0m'
    The output should include $'\e[90m    - [done] grandchild\e[0m'
  End
End
