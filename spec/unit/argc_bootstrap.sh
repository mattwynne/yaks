# shellcheck shell=bash
Describe 'Argc bootstrap'
  It 'has argc available in development environment'
    When call command -v argc
    The status should be success
    The output should include "argc"
  End
End
