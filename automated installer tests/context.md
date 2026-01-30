# Automated Installer Tests

## Goal
Create automated tests for install.sh that can run in CI and test different scenarios:
- Different shells (bash, zsh)
- Different package managers/installation methods
- Different distros
- Interactive prompt handling

## Approach: ShellSpec + Docker
Since we already use ShellSpec for testing yx itself, extend it to test the installer.

**Phase 1: ShellSpec Tests (START HERE)**
1. Add non-interactive mode to install.sh via environment variables:
   - YX_SHELL_CHOICE (1=zsh, 2=bash)
   - YX_AUTO_COMPLETE (y/n)
   - YX_SOURCE (allow specifying a local path for release artefacts instead of downloading)
2. Create spec/install_spec.sh
3. Create spec/support/docker/ with container for Ubuntu
4. basic test in spec/install_spec.sh that runs the install script on the docker image and verifies that you can run a few smoke tests using the installed yx binary. (e.g. `git init . && yx add foo && yx ls`)

**Phase 2: Docker Multi-Distro Testing**
- Create test/docker/ with containers for Ubuntu, Debian, Alpine, and different shells (bash, zsh)
- Wrapper script test/test-all-distros.sh
3. Test core scenarios with all distros

**Phase 3: CI Integration**
- GitHub Actions workflow
- Matrix strategy (shells × distros)
- Test both local and GitHub download modes

**Phase 4: Interactive Testing (Optional)**
- Expect scripts for /dev/tty prompt testing
- Edge cases (invalid input, EOF)

### Key Testing Patterns

**Isolated Test Setup:**
```bash
setup() {
  TEST_HOME=$(mktemp -d)
  export HOME="$TEST_HOME"
  export PATH="$TEST_HOME/.local/bin:$PATH"
}
```

**Test Matrix:**
- Shells: bash, zsh
- Install locations: /usr/local/bin (writable), ~/.local/bin (fallback)
- Sources: local repo vs GitHub download
- Prompts: shell choice, completion setup
- PATH scenarios: already in PATH vs not

**Docker Testing Example:**
```bash
for shell in bash zsh; do
  docker run --rm -v $(pwd):/app -w /app ubuntu:22.04 bash -c "
    apt-get update -qq && apt-get install -y -qq curl $shell
    export YX_SHELL_CHOICE=2 YX_AUTO_COMPLETE=n
    ./install.sh
    yx --help
  "
done
```

### Handling Interactive Prompts

Our install.sh uses `/dev/tty` for prompts (lines 43, 96), which bypasses stdin.

**Solution**: Add environment variable support:
```bash
if [ -n "$YX_SHELL_CHOICE" ]; then
    SHELL_CHOICE="$YX_SHELL_CHOICE"
else
    read -p "Choice [$DEFAULT_CHOICE]: " SHELL_CHOICE </dev/tty
fi
```

Alternative: Use Expect scripts for testing actual interactive behavior.

### CI/CD Integration

GitHub Actions matrix example:
```yaml
strategy:
  matrix:
    shell: [bash, zsh]
    distro: [ubuntu:22.04, ubuntu:24.04, debian:12]
```

### Real-World Examples
- NVM: Multi-shell testing with GitHub Actions
- Rustup: Docker for Linux distro testing
- Homebrew: Extensive CI with containers

## Implementation Priority

1. Add env var support to install.sh (non-breaking change)
2. Create basic ShellSpec tests in spec/install_spec.sh
3. Add Docker Compose setup for multi-distro testing
4. Add GitHub Actions workflow
5. (Optional) Add Expect scripts for interactive testing

## References
- ShellSpec: https://shellspec.info/
- BATS: https://github.com/bats-core/bats-core
- Docker testing patterns: https://www.linux.com/training-tutorials/testing-simple-scripts-docker-container/
