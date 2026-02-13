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

  It 'shows error when marking non-existent yak as done'
    When run yx done "Nonexistent yak"
    The error should include "Error: yak 'Nonexistent yak' not found"
    The status should be failure
  End

  It 'displays mix of done and not-done yaks'
    When run sh -c "
      yx add 'Fix the bug'
      yx add 'Write the docs'
      yx add 'Add tests'
      yx done 'Write the docs'
      yx list --format markdown
    "
    The output should include "- [todo] Fix the bug"
    The output should include $'\e[90m- [done] Write the docs\e[0m'
    The output should include "- [todo] Add tests"
  End

  It 'handles yak names starting with x'
    When run sh -c "
      yx add 'x marks the spot'
      yx list --format markdown
    "
    The output should include "- [todo] x marks the spot"
  End

  It 'marks yak starting with x as done correctly'
    When run sh -c "
      yx add 'x marks the spot'
      yx done 'x marks the spot'
      yx list --format markdown
    "
    The output should include $'\e[90m- [done] x marks the spot\e[0m'
  End

  It 'unmarks a done yak with --undo flag'
    When run sh -c "
      yx add 'Fix the bug'
      yx done 'Fix the bug'
      yx done --undo 'Fix the bug'
      yx list --format markdown
    "
    The output should include "- [todo] Fix the bug"
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
