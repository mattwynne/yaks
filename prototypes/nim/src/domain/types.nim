## Core domain types for Yak management

type
  YakState* = enum
    Todo = "todo"
    Done = "done"

  Yak* = object
    name*: string
    state*: YakState
    context*: string
    mtime*: int64

  YakError* = object of CatchableError

  InvalidNameError* = object of YakError
  NotFoundError* = object of YakError
  AmbiguousNameError* = object of YakError
  HasChildrenError* = object of YakError
