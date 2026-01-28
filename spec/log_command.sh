Describe 'log_command'
  setup_test() {
    export TEST_REPO=$(mktemp -d)
    git -C "$TEST_REPO" init --quiet
    git -C "$TEST_REPO" config user.email "test@example.com"
    git -C "$TEST_REPO" config user.name "Test User"
    setup_gitignore_for_yaks "$TEST_REPO"
    export YAKS_PATH="$TEST_REPO/.yaks"
  }

  cleanup_test() {
    rm -rf "$TEST_REPO"
  }

  BeforeEach 'setup_test'
  AfterEach 'cleanup_test'

  It 'commits yak changes to refs/notes/yaks'
    When run sh -c "
      cd \"\$TEST_REPO\"
      yx add 'test yak'
      # Check that refs/notes/yaks exists and has a commit
      git rev-parse refs/notes/yaks >/dev/null 2>&1
    "
    The status should be success
  End

  It 'uses structured commit message for add command'
    When run sh -c "
      cd \"\$TEST_REPO\"
      yx add 'test yak'
      git log refs/notes/yaks -1 --format=%s
    "
    The output should equal "add test yak"
  End

  It 'includes git author in commits'
    When run sh -c "
      cd \"\$TEST_REPO\"
      yx add 'test yak'
      git log refs/notes/yaks -1 --format='%an <%ae>'
    "
    The output should equal "Test User <test@example.com>"
  End

  It 'creates sequential commits on multiple operations'
    When run sh -c "
      cd \"\$TEST_REPO\"
      yx add 'yak one'
      yx add 'yak two'
      git log refs/notes/yaks --oneline | wc -l
    "
    The output should equal "2"
  End

  It 'done command creates commit with structured message'
    When run sh -c "
      cd \"\$TEST_REPO\"
      yx add 'test yak'
      yx done 'test yak'
      git log refs/notes/yaks -1 --format=%s
    "
    The output should equal "done test yak"
  End

  It 'done --undo command creates commit with structured message'
    When run sh -c "
      cd \"\$TEST_REPO\"
      yx add 'test yak'
      yx done 'test yak'
      yx done --undo 'test yak'
      git log refs/notes/yaks -1 --format=%s
    "
    The output should equal "done --undo test yak"
  End

  It 'logs removal even when YAKS_PATH becomes empty'
    When run sh -c "
      cd \"\$TEST_REPO\"
      yx add 'only yak'
      yx rm 'only yak'
      git log refs/notes/yaks -1 --format=%s
    "
    The output should equal "rm only yak"
  End
End
