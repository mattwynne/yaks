# Manual prose-to-yak-map eval

Run from the development shell with an authenticated Codex CLI:

```sh
dev eval
```

This builds the **current working copy**, including uncommitted help edits,
then runs eight independent attempts with at most four running at once.
Every attempt starts with an empty temporary Git repository. The agent reads
the candidate CLI help, creates its own tasks through yx, and leaves them todo.
The frozen [brief](task.md) is ordinary prose across three codebases; it
supplies neither task names nor a starting map.

Useful variations:

```sh
dev eval --runs 2 --jobs 2
dev eval --compare d5c4bc4
dev eval --compare d5c4bc4 --runs 8 --jobs 4 --model gpt-6.1-sol
dev eval --output /tmp/catalog-eval --timeout 300
```

`--compare` adds the old revision as a baseline and interleaves attempts.
The default model is `gpt-6.1-sol` for both versions. The default timeout is
300 seconds per attempt. Initial compilation can take a few minutes; builds
are cached and performed once per version, not per attempt.

Each run prints its artifact directory. By default, results stay in the
gitignored `eval-results/` directory. An explicit output directory must be
empty. It contains a readable `report.md`, `results.json`, exact revisions,
binary/help snapshots, brief/grader hashes, build logs, and each attempt's
prompt, command trace, saved Git repository, map, and `assessment.json`.

## What the grader checks

The runner independently reads the actual map with `yx list --format json`.
The [grader](grade.py) recognizes task roles from common English words such
as reader, refresh, validation, migration, integration, and deployment.
It checks that:

- The main prerequisite chain is expressed by ancestry in the tree.
- The shared contract has one home, with explicit blockers from consumers
  whose dependency is not already expressed by nesting.
- Reader and refresh each have a manual credential blocker.
- No extra explicit blockers duplicate the prerequisite chain.

Task names need not match exact strings. Unrecognized or ambiguous tasks are
marked **needs review** instead of being called incorrect. This is a
case-specific word-based grader, not a general semantic judge: inspect
`assessment.json` and `map.json` when it needs review or when changing
the brief. Broader language or task decomposition may need human assessment.

Exit status is 0 when all current-copy attempts pass, 1 for current-copy
incorrect maps, and 2 for execution errors or maps needing review.
Baseline failures do not make an otherwise passing current copy fail.
All attempts stay in the reported denominator; failed attempts are not
silently retried. Small samples describe this case, not general model quality.

## Fast verification

`dev check` runs the harness and grader tests without invoking a model.
To run only these tests:

```sh
python3 -m unittest discover -s tests -p 'test_*.py'
shellspec tests/shellspec/dev_dependency_help_eval.sh
```

The tests include real archived agent maps that are nested, flat, and mixed,
plus ambiguous-name and external-blocker cases. Runner tests use a fake agent
process but real yx commands to verify that the persisted map is inspected.

## Recorded experiments

The [free-prose comparison](results/2026-11-02-free-prose.md) is the case
packaged by `dev eval`: old help yielded five nested, two mixed, and one flat
map; new help yielded eight nested maps. These were manually assessed before
the current grader was introduced.

The [earlier comparison](results/2026-11-02.md) used a more explicit brief
and prescribed task names. Both versions passed 10/10; it is historical
evidence, not a run of the current brief.

The lower-level `bin/compare-dependency-help` command remains available
for comparing two committed revisions with `--old-ref` and `--new-ref`.
`dev eval` always evaluates the current working copy as its candidate.
