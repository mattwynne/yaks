# shellcheck shell=bash
Describe 'yx gitignore check'
  It 'shows error when .yaks is not gitignored'
    temp_dir=$(mktemp -d)
    cd "$temp_dir" || return
    git init --initial-branch=main
    When run yx ls
    The status should be failure
    The error should include "Error: .yaks folder is not gitignored"
    cd - || return
    rm -rf "$temp_dir"
  End
End
