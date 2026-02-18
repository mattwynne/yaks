# shellcheck shell=bash
# shellcheck disable=SC1010

TMUX_SOCKET="yx-test-$$"

start_completion_session() {
  tmux -L "$TMUX_SOCKET" new-session -d -s test \
    -x 120 -y 30 \
    "bash --norc --noprofile"
  sleep 0.5
  tmux -L "$TMUX_SOCKET" send-keys \
    "export PATH=\"$TEST_PROJECT_DIR/target/release:\$PATH\"" Enter
  tmux -L "$TMUX_SOCKET" send-keys \
    "export GIT_CONFIG_PARAMETERS=\"'core.hooksPath=/dev/null'\"" Enter
  tmux -L "$TMUX_SOCKET" send-keys \
    "source \"$TEST_PROJECT_DIR/completions/yx.bash\"" Enter
  tmux -L "$TMUX_SOCKET" send-keys \
    "bind 'set show-all-if-ambiguous on'" Enter
  sleep 0.3
}

tmux_send() {
  tmux -L "$TMUX_SOCKET" send-keys "$1"
}

tmux_send_enter() {
  tmux -L "$TMUX_SOCKET" send-keys Enter
}

tmux_send_tab() {
  tmux -L "$TMUX_SOCKET" send-keys Tab
}

tmux_capture() {
  tmux -L "$TMUX_SOCKET" capture-pane -p -t test
}

poll_pane_content() {
  local expected="$1"
  local timeout="${2:-5}"
  local interval=0.2
  local elapsed=0
  while [ "$(echo "$elapsed < $timeout" | bc)" -eq 1 ]; do
    local content
    content=$(tmux_capture)
    if echo "$content" | grep -q "$expected"; then
      return 0
    fi
    sleep "$interval"
    elapsed=$(echo "$elapsed + $interval" | bc)
  done
  return 1
}

destroy_completion_session() {
  tmux -L "$TMUX_SOCKET" kill-server 2>/dev/null || true
}

Describe 'Tab completion (tmux smoke test)'
  Skip if "tmux not installed" \
    [ -z "$(command -v tmux)" ]

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
End
