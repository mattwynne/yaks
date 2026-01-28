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

# Helper function to set up gitignore for .yaks in a repo
# Usage: setup_gitignore_for_yaks /path/to/repo
setup_gitignore_for_yaks() {
  local repo_path="$1"
  echo ".yaks" > "$repo_path/.gitignore"
  git -C "$repo_path" add .gitignore
  git -C "$repo_path" commit --quiet -m "Add .gitignore" 2>/dev/null || true
}

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

# Set up a clean test environment with a git repo
setup_test_environment() {
  TEST_PROJECT_DIR=$(pwd)
  export PATH="$TEST_PROJECT_DIR/bin:$PATH"
  TEST_WORK_DIR=$(mktemp -d)
  cd "$TEST_WORK_DIR"
  git init --quiet
  git config user.email "test@example.com"
  git config user.name "Test User"
  setup_gitignore_for_yaks "."
}

# Clean up test environment
teardown_test_environment() {
  cd "$TEST_PROJECT_DIR"
  rm -rf "$TEST_WORK_DIR"
}

# This callback function will be invoked after core modules has been loaded.
spec_helper_configure() {
  # Available functions: import, before_each, after_each, before_all, after_all
  : import 'support/custom_matcher'

  before_all 'setup_test_environment'
  after_all 'teardown_test_environment'
}
