## Git port - defines interface for git operations
##
## This port defines the abstract interface for git-related operations,
## allowing the domain logic to interact with git without depending on
## a specific implementation.

type
  GitRepository* = ref object of RootObj
    ## Abstract interface for git repository operations.
    ##
    ## Implementations handle git command logging and synchronization.

method isRepository*(self: GitRepository): bool {.base.} =
  raise newException(CatchableError, "Not implemented")

method hasOrigin*(self: GitRepository): bool {.base.} =
  raise newException(CatchableError, "Not implemented")

method logCommand*(self: GitRepository, message: string) {.base.} =
  raise newException(CatchableError, "Not implemented")

method sync*(self: GitRepository) {.base.} =
  raise newException(CatchableError, "Not implemented")

method checkIgnored*(self: GitRepository, path: string): bool {.base.} =
  raise newException(CatchableError, "Not implemented")
