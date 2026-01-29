## Git port - defines interface for git operations

type
  GitRepository* = ref object of RootObj
    ## Abstract git interface

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
