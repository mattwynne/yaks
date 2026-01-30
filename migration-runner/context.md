Create a migration runner script that:
- Reads migration files from a migrations/ directory
- Tracks which migrations have been applied (e.g., in a .migrations_state file)
- Applies migrations in order
- Handles errors gracefully

Should support:
- `migrate up` - apply all pending migrations
- `migrate status` - show which migrations are applied/pending
