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

# Helper function to set up a git test repository
# Usage: setup_test_repo /path/to/repo [user_email] [user_name] [origin_url]
# Arguments:
#   repo_path: Path where the repo should be created
#   user_email: Git user email (default: "test@example.com")
#   user_name: Git user name (default: "Test User")
#   origin_url: Optional origin remote URL
setup_test_repo() {
  local repo_path="$1"
  local user_email="${2:-test@example.com}"
  local user_name="${3:-Test User}"
  local origin_url="${4:-}"

  git -C "$repo_path" init --quiet
  git -C "$repo_path" config user.email "$user_email"
  git -C "$repo_path" config user.name "$user_name"

  if [ -n "$origin_url" ]; then
    git -C "$repo_path" remote add origin "$origin_url"
  fi

  echo ".yaks" > "$repo_path/.gitignore"
  git -C "$repo_path" add .gitignore
  git -C "$repo_path" commit --quiet -m "Add .gitignore"
}

# Helper function to set up gitignore for .yaks in a repo
# Usage: setup_gitignore_for_yaks /path/to/repo
# Deprecated: Use setup_test_repo() instead for new code
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
  setup_test_repo "."
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
