# shellcheck shell=bash
Describe 'yx list'
  BeforeEach 'setup_isolated_repo'
  AfterEach 'teardown_isolated_repo'

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

  It 'supports --format plain for simple yak names'
    When run sh -c "
      yx add 'Fix the bug'
      yx ls --format plain
    "
    The output should equal "Fix the bug"
  End

  It 'supports --format plain with nested yaks showing full paths'
    When run sh -c "
      yx add 'parent task' && sleep 0.1 &&
      yx add 'parent task/child task' &&
      yx ls --format plain
    "
    The line 1 should equal "parent task"
    The line 2 should equal "parent task/child task"
  End

  It 'supports --format raw as an alias for plain'
    When run sh -c "
      yx add 'Fix the bug'
      yx ls --format raw
    "
    The output should equal "Fix the bug"
  End

  It 'supports --format markdown explicitly'
    When run sh -c "
      yx add 'Fix the bug'
      yx ls --format markdown
    "
    The output should equal "- [ ] Fix the bug"
  End

  It 'supports --format md as an alias for markdown'
    When run sh -c "
      yx add 'Fix the bug'
      yx ls --format md
    "
    The output should equal "- [ ] Fix the bug"
  End

  It 'outputs nothing in plain format when no yaks exist'
    When run yx ls --format plain
    The output should equal ''
  End

  It 'supports --only not-done to show only incomplete yaks'
    When run sh -c "
      yx add 'incomplete task' && sleep 0.1 &&
      yx add 'done task' &&
      yx done 'done task' &&
      yx ls --format plain --only not-done
    "
    The output should equal "incomplete task"
  End

  It 'supports --only done to show only completed yaks'
    When run sh -c "
      yx add 'incomplete task' && sleep 0.1 &&
      yx add 'done task' &&
      yx done 'done task' &&
      yx ls --format plain --only done
    "
    The output should equal "done task"
  End

  It 'shows all yaks when no --only filter is specified'
    When run sh -c "
      yx add 'done task' && sleep 0.1 &&
      yx add 'incomplete task' &&
      yx done 'done task' &&
      yx ls --format plain
    "
    The line 1 should equal "done task"
    The line 2 should equal "incomplete task"
  End
End
