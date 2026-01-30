# shellcheck shell=bash
Describe 'yx completions install'
  It 'shows help when no shell detected'
    SHELL="/bin/unknown"
    When run yx completions install
    The status should equal 1
    The error should include "Could not detect shell"
  End

  It 'detects bash and shows install path'
    SHELL="/bin/bash"
    When run yx completions install --dry-run
    The status should equal 0
    The output should include ".bashrc"
    The output should include "completions/yx.bash"
  End

  It 'detects zsh and shows install path'
    SHELL="/bin/zsh"
    When run yx completions install --dry-run
    The status should equal 0
    The output should include ".zshrc"
    The output should include "completions/yx.zsh"
  End

  It 'fails gracefully when unable to write to rc file'
    SHELL="/bin/bash"
    HOME="/nonexistent"
    When run yx completions install
    The status should equal 1
    The error should include "Failed to write"
    The error should include "add the following line manually"
    The error should include "source"
  End
End
