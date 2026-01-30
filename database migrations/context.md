Add database migration support to the project.

This is a two-phase effort:
1. Create the migration runner script first
2. Add CI checks that depend on the runner

Work on child yaks in order - the CI check requires the 
migration runner to exist.
