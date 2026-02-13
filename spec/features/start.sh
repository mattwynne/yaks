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

  It 'shows error when starting non-existent yak'
    When run yx start "Nonexistent yak"
    The error should include "Error: yak 'Nonexistent yak' not found"
    The status should be failure
  End

  It 'resolves yak name with fuzzy matching'
    When run sh -c "
      yx add 'Fix the bug'
      yx start bug
      yx list --format markdown
    "
    The output should include "- [wip] Fix the bug"
  End

  It 'propagates wip to parent'
    When run sh -c "
      yx add 'parent'
      yx add 'parent/child'
      yx start 'parent/child'
      yx list --format markdown
    "
    The line 1 should equal "- [wip] parent"
    The line 2 should equal "  - [wip] child"
  End

  It 'sets state recursively on parent and all descendants'
    When run sh -c "
      yx add 'parent'
      yx add 'parent/child1'
      yx add 'parent/child2'
      yx start --recursive 'parent'
      yx list --format markdown
    "
    The output should include "- [wip] parent"
    The output should include "  - [wip] child1"
    The output should include "  - [wip] child2"
  End

  It 'works with wip alias'
    When run sh -c "
      yx add 'Fix the bug'
      yx wip 'Fix the bug'
      yx list --format markdown
    "
    The output should include "- [wip] Fix the bug"
  End
End
