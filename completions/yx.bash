#!/usr/bin/env bash
_yx_completions() {
    # Remove / from COMP_WORDBREAKS so hierarchical yak names
    # (e.g. grandma/mummy) are treated as single tokens
    COMP_WORDBREAKS="${COMP_WORDBREAKS//\//}"

    local cur="${COMP_WORDS[COMP_CWORD]}"
    local candidates
    candidates=$(yx completions -- "${COMP_WORDS[@]}" 2>/dev/null)
    COMPREPLY=($(compgen -W "$candidates" -- "$cur"))
}
complete -F _yx_completions yx
