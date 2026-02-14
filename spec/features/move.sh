# shellcheck shell=bash
Describe 'yx move'
  BeforeEach 'setup_isolated_repo'
  AfterEach 'teardown_isolated_repo'

  It 'renames a yak'
    When run sh -c "
      yx add 'old name'
      yx move 'old name' 'new name'
      yx list --format markdown
    "
    The output should include "- [todo] new name"
    The output should not include "- [todo] old name"
  End

  It 'moves a flat yak into a nested position'
    When run sh -c "
      yx add 'parent'
      yx add 'standalone'
      yx move 'standalone' 'parent/child'
      yx list --format markdown
    "
    The line 1 should equal "- [todo] parent"
    The line 2 should equal "  - [todo] child"
  End

  It 'accepts parent-only destination, preserving source name'
    When run sh -c "
      yx add 'child-yak'
      yx add 'parent'
      yx move 'child-yak' 'parent'
      yx list --format markdown
    "
    The line 1 should equal "- [todo] parent"
    The line 2 should equal "  - [todo] child-yak"
  End
End
