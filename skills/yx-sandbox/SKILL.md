---
name: yx-sandbox
description: Use when running yx commands that create, modify, or delete yaks outside of real project work — provides an isolated temp environment
---

# yx Sandbox

Use a sandbox whenever you need to run `yx` commands that would
pollute the project's real yak list — exploratory testing, UX
reviews, demos, experiments, etc.

## Setup

Create a temp directory and capture its literal path:

```bash
mktemp -d
# Output: /tmp/tmp.xYz123AbC  ← capture this literal path
```

## Usage

Prefix **every** `yx` command with the env vars using the literal
path from above:

```bash
YAK_PATH=/tmp/tmp.xYz123AbC YX_SKIP_GIT_CHECKS=1 yx <command>
```

- `YAK_PATH` redirects storage to the temp directory
- `YX_SKIP_GIT_CHECKS=1` avoids git setup requirements in temp dirs
- **Never run bare `yx`** without these env vars during sandbox work

**Note:** Shell variables don't persist between Bash tool calls.
Always use the literal path, not a variable like `$SANDBOX`.

## Cleanup

```bash
rm -rf /tmp/tmp.xYz123AbC
```

## Common Mistakes

| Mistake | Fix |
|---------|-----|
| Running `yx` without the prefix | Every command needs both env vars |
| Using `$SANDBOX` variable | Literal path only — vars don't persist between calls |
| Forgetting to clean up | `rm -rf <path>` when done |
