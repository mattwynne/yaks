Once all write operations (add, done, rm, move, context) have been migrated to use the --tree mode, simplify log_command() to only support tree-based operation.

Remove the filesystem fallback code that reads from .yaks/, keeping only the --tree parameter logic.

This cleanup can only happen after all callers have been converted to build trees and pass them via --tree.
