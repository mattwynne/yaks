# shellcheck shell=bash
# shellcheck disable=SC1010
Describe 'Bash completion wiring'
  Skip if "bash lacks programmable completion builtins" \
    [ -z "$(bash -c 'type compgen 2>/dev/null')" ]

  BeforeEach 'setup_isolated_repo'
  AfterEach 'teardown_isolated_repo'

  setup_bash_completions() {
    source "$TEST_PROJECT_DIR/completions/yx.bash"
  }
  BeforeEach 'setup_bash_completions'

  get_completions() {
    COMP_WORDS=("$@" "")
    COMP_CWORD=$(( ${#COMP_WORDS[@]} - 1 ))
    _yx_completions
    printf '%s\n' "${COMPREPLY[@]}"
  }

  It 'offers subcommands'
    When call get_completions yx
    The output should include "add"
    The output should include "done"
  End

  It 'offers yak names for rm'
    yx add "test-yak"
    When call get_completions yx rm
    The output should include "test-yak"
  End

  It 'offers flags for done --'
    call_with_partial_flag() {
      COMP_WORDS=(yx done "--")
      COMP_CWORD=2
      _yx_completions
      printf '%s\n' "${COMPREPLY[@]}"
    }
    When call call_with_partial_flag
    The output should include "--undo"
  End
End
