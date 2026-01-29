## Terminal output adapter - implements OutputPort for console display
##
## This adapter renders yaks to the terminal using:
## - ANSI color codes for styling
## - Hierarchical indentation
## - Checkbox-style markdown format

import ../domain/types
import ../ports/output
import std/[strutils, algorithm]

type
  TerminalOutput* = ref object of OutputPort
    ## Terminal-based implementation of OutputPort.
    ##
    ## Renders output using ANSI escape codes and UTF-8 characters.

proc newTerminalOutput*(): TerminalOutput =
  ## Creates a new TerminalOutput adapter
  TerminalOutput()

proc getDepth(name: string): int =
  name.count('/')

proc getDisplayName(name: string): string =
  let parts = name.split('/')
  if parts.len > 0:
    return parts[^1]
  else:
    return name

proc getParent(name: string): string =
  ## Get the parent path of a yak, or "" if top-level
  let parts = name.split('/')
  if parts.len <= 1:
    return ""
  return parts[0..^2].join("/")

proc displayYakMarkdown(yak: Yak) =
  let depth = yak.name.getDepth()
  let indent = "  ".repeat(depth)
  let displayName = yak.name.getDisplayName()

  if yak.state == Done:
    # Use ANSI code \e[90m for grey color
    stdout.write("\e[90m", indent, "- [x] ", displayName, "\e[0m\n")
  else:
    stdout.write(indent, "- [ ] ", displayName, "\n")

proc displayYakPlain(yak: Yak) =
  echo yak.name

proc sortSiblings(yaks: seq[Yak]): seq[Yak] =
  ## Sort sibling yaks: done first, then by mtime, then by name
  result = yaks
  result.sort do (a, b: Yak) -> int:
    # Priority: 0 for done, 1 for todo
    let aPrio = if a.state == Done: 0 else: 1
    let bPrio = if b.state == Done: 0 else: 1

    if aPrio != bPrio:
      return cmp(aPrio, bPrio)

    # Then by mtime
    if a.mtime != b.mtime:
      return cmp(a.mtime, b.mtime)

    # Then by name
    return cmp(a.name, b.name)

proc displayHierarchy(yaks: seq[Yak], parent: string,
                     filter: OutputFilter, format: OutputFormat) =
  ## Display yaks hierarchically, recursively
  # Get children of this parent
  var children: seq[Yak] = @[]
  for yak in yaks:
    if yak.name.getParent() == parent:
      # Apply filter
      case filter
      of NotDone:
        if yak.state != Done:
          children.add(yak)
      of OnlyDone:
        if yak.state == Done:
          children.add(yak)
      else:
        children.add(yak)

  # Sort siblings
  children = sortSiblings(children)

  # Display each child and its descendants
  for child in children:
    case format
    of Markdown:
      displayYakMarkdown(child)
    of Plain:
      displayYakPlain(child)

    # Recursively display children
    displayHierarchy(yaks, child.name, filter, format)

method displayYaks*(self: TerminalOutput, yaks: seq[Yak],
                   format: OutputFormat, filter: OutputFilter) =
  # Check if empty
  if yaks.len == 0:
    if format == Markdown:
      echo "You have no yaks. Are you done?"
    return

  # Check if any would be displayed after filtering
  var hasVisibleYaks = false
  for yak in yaks:
    case filter
    of NotDone:
      if yak.state != Done:
        hasVisibleYaks = true
        break
    of OnlyDone:
      if yak.state == Done:
        hasVisibleYaks = true
        break
    else:
      hasVisibleYaks = true
      break

  if not hasVisibleYaks:
    if format == Markdown:
      echo "You have no yaks. Are you done?"
    return

  # Display hierarchically starting from root
  displayHierarchy(yaks, "", filter, format)

method displayHelp*(self: TerminalOutput) =
  echo """Usage: yx <command> [arguments]

Commands:
  add <name>                      Add a new yak
  list, ls [--format FMT]         List all yaks
           [--only STATE]
                          --format: Output format
                                    markdown (or md): Checkbox format (default)
                                    plain (or raw): Simple list of names
                          --only: Show only yaks in a specific state
                                  not-done: Show only incomplete yaks
                                  done: Show only completed yaks
  context [--show] <name>         Edit context (uses $EDITOR) or set from stdin
                          --show: Display yak with context
                          --edit: Edit context (default)
  done <name>                     Mark a yak as done
  done --undo <name>              Unmark a yak as done
  rm <name>                       Remove a yak by name
  move <old> <new>                Rename a yak
  mv <old> <new>                  Alias for move
  prune                           Remove all done yaks
  sync                            Push and pull yaks to/from origin via git ref
  completions [cmd]               Output yak names for shell completion
  --help                          Show this help message"""

method displayError*(self: TerminalOutput, message: string) =
  stderr.writeLine(message)

method displayMessage*(self: TerminalOutput, message: string) =
  echo message
