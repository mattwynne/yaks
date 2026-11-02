---
name: work
description: Implement a prepared yak for contributors maintaining Yaks itself using isolated worktrees, one-test-at-a-time TDD, verification, and dev merge.
---

# Work on a contributor yak

This workflow is for maintaining this repository. Read `AGENTS.md` and
`CONTRIBUTING.md` for development commands and project policy. Use `yx --help`
for general Yaks usage; no extra agent plugin is required.

## Select and claim

Read the selected yak with `yx show "yak name" --format json`, including its
context, children, readiness and blockers. Read any agreed examples or ADR
referenced by the context. Use the hierarchy as the plan, not a `plan` field.
If no yak is selected, inspect `yx list --ready`.

If requirements are unclear, use [prepare](../prepare/SKILL.md) to resolve
them with the human before coding. Record concrete unanswered questions on
the yak rather than guessing a materially different outcome.

Claim the yak with `yx start "yak name"`. Reuse the current isolated
worktree when it already belongs to this work. Otherwise, from the main
checkout run `dev start "yak name"`, which marks the yak wip, creates
`.worktrees/<yak-id>`, and records the branch/worktree on the yak.
`yx start` alone does not create a worktree.

Develop in that worktree; leave other people's changes alone. Load the
documented development environment there so `dev`, `yx`, and test tools
are available. Use `devenv shell -- bin/dev check` if starting without direnv.

Delegation is optional. If useful and authorized, use the current agent's
available delegation facility and ensure it works in the intended worktree.
Pass the yak context, examples, scope and project instructions. Review the
returned changes and results; do not assume a particular tool schema or
named worker exists. Keep work that touches the same files sequential.

## Implement one example at a time

Follow the repository's TDD cycle: introduce one failing test, run it to
confirm the expected failure, make the smallest implementation pass, and
refactor while keeping tests green. Repeat for the next example.

- For observable behavior, use agreed examples and
  [BDD formulation](../bdd-formulation/SKILL.md) in Cucumber acceptance tests.
- Use Rust unit tests for internal logic and ShellSpec for cases described
  in `AGENTS.md`. Refactorings must preserve behavior.
- Consult `src/README.md` before changing Rust layers and
  `docs/cli-design-philosophy.md` before changing CLI behavior.
- For architectural decisions, read
  [CQRS and event sourcing](../cqrs-event-sourcing/SKILL.md) and follow the
  policy for new ADRs; accepted ADRs stay unchanged.
- Use [yx sandbox](../yx-sandbox/SKILL.md) for experiments and test data.
  Never edit the real `.yaks` directory directly.

Use `dev mutate-diff` while coding and
[mutation triage](../triaging-mutants/SKILL.md) when needed. Trust passing
tests; do not repeat manual verification already covered by them.

## Verify, merge and complete

Add user-visible changes under the top-level `Unreleased` section of
`CHANGELOG.md`. Run the focused tests and `dev check` before committing.
Report failures and resolve them within the task's scope; never bypass
verification to get a merge through.

Commit the verified change, then run `dev merge "branch-name"` from the
clean main checkout. This rebases onto main, verifies the result, and
fast-forwards only if checks pass. Never replace it with a direct Git merge.
If it fails, inspect the error and fix the branch in its worktree.

After merging, ensure the yak is done using `yx done "yak name"` and
confirm branch/worktree cleanup. `dev merge` normally handles cleanup and
marks a matching yak done; verify before repeating those operations.
Do not remove the checkout from which an active agent still needs to work.
Report what changed, validation results and any outstanding questions.
