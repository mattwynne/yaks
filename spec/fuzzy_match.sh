Describe 'fuzzy match on yak names'
  BeforeEach 'setup_isolated_repo'
  AfterEach 'teardown_isolated_repo'

  It 'matches a yak by unique substring'
    When run sh -c "
      yx add 'ideas/buy a pony'
      yx add 'ideas/fix the build'
      yx add 'ideas/fix the fridge'
      yx done build
      yx list
    "
    The output should include $'\e[90m  - [x] fix the build\e[0m'
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
End
