# Yaks (`yx`)

> It is in the doing of the work that we discover the work that we must do
>
> -- Woody Zuill, https://agilemaxims.com

Yaks is a tiny, fast discovery tree for software teams: a command-line way to capture and manage the work you discover while doing the work.

A discovery tree is a tree of nested goals. Put prerequisites underneath the larger goal they unlock, and the tree shows what can happen now, what can happen in parallel, and what is still blocked.

Yaks is built around three values:

- **Simple**: everything is a yak. No separate epics, stories, tasks, bugs, or chores. Just multi-word names, optional context, tags, and fields when your team needs them. The workflow state is only `todo` → `wip` → `done`; readiness is derived from blockers, hierarchy, and state when work is explicitly waiting.
- **Collaborative**: yaks sync through git with conflict-free event merging. Multiple people and coding agents can update the same discovery tree from different branches, clones, and worktrees without coordinating edits.
- **Delightful**: the CLI should feel instant (<100ms for everyday operations), forgiving, and pleasant: fuzzy matching, tab completion, multi-word names, JSON output, and a pretty tree.

It's designed for humans and AI coding agents working together in the same codebase, without adding a server, database, or heavyweight process.

![demo](demo.gif)

## Install

### macOS/Linux

```bash
curl -fsSL https://raw.githubusercontent.com/mattwynne/yaks/main/install.sh | bash
```

The installer downloads the latest stable GitHub Release, validates the checksum when available, installs the `yx` binary in `/usr/local/bin`, and installs shell completions so you can tab-complete yak names.

To install a specific release:

```bash
curl -fsSL https://raw.githubusercontent.com/mattwynne/yaks/main/install.sh | YX_VERSION=1.2.3 bash
```

To install the edge channel:

```bash
curl -fsSL https://raw.githubusercontent.com/mattwynne/yaks/main/install.sh | YX_CHANNEL=edge bash
```

### Quick start

Tell your coding agent that this repo uses yaks by adding a note to a file it already reads. For example:

```bash
echo 'This project uses yaks (yx) for task management. See !yx help' > AGENTS.md
```

Multiple agents can update the shared discovery tree at the same time. Yaks uses an event-sourced CRDT-style merge on a hidden git ref, so simultaneous updates sync without normal file merge conflicts.

### Readiness and blockers

Yaks keeps workflow state small: `todo`, `wip`, or `done`. Whether a yak is ready to start is derived from that state, its children, and blockers.

```bash
yx list --ready
yx blocker add "publish announcement" --reason "waiting for credentials"
yx blocker add "deploy release" --by "security review" --reason "waiting on approval"
```

`yx list --only not-done` includes every yak that is not `done`, whether ready or blocked. Use `yx list --ready` when you want only actionable `todo` yaks whose children are complete and whose blockers are resolved.

## Why "Yaks"?

The name comes from [yak shaving](https://en.wiktionary.org/wiki/yak_shaving) — when you set out to do task A but discover you need B first, which requires C. Yaks captures this emergent structure as a discovery tree.

It's the same idea as a [Mikado Graph](https://mikadomethod.info) or a [Discovery Tree](https://www.fastagile.io/method/product-mapping-and-discovery-trees). The yak-shaving nickname and Yaks branding are a reminder that this is what we do all day in software.

![A Yaks discovery tree showing nested prerequisites](https://github.com/user-attachments/assets/1e935831-7807-4127-a698-3fdb50615080)

## Related tools and trade-offs

Yaks is not trying to be the most complete issue tracker. It is trying to be a sharp, dependency-first planning tool that stays light enough for a team to use continuously while coding.

### Beads

[Beads](https://github.com/steveyegge/beads) is a comprehensive issue tracker and agent orchestration system. It offers a rich task schema, many dependency types, database-backed sync, workflow features, and integrations for managing agents.

Yaks comes from XP, mob programming, and collaborative planning: make the emerging dependency tree visible, keep the model small, and trust the team to self-organise. Beads is a capable orchestration platform when you need richer workflow management; Yaks stays deliberately smaller so it can keep everyone oriented during the work.

### git-bug

[git-bug](https://github.com/git-bug/git-bug) is a distributed, offline-first issue tracker that stores issues, comments, identities, and history as git objects in the repository. It can sync through git remotes, offers CLI/TUI/web interfaces, and can bridge to systems like GitHub and GitLab.

Yaks also uses git for sharing, but it is not a GitHub-Issues-style tracker. It optimises for dependency-first discovery trees, lightweight CLI workflow, agent-friendly tree/context output, and CRDT-style collaboration through git event sync. git-bug is the better fit when you want full issue tracking inside git; Yaks is for when the tree of discovered prerequisites is the central artefact.

### kata

[kata](https://github.com/wesm/kata) is a local-first issue ledger for humans and coding agents. It emphasises stable agent commands, JSON output, predictable failure modes, a TUI for human oversight, and auditability through events. Its current architecture uses a local daemon and SQLite store, with future shared-server collaboration planned.

Yaks shares kata's interest in human/agent collaboration and a small complexity budget, but makes different choices: project-local discovery trees, git-backed sharing today, no daemon, and a tree-shaped model where dependency structure is primary. kata gives you a durable issue ledger with comments, labels, ownership, and a TUI; Yaks focuses on the fastest possible discovery tree of what is blocking what.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup, testing, and mutation testing instructions.

## License

[MIT](LICENSE)
