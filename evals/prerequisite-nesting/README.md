# Prerequisite nesting eval

Run `dev eval-dependency-help` with an authenticated Codex CLI.
Set `YX_EVAL_PROVIDER=claude` to use Claude instead.

For a repeated old-versus-new comparison, run:

```sh
bin/compare-dependency-help --runs 10 --jobs 4
```

This builds the pre-change revision `d5c4bc4` and current `HEAD` once each,
then interleaves ten attempts per version with four attempts running in total.
Both versions use the same task, grader, model, and timeout. Use `--old-ref`,
`--new-ref`, and `--model` to select these explicitly.

The output directory contains `report.md`, `results.json`, exact revision
metadata, both binaries, build logs, and every attempt's trace and saved map.
Incorrect maps and model/process errors are reported separately. The pass
rate uses all attempts, including errors; failed attempts are not retried.
Compilation time is reported separately from evaluation wall time.

The comparison requires Python 3, Cargo, jq, Git, tar, and an authenticated
Codex CLI. Tests for the comparison accounting can be run with
`python3 -m unittest discover -s tests -p test_dependency_help_comparison.py`.

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

## Recorded comparison

The [2026-11-02 comparison](results/2026-11-02.md) passed 10/10 attempts
with each version of the help. This task did not distinguish the two versions
for the selected model. The [machine-readable results](results/2026-11-02.json)
include timings, binary hashes, exact revisions, and paths to individual runs.
