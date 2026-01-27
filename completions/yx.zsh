#compdef yx
# Zsh completion for yx (yak CLI)
# To enable: source this file or add to your .zshrc:
#   source /path/to/yaks/completions/yx.zsh

_yx() {
    local curcontext="$curcontext" state line
    typeset -A opt_args

    local -a commands
    commands=(
        'add:Add a new yak'
        'list:List yaks'
        'ls:List yaks (alias)'
        'done:Mark a yak as done'
        'rm:Remove a yak'
        'move:Move a yak'
        'mv:Move a yak (alias)'
        'prune:Remove completed yaks'
        'context:Show or edit yak context'
        'completions:Generate completions'
        '--help:Show help'
    )

    _arguments -C \
        '1: :->command' \
        '*: :->args'

    case $state in
        command)
            _describe -t commands 'yx commands' commands
            ;;
        args)
            case ${words[2]} in
                done)
                    # Check if --undo is already present
                    if [[ ${words[(I)--undo]} -gt 0 ]]; then
                        # Complete with done yaks
                        local -a done_yaks
                        done_yaks=(${(f)"$(yx ls --format plain --only done 2>/dev/null)"})
                        _describe -t yaks 'done yaks' done_yaks
                    else
                        # Offer --undo flag and incomplete yaks
                        local -a incomplete_yaks
                        incomplete_yaks=(${(f)"$(yx ls --format plain --only not-done 2>/dev/null)"})
                        _alternative \
                            'flags:flags:(--undo)' \
                            'yaks:incomplete yaks:_describe -t yaks "incomplete yaks" incomplete_yaks'
                    fi
                    ;;
                rm|move|mv)
                    # Complete with all yaks
                    local -a all_yaks
                    all_yaks=(${(f)"$(yx ls --format plain 2>/dev/null)"})
                    _describe -t yaks 'yaks' all_yaks
                    ;;
                context)
                    # Offer --show, --edit flags and yak names
                    local -a all_yaks
                    all_yaks=(${(f)"$(yx ls --format plain 2>/dev/null)"})
                    _alternative \
                        'flags:flags:(--show --edit)' \
                        'yaks:yaks:_describe -t yaks "yaks" all_yaks'
                    ;;
            esac
            ;;
    esac
}

compdef _yx yx
