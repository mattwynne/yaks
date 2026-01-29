## Storage port - defines interface for yak persistence
##
## This port defines the abstract interface for yak storage operations,
## following the hexagonal architecture pattern. Concrete implementations
## (adapters) can store yaks in different backends while the domain logic
## remains unchanged.

import ../domain/types

type
  YakStorage* = ref object of RootObj
    ## Abstract storage interface for yak persistence.
    ##
    ## Implementations must override all methods with concrete behavior.

method findAll*(self: YakStorage): seq[Yak] {.base.} =
  ## Find all yaks in storage
  raise newException(CatchableError, "Not implemented")

method findByName*(self: YakStorage, name: string): Yak {.base.} =
  ## Find a specific yak by exact name
  raise newException(CatchableError, "Not implemented")

method exists*(self: YakStorage, name: string): bool {.base.} =
  ## Check if a yak exists
  raise newException(CatchableError, "Not implemented")

method save*(self: YakStorage, yak: Yak) {.base.} =
  ## Save or update a yak
  raise newException(CatchableError, "Not implemented")

method remove*(self: YakStorage, name: string) {.base.} =
  ## Remove a yak from storage
  raise newException(CatchableError, "Not implemented")

method updateState*(self: YakStorage, name: string, state: YakState) {.base.} =
  ## Update the state of a yak
  raise newException(CatchableError, "Not implemented")

method updateContext*(self: YakStorage, name: string, context: string) {.base.} =
  ## Update the context of a yak
  raise newException(CatchableError, "Not implemented")

method rename*(self: YakStorage, oldName: string, newName: string) {.base.} =
  ## Rename a yak
  raise newException(CatchableError, "Not implemented")

method hasChildren*(self: YakStorage, name: string): bool {.base.} =
  ## Check if a yak has child yaks
  raise newException(CatchableError, "Not implemented")

method getChildren*(self: YakStorage, name: string): seq[Yak] {.base.} =
  ## Get all child yaks of a parent
  raise newException(CatchableError, "Not implemented")
