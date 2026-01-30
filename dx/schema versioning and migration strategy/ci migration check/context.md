Add CI check that runs migrations in test environment.

DEPENDS ON: migration runner script must exist first.

This check should:
- Run in CI pipeline
- Apply migrations to a test database
- Verify migrations succeed
- Fail the build if migrations break

---

Previous context from duplicate "ci-migration-check" yak:
Add CI check that:
- Runs migrations in test environment before running tests
- Fails the build if migrations fail
- Ensures migrations are always tested

This validates that migrations work in a clean environment.
