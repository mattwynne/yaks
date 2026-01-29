## Output port - defines interface for user-facing output

import ../domain/types

type
  OutputFormat* = enum
    Markdown
    Plain

  OutputFilter* = enum
    All
    NotDone
    OnlyDone

  OutputPort* = ref object of RootObj

method displayYaks*(self: OutputPort, yaks: seq[Yak], format: OutputFormat,
                    filter: OutputFilter) {.base.} =
  raise newException(CatchableError, "Not implemented")

method displayHelp*(self: OutputPort) {.base.} =
  raise newException(CatchableError, "Not implemented")

method displayError*(self: OutputPort, message: string) {.base.} =
  raise newException(CatchableError, "Not implemented")

method displayMessage*(self: OutputPort, message: string) {.base.} =
  raise newException(CatchableError, "Not implemented")
