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

  It 'exposes dev eval with eight current-copy attempts and four workers by default'
    cat > "$EVAL_TMP/bin/python3" <<'EOF'
#!/usr/bin/env bash
printf '%s\n' "$@"
EOF
    chmod +x "$EVAL_TMP/bin/python3"
    When run env PATH="$EVAL_TMP/bin:$PATH" "$TEST_PROJECT_DIR/bin/dev" eval
    The status should be success
    The output should include '--current-only'
    The output should include '--working-tree'
    The output should include '--runs'
    The output should include '8'
    The output should include '--jobs'
    The output should include '4'
  End

  It 'accepts a baseline and caller-selected run settings'
    cat > "$EVAL_TMP/bin/python3" <<'EOF'
#!/usr/bin/env bash
printf '%s\n' "$@"
EOF
    chmod +x "$EVAL_TMP/bin/python3"
    When run env PATH="$EVAL_TMP/bin:$PATH" "$TEST_PROJECT_DIR/bin/dev" eval \
      --compare d5c4bc4 --runs 2 --jobs 1 --model test-model --timeout 60 --output /tmp/eval-output
    The status should be success
    The output should include '--old-ref'
    The output should include 'd5c4bc4'
    The output should not include '--current-only'
    The output should include 'test-model'
    The output should include '/tmp/eval-output'
  End

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
