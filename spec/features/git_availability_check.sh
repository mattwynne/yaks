# shellcheck shell=bash
Describe 'yx git availability check'
  It 'shows error when git command is not available'
    # Test by removing git from PATH while keeping other essential commands
    # We build a minimal PATH that has shell utilities but not git
    temp_bin=$(mktemp -d)

    # Copy or link essential commands (but not git)
    for cmd in sh bash dirname basename readlink realpath pwd cat mkdir rm ls head tail grep sed awk argc; do
      cmd_path=$(command -v $cmd 2>/dev/null || true)
      if [ -n "$cmd_path" ] && [ -f "$cmd_path" ]; then
        ln -sf "$cmd_path" "$temp_bin/$cmd" 2>/dev/null || true
      fi
    done

    # Also need to make yx itself available
    ln -sf "$(command -v yx)" "$temp_bin/yx"

    # Use only our temp_bin in PATH - git will not be found
    When run sh -c "PATH='$temp_bin' yx ls"
    The status should be failure
    The error should include "Error: git command not found"

    rm -rf "$temp_bin"
  End
End
