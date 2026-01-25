# Yak - DAG-based TODO List

A CLI tool for managing TODO lists as a directed acyclic graph (DAG), designed for teams working on software projects.

The name comes from "yak shaving" - when you set out to do task A but discover you need B first, which requires C, creating chains of dependencies.

## Installation

### Quick Install (macOS/Linux)

```bash
curl -fsSL https://raw.githubusercontent.com/mattwynne/yaks/main/install.sh | bash
```

### Manual Install

1. Clone the repository
2. Add `bin/` to your PATH
3. Source the completion script in your shell config:
   ```bash
   source /path/to/yaks/completions/yx.bash
   ```

### Development Setup

Uses direnv to automatically configure PATH and completions:

```bash
git clone https://github.com/mattwynne/yaks.git
cd yaks
direnv allow
```

## Usage

```bash
yx add "Fix the bug"        # Add a new yak
yx list                     # Show all yaks
yx done "Fix the bug"       # Mark as complete
yx context "Fix the bug"    # Add context/notes
yx rm "Fix the bug"         # Remove a yak
yx prune                    # Remove all done yaks
```

Tab completion works for yak names after sourcing the completion script.

## Project Status

**Active development** - This tool is being used to build itself (dogfooding). See `.yaks/` for the actual work tracker.

## Testing

Uses [ShellSpec](https://shellspec.info/) for testing:

```bash
shellspec
```

## License

[Add license here]
