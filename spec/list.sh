Describe 'yx list'
  BeforeEach 'export YAK_PATH=$(mktemp -d)'
  AfterEach 'rm -rf "$YAK_PATH"'

  It 'shows message when no yaks exist'
    When run yx list
    The output should equal 'You have no yaks. Are you done?'
  End

  It 'lists added yaks'
    When run sh -c 'yx add "Fix the bug" && yx list'
    The output should equal "- [ ] Fix the bug"
  End

  It 'supports ls as an alias for list'
    When run yx ls
    The output should equal 'You have no yaks. Are you done?'
  End

  It 'supports ls as an alias for list (with yaks)'
    When run sh -c 'yx add "Fix the bug" && yx ls'
    The output should equal "- [ ] Fix the bug"
  End

  It 'sorts yaks with done first, then by creation time (oldest first)'
    When run sh -c '
      yx add "oldest" && sleep 0.1 &&
      yx add "middle" && sleep 0.1 &&
      yx add "newest" &&
      yx done "middle" &&
      yx list
    '
    The line 1 should equal "- [x] middle"
    The line 2 should equal "- [ ] oldest"
    The line 3 should equal "- [ ] newest"
  End
End
