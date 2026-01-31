## Refactor: Separate CLI Wrapper from Library Functions

**The Idea:**
Structure the code so that:
- `bin/yx` is a thin CLI wrapper that handles argc integration
- Core logic lives in sourced library files (e.g., `lib/yaks.sh`)
- CLI wrapper delegates to library functions

**Why This Could Be Good:**

1. **Testability**: Library functions can be tested independently of argc
2. **Reusability**: Core logic can be used by other tools/scripts
3. **Clarity**: Separates CLI concerns from business logic
4. **Incremental Migration**: Can migrate commands one at a time

**Proposed Structure:**
```
bin/yx          # Thin wrapper with argc integration
lib/
  yaks-core.sh  # Core functions (validate_yak_name, find_yak, etc.)
  yaks-add.sh   # Add command logic
  yaks-list.sh  # List command logic
  yaks-done.sh  # Done command logic
  ...
```

**Pattern:**
```bash
#!/usr/bin/env bash
# bin/yx - CLI wrapper

# Source library functions
source "${BASH_SOURCE%/*}/../lib/yaks-core.sh"
source "${BASH_SOURCE%/*}/../lib/yaks-add.sh"

# @cmd Add a new yak
# @arg name The yak name
add() {
  # Delegate to library function
  yaks_add "${argc_name}"
}

# Argc eval at end
eval "$("$ARGC_BIN" --argc-eval "$0" "$@")"
```

**Potential Concerns:**

1. **Sourcing Overhead**: Each invocation sources multiple files
2. **Path Management**: Need reliable way to find lib files
3. **Complexity**: Is this over-engineering for a ~900 line script?
4. **Testing**: Do we actually need library functions separated for testing?

**Alternative: Single File with Clear Sections**
Maybe just keep everything in one file but with clear sections:
- Configuration
- Core utilities
- Command implementations  
- Argc integration

**Questions:**
- Is the current minimal version (73 lines) already simple enough?
- Would we gain real benefits from separation?
- Are we solving a problem that doesn't exist?

**Decision Needed:**
Should we refactor to library pattern, or keep the simple single-file approach?
