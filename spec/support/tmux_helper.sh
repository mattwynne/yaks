#!/usr/bin/env bash
# tmux helper functions for completion smoke tests

TMUX_SOCKET="yx-test-$$"

start_completion_session() {
  # Start tmux with isolated socket, source completions, set PATH
  tmux -L "$TMUX_SOCKET" new-session -d -s test \
    -x 120 -y 30 \
    "bash --norc --noprofile"

  # Wait for shell to start
  sleep 0.5

  # Set up environment in the tmux session
  tmux -L "$TMUX_SOCKET" send-keys \
    "export PATH=\"$TEST_PROJECT_DIR/target/release:\$PATH\"" Enter
  tmux -L "$TMUX_SOCKET" send-keys \
    "export GIT_WORK_TREE=\"$TEST_REPO\"" Enter
  tmux -L "$TMUX_SOCKET" send-keys \
    "source \"$TEST_PROJECT_DIR/completions/yx.bash\"" Enter
  tmux -L "$TMUX_SOCKET" send-keys \
    "bind 'set show-all-if-ambiguous on'" Enter

  # Wait for setup to complete
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

# Poll until expected text appears or timeout
# Usage: poll_pane_content "expected text" [timeout_seconds]
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
