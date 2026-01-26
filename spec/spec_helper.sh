# shellcheck shell=sh

# Defining variables and functions here will affect all specfiles.
# Change shell options inside a function may cause different behavior,
# so it is better to set them here.
# set -eu

# Disable git hooks for all test repositories
# This prevents pre-commit hooks (like git-mit) from interfering with test setup
export GIT_CONFIG_PARAMETERS="'core.hooksPath=/dev/null'"

# Prevent tests from polluting the main repository's git refs
# Set GIT_CEILING_DIRECTORIES to stop git from finding the main repo
# when tests use temp directories
export GIT_CEILING_DIRECTORIES="$(pwd)"

# This callback function will be invoked only once before loading specfiles.
spec_helper_precheck() {
  # Available functions: info, warn, error, abort, setenv, unsetenv
  # Available variables: VERSION, SHELL_TYPE, SHELL_VERSION
  : minimum_version "0.28.1"
}

# This callback function will be invoked after a specfile has been loaded.
spec_helper_loaded() {
  :
}

# This callback function will be invoked after core modules has been loaded.
spec_helper_configure() {
  # Available functions: import, before_each, after_each, before_all, after_all
  : import 'support/custom_matcher'

  # Run all tests from a temp directory to prevent git pollution
  # Add bin to PATH so tests can call yx directly
  before_all 'TEST_PROJECT_DIR=$(pwd) && export PATH="$TEST_PROJECT_DIR/bin:$PATH" && TEST_WORK_DIR=$(mktemp -d) && cd "$TEST_WORK_DIR"'
  after_all 'cd "$TEST_PROJECT_DIR" && rm -rf "$TEST_WORK_DIR"'
}
