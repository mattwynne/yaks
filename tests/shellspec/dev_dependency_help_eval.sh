# shellcheck shell=bash

Describe 'bin/dev eval-dependency-help'
  setup_dependency_eval() {
    EVAL_TMP=$(mktemp -d)
    export EVAL_TMP
    mkdir -p "$EVAL_TMP/bin"
    cat > "$EVAL_TMP/bin/cargo" <<'EOF'
#!/usr/bin/env bash
# dev check builds the real binary before ShellSpec; avoid a concurrent rebuild.
exit 0
EOF
    chmod +x "$EVAL_TMP/bin/cargo"
    cat > "$EVAL_TMP/bin/codex" <<'EOF'
#!/usr/bin/env bash
cat >/dev/null
echo 'I created the perfect yak map.'
EOF
    chmod +x "$EVAL_TMP/bin/codex"
  }

  cleanup_dependency_eval() {
    rm -rf "$EVAL_TMP"
  }

  BeforeEach 'setup_dependency_eval'
  AfterEach 'cleanup_dependency_eval'

  It 'fails when the model claims success but creates no yaks'
    When run bash -c '
      export PATH="'"$EVAL_TMP"'/bin:$PATH"
      export YX_EVAL_PROVIDER=codex
      export YX_DEV_SOURCE_ONLY=1
      source "'"$TEST_PROJECT_DIR"'/bin/dev"
      eval-dependency-help
    '
    The status should be failure
    The output should include 'Eval artifacts:'
    The error should include 'FAIL: map did not use the expected prerequisite tree'
  End

  It 'passes by inspecting a map created through real yx commands'
    cat > "$EVAL_TMP/bin/codex" <<'EOF'
#!/usr/bin/env bash
set -eu
cat >/dev/null
test "$PWD" = "$YX_ROOT"
yx --help >/dev/null
bash "$TEST_PROJECT_DIR/tests/fixtures/dependency-help-map.sh" >/dev/null
echo 'The final answer is prose, not map JSON.'
EOF
    When run bash -c '
      export PATH="'"$EVAL_TMP"'/bin:$PATH"
      export YX_EVAL_PROVIDER=codex
      export YX_DEV_SOURCE_ONLY=1
      source "'"$TEST_PROJECT_DIR"'/bin/dev"
      eval-dependency-help
    '
    The status should be success
    The output should include 'PASS: prerequisite nesting guidance'
  End

  It 'accepts a shared prerequisite nested in one consuming branch'
    cat > "$EVAL_TMP/bin/codex" <<'EOF'
#!/usr/bin/env bash
set -eu
cat >/dev/null
bash "$TEST_PROJECT_DIR/tests/fixtures/dependency-help-map.sh" >/dev/null
./bin/yx move "Publish shared contract" --under "Implement reader" >/dev/null
EOF
    When run bash -c '
      export PATH="'"$EVAL_TMP"'/bin:$PATH"
      export YX_EVAL_PROVIDER=codex
      export YX_DEV_SOURCE_ONLY=1
      source "'"$TEST_PROJECT_DIR"'/bin/dev"
      eval-dependency-help
    '
    The status should be success
    The output should include 'PASS: prerequisite nesting guidance'
  End
End
