Describe 'yx context'
  BeforeEach 'export YAK_PATH=$(mktemp -d)'
  AfterEach 'rm -rf "$YAK_PATH"'

  It 'sets context from stdin (default)'
    When run sh -c "
      yx add 'my yak'
      echo '# Some context' | yx context 'my yak'
      yx context --show 'my yak'
    "
    The output should equal "my yak

# Some context"
  End

  It 'shows a yak without context'
    When run sh -c "
      yx add 'my yak'
      yx context --show 'my yak'
    "
    The output should equal "my yak"
  End

  It 'shows a yak with context'
    When run sh -c "
      yx add 'my yak'
      echo '# Some context' | yx context 'my yak'
      yx context --show 'my yak'
    "
    The output should equal "my yak

# Some context"
  End

  It 'replaces existing context from stdin'
    When run sh -c "
      yx add 'my yak'
      echo 'old' | yx context 'my yak'
      echo 'new' | yx context 'my yak'
      yx context --show 'my yak'
    "
    The output should equal "my yak

new"
  End

  It 'shows error when yak not found (edit mode)'
    When run sh -c "
      echo 'context' | yx context 'nonexistent'
    "
    The status should be failure
    The error should include "Error: yak 'nonexistent' not found"
  End

  It 'shows error when yak not found (show mode)'
    When run yx context --show "nonexistent"
    The status should be failure
    The error should include "Error: yak 'nonexistent' not found"
  End

  It 'sets and shows context for nested yak'
    When run sh -c "
      yx add 'parent'
      yx add 'parent/child'
      echo '# Nested context' | yx context 'parent/child'
      yx context --show 'parent/child'
    "
    The output should equal "parent/child

# Nested context"
  End
End
