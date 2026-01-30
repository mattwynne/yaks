Add CI check that:
- Runs migrations in test environment before running tests
- Fails the build if migrations fail
- Ensures migrations are always tested

Depends on: migration-runner script being complete

This validates that migrations work in a clean environment.
