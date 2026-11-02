# Free-prose comparison, 2026-11-02

The [brief](../task.md) describes a catalog-service release across three
codebases. Each agent started from an empty repository, chose its own task
names, read CLI help, and created the map through yx.

Model: gpt-6.1-sol. Eight attempts per help version, at most four concurrent.
These maps were manually assessed before the packaged structural grader was
introduced. The fixed-name grader used in earlier experiments was not used
to score these freely named tasks.

| Help | Main prerequisite chain nested | Mixed | Flat with explicit blockers |
| --- | ---: | ---: | ---: |
| Old | 5 | 2 | 1 |
| New | 8 | 0 | 0 |

Old revision: d5c4bc440c976e42ca0d177bda7f7b969b23ddd0.
New revision: b5e295710802bc8a7bb25493053e6ae78efadbcc.
Brief SHA-256: 6700ef2a89d175c26a255edf5d657676282025c9eab138a20cf6ebf5f28cf1a5.

The old flat result put seven tasks directly under the deployment goal and
used explicit blockers for the main chain. The mixed results left SDK
validation as a separate root or beside integration, linked to importer
adaptation by a blocker. Packaging replay caught attempt 5, which the
initial manual tally had mistakenly counted as fully nested.
Every new-help map nested the main chain, gave the shared contract one home,
and blocked reader and refresh individually for missing credentials.

Representative archived maps are retained as compact test fixtures:
[old nested](../../../tests/fixtures/eval-old-nested.json),
[old flat](../../../tests/fixtures/eval-old-flat.json),
[old mixed](../../../tests/fixtures/eval-old-mixed.json), and
[new nested](../../../tests/fixtures/eval-new-nested.json).

This brief was selected after probing old-help behavior. Three of the four
repeat old-help attempts fully nested the chain; one was mixed. The counts are encouraging but too
small and too selected to establish a general improvement rate.

Earlier experiments starting with seeded maps do not count as evidence for
the prose-from-empty requirement. The packaged eval never seeds yaks.
