Describe 'install.sh'
  It 'installs yx from release zip and runs smoke tests'
    run_install() {
      docker run --rm \
        -v "$TEST_PROJECT_DIR:/workspace" \
        -w /workspace \
        -e YX_SOURCE="/workspace/release/yx.zip" \
        -e YX_SHELL_CHOICE="2" \
        -e YX_AUTO_COMPLETE="n" \
        ubuntu:22.04 \
        bash -c '
          apt-get update -qq 2>/dev/null
          apt-get install -y -qq curl bash unzip git 2>/dev/null
          ./install.sh 2>&1
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
