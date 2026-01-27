Add integration tests that verify the sync feature works correctly with remote git repositories.

Currently, sync tests only use local operations. We need tests that:
- Verify push/fetch work correctly with actual remote repositories
- Test the refs/notes/yaks reference is properly synced
- Ensure the sync feature works across different machines/clones

This will catch issues that only appear when working with real git remotes.
