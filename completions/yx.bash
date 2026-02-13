#!/usr/bin/env bash
_yx_completions() {
    local cur="${COMP_WORDS[COMP_CWORD]}"
    local candidates
    candidates=$(yx completions -- "${COMP_WORDS[@]}" 2>/dev/null)
    COMPREPLY=($(compgen -W "$candidates" -- "$cur"))
}
complete -F _yx_completions yx
