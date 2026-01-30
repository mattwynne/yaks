# shellcheck shell=bash
Describe 'yak directories'
  BeforeEach 'setup_isolated_repo'
  AfterEach 'teardown_isolated_repo'

  It 'have context.md file by default'
    yx add "test yak"

    When call sh -c "find '$GIT_WORK_TREE/.yaks/test yak' -type f -name 'context.md' | wc -l"
    The output should equal "1"
  End
End
