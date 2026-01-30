Describe 'install.sh'
  It 'installs yx from release zip and runs smoke tests'
    run_install() {
      docker build -t yx-installer-test-base -f "$TEST_PROJECT_DIR/spec/features/Dockerfile.installer-test" "$TEST_PROJECT_DIR"

      docker run --rm \
        -v "$TEST_PROJECT_DIR:/workspace" \
        -w /workspace \
        -e YX_SOURCE="/workspace/release/yx.zip" \
        -e YX_SHELL_CHOICE="2" \
        -e YX_AUTO_COMPLETE="n" \
        yx-installer-test-base \
        bash -c '
          ./install.sh
          echo "=== Smoke tests ==="
          yx --help
          cd /tmp
          git init -q .
          git config user.email "test@example.com"
          git config user.name "Test"
          echo ".yaks" > .gitignore
          yx add foo
          yx ls
        '
    }
    When call run_install
    The status should be success
    The output should include "Installing yx"
    The output should include "=== Smoke tests ==="
    The output should include "foo"
  End
End
