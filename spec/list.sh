Describe 'yx list'
  BeforeEach 'export YAK_PATH=$(mktemp -d)'
  AfterEach 'rm -rf "$YAK_PATH"'

  It 'shows message when no yaks exist'
    When run yx list
    The output should equal 'You have no yaks. Are you done?'
  End

  It 'lists added yaks'
    When run sh -c "
      yx add 'Fix the bug'
      yx list
    "
    The output should equal "- [ ] Fix the bug"
  End

  It 'supports ls as an alias for list'
    When run yx ls
    The output should equal 'You have no yaks. Are you done?'
  End

  It 'supports ls as an alias for list (with yaks)'
    When run sh -c "
      yx add 'Fix the bug'
      yx ls
    "
    The output should equal "- [ ] Fix the bug"
  End

  It 'sorts sibling yaks with done first, then by mtime'
    When run sh -c '
      yx add "first" && sleep 0.1 &&
      yx add "second" && sleep 0.1 &&
      yx add "third" &&
      yx done "second" &&
      yx list
    '
    The line 1 should equal $'\e[90m- [x] second\e[0m'
    The line 2 should equal "- [ ] first"
    The line 3 should equal "- [ ] third"
  End

  It 'shows done yaks in grey'
    When run sh -c '
      yx add "todo task"
      yx add "done task"
      yx done "done task"
      yx list
    '
    The line 1 should equal $'\e[90m- [x] done task\e[0m'
    The line 2 should equal "- [ ] todo task"
  End

  It 'displays nested yaks with indentation'
    When run sh -c "
      yx add 'first task'
      yx add 'first task/second task'
      yx list
    "
    The line 1 should equal "- [ ] first task"
    The line 2 should equal "  - [ ] second task"
  End

  It 'keeps hierarchy when child is done'
    When run sh -c "
      yx add 'parent a' && sleep 0.1 &&
      yx add 'parent a/child 1' && sleep 0.1 &&
      yx add 'parent a/child 2' &&
      yx done 'parent a/child 1' &&
      yx add 'parent b' &&
      yx list
    "
    The line 1 should equal "- [ ] parent a"
    The line 2 should equal $'\e[90m  - [x] child 1\e[0m'
    The line 3 should equal "  - [ ] child 2"
    The line 4 should equal "- [ ] parent b"
  End
End
