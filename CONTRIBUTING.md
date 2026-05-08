# Contributing to Yaks

## Development Setup

Uses direnv and devenv to set up the development environment.

```bash
git clone https://github.com/mattwynne/yaks.git
cd yaks
direnv allow
dev setup  # Install git hooks
```

Before committing, always run:

```bash
dev check  # Runs tests and linting
```

Git hooks will prevent commits, merges, and pushes without recent
verification.

## Changelog

Add user-visible changes to [CHANGELOG.md](CHANGELOG.md) under
`[Unreleased]` in the appropriate section.

## Testing

```bash
dev check               # Run all checks (tests + lint + audit)
cargo test --features test-support  # Cucumber + unit tests
shellspec               # ShellSpec tests (tmux, git, installer)
```

## Code Complexity

```bash
dev complexity          # Run code complexity analysis
```

Uses [rust-code-analysis](https://github.com/nickel-org/rust-code-analysis)
to measure cyclomatic complexity, cognitive complexity, and SLOC across
all Rust source files. Reports the top 5 most complex functions.

This also runs automatically as part of `dev check`. Currently it's
visibility-only — no thresholds enforced yet.

## Mutation Testing

```bash
dev mutate-diff         # Fast: only your changes (~seconds)
dev mutate              # Full run (~7 min)
dev mutate-sync         # Sync results to yak tracker
```

Mutation testing validates that your tests actually catch
regressions. Use `dev mutate-diff` during development for
fast feedback. Missed mutants are tracked as yaks under
"fix missed mutants" — run `dev mutate-sync` after a full
run to update them.
