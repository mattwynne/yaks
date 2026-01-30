Complete rewrite of yx CLI in a compiled language.

**Goals:**
- Portability: Single binary, easy to install
- Maintainability: Clean multi-file structure, easy to understand
- Growth potential: Access to mature TUI libraries for future features

**Candidate languages:** Zig, Nim, Rust, Go

**Approach:** Build minimal prototypes in each language to evaluate through actual experience rather than theory.

**Evaluation criteria:**
- How easy to get started?
- How clean is the code?
- Compile time and binary size?
- TUI library ecosystem?
- Day-to-day development experience?

**Prototype scope:**
- All core commands (add, list, done, rm, prune, move, context)
- File system storage (.yaks/ directories)
- Git ref syncing (sync command)
- Feature test compatibility

**Status:** Need to separate feature tests first so they can validate all implementations.
