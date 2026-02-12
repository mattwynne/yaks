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

  It 'sets a yak to done state'
    When run sh -c "
      yx add 'get milk'
      yx state 'get milk' done
      yx list --format markdown
    "
    The output should include $'\e[90m- [done] get milk\e[0m'
  End

  It 'sets a yak back to todo state'
    When run sh -c "
      yx add 'get milk'
      yx state 'get milk' wip
      yx state 'get milk' todo
      yx list --format markdown
    "
    The output should include "- [todo] get milk"
  End

  It 'shows error when setting state of non-existent yak'
    When run yx state "Nonexistent yak" wip
    The error should include "Error: yak 'Nonexistent yak' not found"
    The status should be failure
  End

  It 'shows error when setting invalid state'
    When run sh -c "
      yx add 'get milk'
      yx state 'get milk' invalid-state
    "
    The error should include "Error: Invalid state 'invalid-state'. Valid states are: todo, wip, done"
    The status should be failure
  End

  It 'sets parent to wip when child state changes from todo'
    When run sh -c "
      yx add 'make tea'
      yx add 'make tea/get milk'
      yx state 'make tea/get milk' wip
      yx list --format markdown
    "
    The line 1 should equal "- [wip] make tea"
    The line 2 should equal "  - [wip] get milk"
  End

  It 'keeps parent as wip when child is done if other children remain in todo'
    When run sh -c "
      yx add 'make tea'
      yx add 'make tea/get milk'
      yx add 'make tea/boil water'
      yx state 'make tea/get milk' done
      yx list --format markdown
    "
    The line 1 should equal "- [wip] make tea"
    The line 2 should equal $'\e[90m  - [done] get milk\e[0m'
    The line 3 should equal "  - [todo] boil water"
  End
End
