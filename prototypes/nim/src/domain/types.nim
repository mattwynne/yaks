## Core domain types for Yak management
##
## This module defines the fundamental domain types used throughout
## the yx application, following domain-driven design principles.

type
  YakState* = enum
    ## Represents the completion state of a yak
    Todo = "todo"  ## Yak is not yet completed
    Done = "done"  ## Yak has been completed

  Yak* = object
    ## A yak represents a task or work item in the system
    name*: string      ## Unique identifier/path for the yak (e.g., "parent/child")
    state*: YakState   ## Current completion state
    context*: string   ## Additional markdown context/notes
    mtime*: int64      ## Last modification time (Unix timestamp)

  YakError* = object of CatchableError
    ## Base exception type for all yak-related errors

  InvalidNameError* = object of YakError
    ## Raised when a yak name contains forbidden characters

  NotFoundError* = object of YakError
    ## Raised when a yak cannot be found

  AmbiguousNameError* = object of YakError
    ## Raised when a fuzzy match returns multiple results

  HasChildrenError* = object of YakError
    ## Raised when attempting to mark a parent as done with incomplete children
