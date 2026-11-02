# Prerequisite nesting eval

Run `dev eval-dependency-help` with an authenticated Codex CLI.
Set `YX_EVAL_PROVIDER=claude` to use Claude instead.

The runner builds yx, copies it into a fresh temporary Git repository, and
asks the agent to create a map using the CLI. The agent reads help itself.
The task specifies yak names and dependencies in prose, without supplying
the expected hierarchy.

After the agent finishes, the runner independently calls
`yx list --format json` and grades the persisted hierarchy and blockers.
The final agent reply is not graded.

The grader requires the prerequisite chain to be nested, the shared contract
to have one home with explicit blockers from the other consuming branches, and missing vendor
credentials to block both affected leaves. It rejects redundant blockers.

Each run prints its artifact directory. It retains the scratch repository,
prompt, agent trace, stderr, and map.json for inspection. These are temporary
artifacts, not changes to this project's yak map.

The fast ShellSpec tests use real yx commands with a fake model process. They
verify that a correct persisted map passes and an empty map fails even when
the model claims success. They do not measure model performance.

A live run measures one model attempt. Repeated runs and comparison against
older help are needed to measure whether a wording change improves success.
