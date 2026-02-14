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

  It 'offers nested yak names for add after slash'
    yx add "grandma/mummy"
    get_completions_after_slash() {
      COMP_WORDS=(yx add "grandma" "/" "")
      COMP_CWORD=4
      _yx_completions
      printf '%s\n' "${COMPREPLY[@]}"
    }
    When call get_completions_after_slash
    The output should include "grandma/mummy/"
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
