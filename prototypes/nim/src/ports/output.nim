## Output port - defines interface for user-facing output
##
## This port defines how the application displays information to users,
## allowing different output implementations (terminal, GUI, etc.) without
## changing the core business logic.

import ../domain/types

type
  OutputFormat* = enum
    ## Display format for yak listings
    Markdown  ## Checkbox-style markdown format
    Plain     ## Simple list of names

  OutputFilter* = enum
    ## Filter for which yaks to display
    All        ## Show all yaks
    NotDone    ## Show only incomplete yaks
    OnlyDone   ## Show only completed yaks

  OutputPort* = ref object of RootObj
    ## Abstract interface for displaying information to users

method displayYaks*(self: OutputPort, yaks: seq[Yak], format: OutputFormat,
                    filter: OutputFilter) {.base.} =
  raise newException(CatchableError, "Not implemented")

method displayHelp*(self: OutputPort) {.base.} =
  raise newException(CatchableError, "Not implemented")

method displayError*(self: OutputPort, message: string) {.base.} =
  raise newException(CatchableError, "Not implemented")

method displayMessage*(self: OutputPort, message: string) {.base.} =
  raise newException(CatchableError, "Not implemented")
