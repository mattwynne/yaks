#compdef yx
_yx() {
    local -a candidates
    candidates=(${(f)"$(yx completions -- "${words[@]}" 2>/dev/null)"})
    _describe 'completions' candidates
}
compdef _yx yx
