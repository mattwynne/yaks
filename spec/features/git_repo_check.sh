# shellcheck shell=bash
Describe 'yx git repository check'
  It 'shows error when run outside a git repository'
    temp_dir=$(mktemp -d)
    When run sh -c "cd '$temp_dir' && yx ls"
    The status should be failure
    The error should include "Error: not in a git repository"
    rm -rf "$temp_dir"
  End
End
