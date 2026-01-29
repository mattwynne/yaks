Separate tests into two categories:

1. **Feature tests** - Black-box CLI tests that can be reused across all language implementations (Zig, Nim, Rust, Go)
   - Test the CLI interface: commands, arguments, output format
   - Test file system effects: what gets created in .yaks/
   - Test git ref behavior (sync command)
   - These should work identically regardless of implementation language

2. **Unit tests** - Implementation-specific tests for internal functions
   - Currently ShellSpec tests for bash functions
   - Will need to be rewritten per language
   - Test internal logic, edge cases, helpers

**Goal**: Create a feature test suite that can validate any implementation of yx, then use it to verify rewrites in Zig/Nim/Rust/Go.

**Current state**: All tests are in spec/ directory using ShellSpec. Need to identify which are truly feature tests vs unit tests.
