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
End
