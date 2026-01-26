Describe 'yx done'
  BeforeEach 'export YAK_PATH=$(mktemp -d)'
  AfterEach 'rm -rf "$YAK_PATH"'

  It 'marks a yak as done'
    When run sh -c "
      yx add 'Fix the bug'
      yx done 'Fix the bug'
      yx list
    "
    The output should include $'\e[90m- [x] Fix the bug\e[0m'
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
    The output should include $'\e[90m- [x] Write the docs\e[0m'
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
    The output should include $'\e[90m- [x] x marks the spot\e[0m'
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

  It 'marks a nested yak as done'
    When run sh -c "
      yx add 'parent'
      yx add 'parent/child'
      yx done 'parent/child'
      yx list
    "
    The line 1 should equal "- [ ] parent"
    The line 2 should equal $'\e[90m  - [x] child\e[0m'
  End

  It 'migrates old done files to state files'
    When run sh -c "
      yx add 'old yak'
      # Simulate old format by creating done file directly
      touch \"\$YAK_PATH/old yak/done\"
      rm -f \"\$YAK_PATH/old yak/state\"
      # Any yx command should trigger migration
      yx list
      # Check that state file now exists
      [ -f \"\$YAK_PATH/old yak/state\" ] && cat \"\$YAK_PATH/old yak/state\"
    "
    The output should include "done"
  End
End
