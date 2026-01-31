Create a migration runner script that:
- Reads migration files from a migrations/ directory
- Tracks which migrations have been applied (e.g., in a .migrations_state file)
- Applies migrations in order
- Handles errors gracefully

Should support:
- `migrate up` - apply all pending migrations
- `migrate status` - show which migrations are applied/pending

---

Previous context from duplicate "migration-runner" yak:
Create a migration runner script that:
- Reads migration files from a directory
- Tracks which migrations have been applied
- Applies pending migrations in order
- Handles rollback on failure

This must be completed BEFORE working on the CI check.
