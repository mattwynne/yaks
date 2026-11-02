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

## Agent skills for contributors

The skills in [`.agents/skills/`](.agents/skills/) describe how we maintain
Yaks itself. They are contributor tooling, not a plugin or marketplace
for Yaks users. General usage and yak-mapping guidance lives in `yx --help`
and command-specific help.

Claude Code discovers these project skills through the checked-in
`.claude/skills -> ../.agents/skills` directory symlink. Start `claude` in
this checkout after loading the development environment; no plugin or
Superpowers installation is needed. Other agents can read the same files.

- [prepare](.agents/skills/prepare/SKILL.md): agree scope and decisions,
  use example mapping and BDD formulation, and record implementable work
  on the yak. In Claude Code, invoke `/prepare`.
- [work](.agents/skills/work/SKILL.md): claim the yak, work in isolation,
  follow one-test-at-a-time TDD, verify, and merge with `dev merge`.
  In Claude Code, invoke `/work`.

Supporting skills cover BDD discovery/formulation, CQRS and event sourcing,
mutation triage, exploratory testing, sandboxing, and domain language.
These workflows use the repository's commands and the current agent's
tools; delegation is optional.

From an agent shell without direnv loaded, run checks with
`devenv shell -- bin/dev check`. `yx start` only changes a yak's state;
`dev start` also creates a worktree. Do not confuse those commands.

## Changelog

Add user-visible changes to [CHANGELOG.md](CHANGELOG.md) under
`[Unreleased]` in the appropriate section.

## Releases

See [docs/releases.md](docs/releases.md) for the versioning policy,
release channels, changelog discipline, and intended no-think release
workflow.

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
