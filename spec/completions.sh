Describe 'yx completions'
  BeforeEach 'export YAK_PATH=$(mktemp -d)'
  AfterEach 'rm -rf "$YAK_PATH"'

  It 'outputs nothing when no yaks exist'
    When run yx completions
    The output should equal ''
  End

  It 'lists all yak names (one per line)'
    When run sh -c 'yx add "Fix bug" && yx add "Add feature" && yx completions'
    The output should equal "Add feature
Fix bug"
  End

  It 'filters to incomplete yaks for "done" command'
    When run sh -c 'yx add "Fix bug" && yx add "Add feature" && yx done "Fix bug" && yx completions done'
    The output should equal "Add feature"
  End

  It 'filters to done yaks for "done --undo" command'
    When run sh -c 'yx add "Fix bug" && yx add "Add feature" && yx done "Fix bug" && yx completions done --undo'
    The output should equal "Fix bug"
  End
End
