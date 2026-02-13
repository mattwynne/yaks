# shellcheck shell=bash
Describe 'fuzzy match on yak names'
  BeforeEach 'setup_isolated_repo'
  AfterEach 'teardown_isolated_repo'

  It 'matches a yak by unique substring'
    When run sh -c "
      yx add 'ideas/buy a pony'
      yx add 'ideas/fix the build'
      yx add 'ideas/fix the fridge'
      yx done build
      yx list --format markdown
    "
    The output should include $'\e[90m  - [done] fix the build\e[0m'
  End

  It 'fails with ambiguous match error'
    When run sh -c "
      yx add 'ideas/buy a pony'
      yx add 'ideas/fix the build'
      yx add 'ideas/fix the fridge'
      yx done fix
    "
    The error should include "Error: yak name 'fix' is ambiguous"
    The status should be failure
  End

  It 'matches parent yak without ambiguity from parent/child paths'
    When run sh -c "
      yx add parent
      yx add parent/child1
      echo 'test context' | yx context parent
      yx context --show parent
    "
    The status should be success
    The output should include "test context"
    The output should not include "ambiguous"
  End

  It 'fuzzy matches for rm'
    When run sh -c "
      yx add 'ideas/buy a pony'
      yx add 'ideas/fix the build'
      yx rm build
      yx list --format markdown
    "
    The output should not include "fix the build"
    The output should include "buy a pony"
  End

  It 'fuzzy matches for context edit'
    When run sh -c "
      yx add 'ideas/fix the build'
      unset YX_IGNORE_STDIN
      echo 'build notes' | yx context build
      yx context --show 'ideas/fix the build'
    "
    The output should include "build notes"
  End

  It 'fuzzy matches for context show'
    When run sh -c "
      yx add 'ideas/fix the build'
      unset YX_IGNORE_STDIN
      echo 'build notes' | yx context 'ideas/fix the build'
      yx context --show build
    "
    The output should include "build notes"
  End

  It 'fuzzy matches source for move'
    When run sh -c "
      yx add 'fix the build'
      yx mv build 'renamed build'
      yx list --format markdown
    "
    The output should include "renamed build"
    The output should not include "fix the build"
  End
End
