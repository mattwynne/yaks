# ShellSpec Mastery

**BDD-style testing for shell scripts with professional precision**

## Core Philosophy

ShellSpec is a full-featured BDD framework for POSIX shells. Write expressive tests that document behavior while catching regressions.

## Quick Reference

### Test Structure

```bash
Describe 'feature description'
  It 'does something specific'
    When call function_name arg1 arg2
    The output should equal "expected"
    The status should be success
  End
End
```

### Evaluation Methods

| Method | Use Case | Subshell | Best For |
|--------|----------|----------|----------|
| `When call` | Function testing | No | Unit tests, fast feedback |
| `When run` | Isolated execution | Yes | Integration, side effects |
| `When run command` | External commands | Yes | CLI tools, binaries |
| `When run script` | Shell scripts | Yes | Testing .sh files |
| `When run source` | Sourced scripts | Yes | Enables mocking with Intercept |

**Rule of thumb**: Use `call` for functions, `run` for everything else.

### Common Subjects

Extract aspects to test:

- `output` / `stdout` - Standard output
- `error` / `stderr` - Error output
- `status` - Exit code
- `line N` - Specific output line
- `word N` - Specific word
- `variable VAR` - Variable value
- `path PATH` - Resolved file path
- `length` - String/array length
- `contents` - File contents

### Essential Matchers

**Status & Equality**
```bash
The status should be success        # Exit 0
The status should be failure        # Exit non-zero
The output should equal "exact"     # Exact match
The output should include "partial" # Contains
```

**Pattern Matching**
```bash
The output should match pattern "^[0-9]+$"  # Regex
The output should start with "prefix"
The output should end with "suffix"
```

**File System**
```bash
The path "/tmp/file" should be file
The path "/tmp/dir" should be directory
The file "/tmp/data" should be exist
The file "/tmp/empty" should be empty file
The path "/tmp/script" should be executable
```

**Variables**
```bash
The variable VAR should be defined
The variable VAR should be present    # Defined and non-empty
The variable VAR should be blank      # Empty or whitespace
The variable VAR should be exported
```

**Negation**
```bash
The output should not equal "wrong"
The status should not be success
```

### Modifiers

Transform subjects before testing:

```bash
The line 2 of output should equal "second line"
The word 3 of output should equal "third-word"
The length of output should equal 42
The contents of file "data.txt" should include "needle"
```

## Test Organization

### Grouping

```bash
Describe 'Component'
  Context 'when condition X'
    It 'behaves like Y'
      # test
    End
  End

  Context 'when condition Z'
    It 'behaves like W'
      # test
    End
  End
End
```

**Aliases**:
- `Describe` / `Context` / `ExampleGroup` (grouping)
- `It` / `Specify` / `Example` (tests)

### Hooks

Setup and teardown at different scopes:

```bash
Describe 'Feature'
  # Run once before all tests in this block
  BeforeAll
    setup_database
  End

  # Run before each test
  BeforeEach
    create_temp_file
  End

  It 'test 1'
    # test
  End

  It 'test 2'
    # test
  End

  # Run after each test
  AfterEach
    cleanup_temp_file
  End

  # Run once after all tests
  AfterAll
    teardown_database
  End
End
```

**Hook Types**:
- `BeforeAll` / `AfterAll` - Once per group
- `Before` / `After` - Alias for BeforeEach/AfterEach
- `BeforeEach` / `AfterEach` - Per test
- `BeforeCall` / `AfterCall` - Around `When call`
- `BeforeRun` / `AfterRun` - Around `When run`

## Parameterized Tests

### Data-Driven Testing

```bash
Describe 'validation'
  Parameters
    "valid@email.com"   success
    "invalid"           failure
    ""                  failure
  End

  It "validates email $1"
    When call validate_email "$1"
    The status should be "$2"
  End
End
```

**Formats**:
- `Parameters:block` - Block format (default)
- `Parameters:value` - Single values
- `Parameters:matrix` - Cartesian product
- `Parameters:dynamic` - Generate from function

### Data Directive

```bash
Describe 'parsing'
  Data
    #|line1
    #|line2
    #|line3
  End

  It 'processes multiline input'
    When call process_data
    The line 1 of output should equal "processed line1"
  End
End
```

**Data Sources**:
- `Data` - Heredoc style
- `Data:raw` - No variable expansion
- `Data:expand` - With expansion
- `Data < "file.txt"` - From file
- `Data "inline text"` - Inline string

## Mocking & Stubbing

### Function Mocking

```bash
Describe 'with mock'
  mock_function() {
    echo "mocked output"
  }

  It 'uses mock instead of real function'
    When call code_that_calls_mock_function
    The output should include "mocked output"
  End
End
```

### Command Intercepts

```bash
Describe 'command mocking'
  Intercept curl
    # Fake curl command
    echo '{"status": "ok"}'
  End

  It 'uses intercepted curl'
    When run source api_client.sh
    The output should include "ok"
  End
End
```

**Note**: Intercept requires `When run source` (not `call` or `run`).

## Running Tests

### Basic Execution

```bash
shellspec                    # Run all specs in spec/
shellspec spec/util_spec.sh  # Run specific file
shellspec spec/             # Run directory
```

### Quick Workflows

```bash
# TDD Red-Green-Refactor
shellspec --fail-fast        # Stop at first failure
shellspec --next-failure     # Run failures then stop at first

# Quick Mode (rerun failures)
shellspec --quick            # Auto-enabled after first run
shellspec --repair           # Run only failures
```

### Targeted Execution

```bash
# By line number
shellspec spec/file_spec.sh:42

# By example ID
shellspec spec/file_spec.sh:@1-5

# By name pattern
shellspec --example "specific test"

# By tag
shellspec --tag unit
shellspec --tag integration

# Focus on specific tests (add 'f' prefix)
fDescribe 'focused group'
  fIt 'focused test'
    # Only focused tests run with --focus
  End
End
shellspec --focus
```

### Parallel Execution

```bash
shellspec --jobs 4           # Run 4 parallel jobs
                             # Parallelizes at specfile level
```

**Tip**: Organize tests into multiple files to maximize parallel benefit.

### Output Formats

```bash
shellspec                    # Progress (dots)
shellspec -f d               # Documentation style
shellspec -f t               # TAP format
shellspec -f j               # JUnit XML
shellspec -f failures        # File:line:message (editor integration)
```

### Coverage

```bash
shellspec --kcov             # Enable coverage with kcov
shellspec --kcov --kcov-options "--include-pattern=.sh"
```

**Requirements**: kcov installed, specific shells only (bash, zsh, ksh).

## Advanced Features

### Skip & Pending

```bash
# Skip a test
xIt 'not ready yet'
  # skipped
End

# Skip conditionally
It 'requires bash'
  Skip if "not bash" [ "$SHELLSPEC_SHELL_TYPE" != "bash" ]
  # test
End

# Pending (temporary)
Pending "waiting for bug fix"
It 'will work after fix'
  # test
End

# TODO (permanent marker)
Todo "future enhancement"
It 'planned feature'
  # test
End
```

### Directives

```bash
# Constants
%const FIXTURE_DIR:/path/to/fixtures

# Text blocks
%text
#|Multi
#|line
#|text
%end

# Preserve variables across subshells
%preserve VAR

# Debug logging
%logger info "Debug message"
```

### Helpers

```bash
# In spec_helper.sh
Include lib/helpers.sh

# In tests
Describe 'using helpers'
  Include lib/custom_helpers.sh

  It 'can use included functions'
    When call helper_function
    The status should be success
  End
End
```

## Integration with TDD Workflow

### Red-Green-Refactor with ShellSpec

1. **Write ONE failing test**
   ```bash
   It 'validates empty input'
     When call validate ""
     The status should be failure
   End
   ```

2. **Run tests - watch it fail (RED)**
   ```bash
   shellspec --fail-fast
   ```

3. **Write minimal code to pass**
   ```bash
   validate() {
     [ -n "$1" ]
   }
   ```

4. **Run ALL tests - verify pass (GREEN)**
   ```bash
   shellspec
   ```

5. **Refactor if needed**

6. **Commit**

### Fast Feedback Loop

```bash
# During active development
shellspec --fail-fast --format documentation

# Quick mode automatically tracks failures
shellspec --quick

# Fix and rerun only failures
shellspec --repair
```

## Project Setup

### Initialize Project

```bash
shellspec --init              # Create .shellspec and spec/spec_helper.sh
shellspec --init spec git     # Also create example spec and .gitignore
```

### Directory Structure

```
project/
├── .shellspec              # Project options
├── bin/
│   └── yak                 # Script under test
├── spec/
│   ├── spec_helper.sh      # Shared setup
│   ├── yak_spec.sh         # Test file
│   └── support/
│       └── fixtures/       # Test data
├── coverage/               # Coverage reports (generated)
└── report/                 # Test reports (generated)
```

### Configuration (.shellspec)

```bash
# Common options
--require spec_helper
--format documentation
--color
--fail-fast
--example "fast tests"
```

## Best Practices

### DO

- ✅ Use `call` for functions (fast, no subshell overhead)
- ✅ Use `run` for commands and side effects
- ✅ Write focused, single-assertion tests
- ✅ Organize tests into multiple files for parallel execution
- ✅ Use descriptive test names that document behavior
- ✅ Test edge cases and error conditions
- ✅ Use hooks for setup/teardown
- ✅ Run full suite regularly (not just --fail-fast)

### DON'T

- ❌ Test implementation details (test behavior)
- ❌ Mix unit and integration tests in same file
- ❌ Skip tests without tracking (use Skip/Pending/Todo)
- ❌ Use complex logic in test assertions
- ❌ Ignore test failures ("flaky tests")
- ❌ Write tests that depend on execution order
- ❌ Mock everything (balance isolation with realism)

## Common Patterns

### Testing Exit Codes

```bash
It 'succeeds on valid input'
  When call process "valid"
  The status should be success
End

It 'fails on invalid input'
  When call process "invalid"
  The status should be failure
  The error should include "Invalid"
End
```

### Testing Output

```bash
It 'outputs expected format'
  When call generate_report
  The line 1 should equal "Report Header"
  The line 2 should match pattern "^Date: [0-9-]+"
  The lines should equal 10
End
```

### Testing Files

```bash
It 'creates output file'
  When call export_data "/tmp/export.csv"
  The path "/tmp/export.csv" should be file
  The contents of file "/tmp/export.csv" should include "column1,column2"
End
```

### Testing Variables

```bash
It 'sets environment variable'
  When call setup_environment
  The variable CONFIG_LOADED should be present
  The variable CONFIG_LOADED should equal "true"
End
```

### Testing Functions Exist

```bash
It 'defines required functions'
  When call source ./lib.sh
  The function "validate" should be defined
End
```

## Debugging Tests

### Trace Execution

```bash
shellspec --xtrace spec/file_spec.sh     # Full trace
shellspec --xtrace-only                  # Skip assertions
```

### Dry Run

```bash
shellspec --dry-run                      # Show what would run
```

### Logging

```bash
# In tests
%logger debug "Value of x: $x"
%logger info "Checkpoint reached"
```

### Syntax Check

```bash
shellspec --syntax-check                 # Parse without running
```

## Troubleshooting

### Test Not Running

- Check file naming: `*_spec.sh` pattern
- Verify file location: under `spec/` directory
- Check for syntax errors: `shellspec --syntax-check`

### Unexpected Failures

- Run with `--xtrace` to see execution
- Check variable scope (subshell issues)
- Verify hooks aren't interfering
- Use `--format documentation` for clarity

### Mock Not Working

- Use `Intercept` only with `When run source`
- Verify function scope
- Check if command is aliased

### Performance Issues

- Profile slow tests: `shellspec --profile`
- Enable parallel execution: `shellspec --jobs N`
- Split large spec files
- Optimize expensive setup in hooks

## Resources

- Official Site: https://shellspec.info/
- GitHub: https://github.com/shellspec/shellspec
- Documentation: https://github.com/shellspec/shellspec/blob/master/docs/
- References: https://github.com/shellspec/shellspec/blob/master/docs/references.md

## Integration with Yak Project

This skill works with:
- `incremental-tdd` - One test at a time, red-green-refactor
- `discovery-tree-workflow` - Tests reveal complexity, create subtasks
- ShellSpec's `--quick` and `--repair` modes for rapid iteration

## Remember

**ShellSpec is not just a test runner - it's a design tool.**

Write tests that document intended behavior. Let failures guide implementation. Keep tests simple, focused, and maintainable.

**The best test is one that clearly expresses intent and fails obviously when behavior breaks.**
