We want to prevent commits getting into the main branch that will fail CI.

We should run some checks like `shellspec` and `dev lint` automatically in git hooks to mitigate this risk.
