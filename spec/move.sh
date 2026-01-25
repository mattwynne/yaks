Describe 'yx move'
  BeforeEach 'export YAK_PATH=$(mktemp -d)'
  AfterEach 'rm -rf "$YAK_PATH"'

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
    The output should include "- [x] new task"
    The output should not include "- [x] old task"
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
      yx move 'valid' 'invalid/name'
    "
    The status should be failure
    The error should include "forbidden characters"
  End
End
