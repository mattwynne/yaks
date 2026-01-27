Describe 'yx git availability check'
  It 'shows error when git command is not available'
    # Create a temp directory with only yx and essential tools, but not git
    temp_bin=$(mktemp -d)
    ln -s "$(which yx)" "$temp_bin/yx"
    ln -s /bin/sh "$temp_bin/sh"
    ln -s /bin/bash "$temp_bin/bash" 2>/dev/null || true

    # Use PATH with only our temp dir - no git available
    When run sh -c "PATH='$temp_bin' yx ls"
    The status should be failure
    The error should include "Error: git command not found"

    rm -rf "$temp_bin"
  End
End
