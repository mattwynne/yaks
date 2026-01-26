Describe 'yak directories'
  BeforeEach 'export YAKS_PATH=$(mktemp -d)'
  AfterEach 'rm -rf "$YAKS_PATH"'

  It 'have context.md file by default'
    yx add "test yak"

    When call sh -c "find '$YAKS_PATH/test yak' -type f -name 'context.md' | wc -l"
    The output should equal "1"
  End
End
