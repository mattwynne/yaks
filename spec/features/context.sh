# shellcheck shell=bash
Describe 'yx context'
  BeforeEach 'setup_isolated_repo'
  AfterEach 'teardown_isolated_repo'

  It 'sets context from stdin and shows it'
    When run sh -c "
      yx add 'my yak'
      echo '# Some context' | yx context 'my yak'
      yx context --show 'my yak'
    "
    The output should equal "my yak

# Some context"
  End

  It 'replaces existing context from stdin'
    When run sh -c "
      yx add 'my yak'
      echo 'old' | yx context 'my yak'
      echo 'new' | yx context 'my yak'
      yx context --show 'my yak'
    "
    The output should equal "my yak

new"
  End
End
