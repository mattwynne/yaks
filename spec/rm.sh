Describe 'yx rm'
  BeforeEach 'setup_isolated_repo'
  AfterEach 'teardown_isolated_repo'

  It 'removes a yak by name'
    When run sh -c "
      yx add 'Fix the bug'
      yx add 'Write docs'
      yx rm 'Fix the bug'
      yx list
    "
    The output should include "- [ ] Write docs"
    The output should not include "- [ ] Fix the bug"
  End

  It 'shows error when yak not found'
    When run yx rm "Nonexistent yak"
    The status should be failure
    The error should include "not found"
  End

  It 'handles removing the only yak'
    When run sh -c "
      yx add 'Only yak'
      yx rm 'Only yak'
      yx list
    "
    The output should equal "You have no yaks. Are you done?"
  End

  It 'removes multi-word yak names without quotes'
    When run sh -c "
      yx add this is a test
      yx add another yak
      yx rm this is a test
      yx list
    "
    The output should include "- [ ] another yak"
    The output should not include "- [ ] this is a test"
  End

  It 'removes a nested yak'
    When run sh -c "
      yx add 'parent'
      yx add 'parent/child'
      yx rm 'parent/child'
      yx list
    "
    The output should equal "- [ ] parent"
  End
End
