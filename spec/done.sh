Describe 'yx done'
  BeforeEach 'export YAK_PATH=$(mktemp -d)'
  AfterEach 'rm -rf "$YAK_PATH"'

  It 'marks a yak as done'
    When run sh -c "
      yx add 'Fix the bug'
      yx done 'Fix the bug'
      yx list
    "
    The output should include "- [x] Fix the bug"
  End

  It 'shows error when marking non-existent yak as done'
    When run yx done "Nonexistent yak"
    The error should include "Error: yak 'Nonexistent yak' not found"
    The status should be failure
  End

  It 'displays mix of done and not-done yaks'
    Data
      #|Fix the bug
      #|Write the docs
      #|Add tests
      #|
    End
    When run sh -c "
      yx add
      yx done 'Write the docs'
      yx list
    "
    The output should include "- [ ] Fix the bug"
    The output should include "- [x] Write the docs"
    The output should include "- [ ] Add tests"
  End

  It 'handles yak names starting with x'
    When run sh -c "
      yx add 'x marks the spot'
      yx list
    "
    The output should include "- [ ] x marks the spot"
  End

  It 'marks yak starting with x as done correctly'
    When run sh -c "
      yx add 'x marks the spot'
      yx done 'x marks the spot'
      yx list
    "
    The output should include "- [x] x marks the spot"
  End

  It 'unmarks a done yak with --undo flag'
    When run sh -c "
      yx add 'Fix the bug'
      yx done 'Fix the bug'
      yx done --undo 'Fix the bug'
      yx list
    "
    The output should include "- [ ] Fix the bug"
  End
End
