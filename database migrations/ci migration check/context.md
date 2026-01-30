Add CI check that runs migrations in test environment.

DEPENDS ON: migration runner script must exist first.

This check should:
- Run in CI pipeline
- Apply migrations to a test database
- Verify migrations succeed
- Fail the build if migrations break
