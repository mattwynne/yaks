#!/usr/bin/env bash
# Bash completion for yx (yak CLI)
# To enable: source this file or add to your .bashrc:
#   source /path/to/yx/completions/yx.bash

_yx_completions() {
    local cur="${COMP_WORDS[COMP_CWORD]}"
    local prev="${COMP_WORDS[COMP_CWORD-1]}"
    local cmd="${COMP_WORDS[1]}"

    # Complete commands
    if [ "$COMP_CWORD" -eq 1 ]; then
        COMPREPLY=($(compgen -W "add list ls done rm move mv prune context completions --help" -- "$cur"))
        return 0
    fi

    # Complete yak names based on command
    case "$cmd" in
        done)
            # Check if --undo flag is present
            if [[ " ${COMP_WORDS[@]} " =~ " --undo " ]]; then
                # After --undo, complete with done yaks
                COMPREPLY=($(compgen -W "$(yx completions done --undo 2>/dev/null)" -- "$cur"))
            elif [ "$prev" = "done" ] && [ "$cur" = "--" ]; then
                # Offer --undo flag
                COMPREPLY=($(compgen -W "--undo" -- "$cur"))
            else
                # Complete with incomplete yaks
                COMPREPLY=($(compgen -W "$(yx completions done 2>/dev/null)" -- "$cur"))
            fi
            ;;
        rm|context|move|mv)
            # Complete with all yaks
            COMPREPLY=($(compgen -W "$(yx completions 2>/dev/null)" -- "$cur"))
            ;;
        context)
            # Offer --show and --edit flags
            if [ "$prev" = "context" ]; then
                COMPREPLY=($(compgen -W "--show --edit $(yx completions 2>/dev/null)" -- "$cur"))
            else
                COMPREPLY=($(compgen -W "$(yx completions 2>/dev/null)" -- "$cur"))
            fi
            ;;
    esac
}

complete -F _yx_completions yx
