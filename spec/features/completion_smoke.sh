# shellcheck shell=bash
# shellcheck disable=SC1010
Describe 'Tab completion (tmux smoke test)'
  Skip if "tmux not installed" \
    [ -z "$(command -v tmux)" ]
  Include spec/support/tmux_helper.sh

  BeforeEach 'setup_isolated_repo'
  AfterEach 'destroy_completion_session'
  AfterEach 'teardown_isolated_repo'

  It 'yx <TAB> shows commands'
    start_completion_session
    tmux_send "yx "
    tmux_send_tab
    poll_pane_content "add" 5
    When call tmux_capture
    The output should include "add"
    The output should include "done"
  End

  It 'yx rm <TAB> shows yak names'
    yx add "smoke-test-yak"
    start_completion_session
    tmux_send "yx rm "
    tmux_send_tab
    poll_pane_content "smoke-test-yak" 5
    When call tmux_capture
    The output should include "smoke-test-yak"
  End
End
