Describe 'yx move'
  BeforeEach 'setup_isolated_repo'
  AfterEach 'teardown_isolated_repo'

  It 'renames a yak'
    When run sh -c "
      yx add 'old name'
      yx move 'old name' 'new name'
      yx list
    "
    The output should include "- [ ] new name"
    The output should not include "- [ ] old name"
  End

  It 'shows error when source yak not found'
    When run yx move "nonexistent" "new name"
    The status should be failure
    The error should include "not found"
  End

  It 'preserves done state when renaming'
    When run sh -c "
      yx add 'old task'
      yx done 'old task'
      yx move 'old task' 'new task'
      yx list
    "
    The output should include $'\e[90m- [x] new task\e[0m'
    The output should not include "old task"
  End

  It 'preserves context when renaming'
    When run sh -c "
      yx add 'old name'
      echo 'some context' | yx context 'old name'
      yx move 'old name' 'new name'
      yx context --show 'new name'
    "
    The output should include "some context"
  End

  It 'supports mv as an alias for move'
    When run sh -c "
      yx add 'original'
      yx mv 'original' 'renamed'
      yx list
    "
    The output should include "- [ ] renamed"
    The output should not include "- [ ] original"
  End

  It 'rejects new name with forbidden characters'
    When run sh -c "
      yx add 'valid'
      yx move 'valid' 'invalid:name'
    "
    The status should be failure
    The error should include "forbidden characters"
  End

  It 'moves a flat yak into a nested position'
    When run sh -c "
      yx add 'parent'
      yx add 'standalone'
      yx move 'standalone' 'parent/child'
      yx list
    "
    The line 1 should equal "- [ ] parent"
    The line 2 should equal "  - [ ] child"
  End

  It 'implicitly creates parent yaks when moving'
    When run sh -c "
      yx add 'standalone'
      yx move 'standalone' 'parent/child'
      yx list
    "
    The line 1 should equal "- [ ] parent"
    The line 2 should equal "  - [ ] child"
  End
End
