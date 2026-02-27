# Yaks - An iterative, emergent, non-linear TODO list for humans and robots

> It is in the doing of the work that we discover the work that we must do
> 
> -- Woody Zuill, https://agilemaxims.com

Yaks is command-line tool for managing a _Yak Map_ - a TODO list of nested goals - designed for teams of humans and robots working on software projects together.

![demo](demo.gif)

Yaks uses a hidden git ref to sync your yaks, allowing you to share the latest state of the work across branches and clones of your repo.

## Usage


```bash
yx add Fix the bug          # Add a new yak
yx context Fix the bug      # Add context/notes
yx show Fix the bug         # Show yak details
yx ls                       # Show all yaks
yx done Fix the bug         # Mark as complete
yx rm Fix the bug           # Remove a yak
yx prune                    # Remove all done yaks
```

## Why "Yaks"?

A Yak Map is basically the same as a [Mikado Graph](https://mikadomethod.info) or a [Discovery Tree](https://www.fastagile.io/method/product-mapping-and-discovery-trees). But I like calling it a Yak Map, because yak shaving is what we do all day in software.

![image](https://github.com/user-attachments/assets/1e935831-7807-4127-a698-3fdb50615080)

## Isn't this just like Beads?

I've been using Yak Maps for several years working on teams of humans. We just used to cobble something together in Miro or whatever. [Beads](https://github.com/steveyegge/beads) was the first tool I've seen that supports this kind of acyclic graph for managing work, and I've found it hugely inspiring in this robot-driven era.

But beads has some shortcomings, for me:

* I like my software simple. I want my tools to do one thing well, and have minimal code and feeatures. Beads, for me, is over-featured and complicated.

* Yaks all the way down. There are no classifications of task here: epics, stories, tasks and whatnot. Everything is a yak.

* No more committing your plan to git. Yaks uses a hidden git ref to sync changes, so with `yx sync` anyone with a clone of the repo and a connection to `origin` can be working off the same list at the same time.

## Installation

### Quick Install (macOS/Linux)

```bash
curl -fsSL https://raw.githubusercontent.com/mattwynne/yaks/main/install.sh | bash
```

The installer downloads the latest release, validates the checksum and installs the binary in `/usr/local/bin`. It also installs shell completions so you can tab-complete yak names.

### Development Setup

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

Git hooks will prevent commits, merges, and pushes without recent verification.

### Testing

```bash
dev check               # Run all checks (tests + lint + audit)
cargo test --features test-support  # Cucumber + unit tests
shellspec               # ShellSpec tests (tmux, git, installer)
```

### Mutation Testing

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

## License

[MIT](LICENSE)
