Describe 'yx prune'
  BeforeEach 'export YAK_PATH=$(mktemp -d)'
  AfterEach 'rm -rf "$YAK_PATH"'

  It 'removes all done yaks'
    When run sh -c 'yx add "Fix the bug" && yx add "Write docs" && yx done "Fix the bug" && yx prune && yx list'
    The output should include "- [ ] Write docs"
    The output should not include "Fix the bug"
  End

  It 'handles prune when no yaks exist'
    When run sh -c 'yx prune && yx list'
    The output should equal "You have no yaks. Are you done?"
  End

  It 'keeps all yaks when none are done'
    When run sh -c 'yx add "Fix the bug" && yx add "Write docs" && yx prune && yx list'
    The output should include "- [ ] Fix the bug"
    The output should include "- [ ] Write docs"
  End

  It 'removes all yaks when all are done'
    When run sh -c 'yx add "Fix the bug" && yx add "Write docs" && yx done "Fix the bug" && yx done "Write docs" && yx prune && yx list'
    The output should equal "You have no yaks. Are you done?"
  End
End
