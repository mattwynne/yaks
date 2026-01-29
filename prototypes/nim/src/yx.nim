## Main CLI entry point - wires adapters to domain logic

import std/[os, strutils, rdstdin, terminal]
import domain/[types, services]
import ports/[storage, git, output]
import adapters/[filesystem_storage, git_repository, terminal_output]

proc getWorkTree(): string =
  getEnv("GIT_WORK_TREE", getCurrentDir())

proc getYaksPath(workTree: string): string =
  workTree / ".yaks"

proc main() =
  let workTree = getWorkTree()
  let yaksPath = getYaksPath(workTree)

  # Wire up adapters
  let storage = newFilesystemStorage(yaksPath)
  let git = newShellGitRepository(workTree)
  let output = newTerminalOutput()

  var service = newYakService(storage, git)

  # Get command line args
  let args = commandLineParams()

  if args.len == 0 or args[0] == "--help":
    output.displayHelp()
    quit(0)

  # Check git requirements (except for help)
  if not git.isRepository():
    output.displayError("Error: not in a git repository")
    output.displayError("yx must be run from within a git repository")
    quit(1)

  if not git.checkIgnored(".yaks"):
    output.displayError("Error: .yaks folder is not gitignored")
    output.displayError("Please add .yaks to your .gitignore file")
    quit(1)

  let command = args[0]

  try:
    case command
    of "add":
      if args.len == 1:
        # Interactive mode
        echo "Enter yaks (empty line to finish):"
        while true:
          let line = readLineFromStdin("").strip()
          if line == "":
            break
          service.addYak(line)
      else:
        # Single yak from args
        let name = args[1..^1].join(" ")
        service.addYak(name)

    of "list", "ls":
      var format = Markdown
      var filter = All
      var i = 1

      while i < args.len:
        if args[i] == "--format" and i + 1 < args.len:
          case args[i + 1]
          of "plain", "raw":
            format = Plain
          of "markdown", "md":
            format = Markdown
          else:
            format = Markdown
          i += 2
        elif args[i] == "--only" and i + 1 < args.len:
          case args[i + 1]
          of "not-done":
            filter = NotDone
          of "done":
            filter = OnlyDone
          else:
            filter = All
          i += 2
        else:
          i += 1

      let yaks = service.listYaks()
      output.displayYaks(yaks, format, filter)

    of "done":
      if args.len < 2:
        output.displayError("Error: yak name required")
        quit(1)

      if args[1] == "--undo":
        if args.len < 3:
          output.displayError("Error: yak name required")
          quit(1)
        let name = args[2..^1].join(" ")
        service.markUndone(name)
      elif args[1] == "--recursive":
        if args.len < 3:
          output.displayError("Error: yak name required")
          quit(1)
        let name = args[2..^1].join(" ")
        service.markDone(name, recursive = true)
      else:
        let name = args[1..^1].join(" ")
        service.markDone(name)

    of "rm":
      if args.len < 2:
        output.displayError("Error: yak name required")
        quit(1)
      let name = args[1..^1].join(" ")
      service.removeYak(name)

    of "prune":
      service.pruneYaks()

    of "move", "mv":
      if args.len < 3:
        output.displayError("Error: old and new names required")
        quit(1)
      let oldName = args[1]
      let newName = args[2..^1].join(" ")
      service.moveYak(oldName, newName)

    of "context":
      var showMode = false
      var startIdx = 1

      if args.len > 1 and args[1] == "--show":
        showMode = true
        startIdx = 2
      elif args.len > 1 and args[1] == "--edit":
        startIdx = 2

      if args.len < startIdx + 1:
        output.displayError("Error: yak name required")
        quit(1)

      let name = args[startIdx..^1].join(" ")

      if showMode:
        let (resolvedName, context) = service.getContext(name)
        echo resolvedName
        if context != "":
          echo ""
          echo context
      else:
        # Check if stdin is available
        if stdin.isatty():
          # Use editor
          let editor = getEnv("EDITOR", "vi")
          let (resolvedName, currentContext) = service.getContext(name)
          let tempFile = getTempDir() / "yak_context_" & $getCurrentProcessId() & ".md"
          writeFile(tempFile, currentContext)

          let exitCode = execShellCmd(editor & " " & quoteShell(tempFile))
          if exitCode == 0:
            let newContext = readFile(tempFile)
            service.updateContext(name, newContext)

          removeFile(tempFile)
        else:
          # Read from stdin
          var lines: seq[string] = @[]
          while true:
            try:
              let line = readLine(stdin)
              lines.add(line)
            except EOFError:
              break

          let context = lines.join("\n")
          service.updateContext(name, context)

    of "sync":
      if not git.hasOrigin():
        output.displayError("Error: no origin remote configured")
        quit(1)
      git.sync()

    of "completions":
      # Completion support with filtering
      var cmd = ""
      var flag = ""
      if args.len > 1:
        cmd = args[1]
      if args.len > 2:
        flag = args[2]

      let yaks = service.listYaks()
      for yak in yaks:
        case cmd
        of "done":
          if flag == "--undo":
            # Show only done yaks for undo
            if yak.state == Done:
              echo yak.name
          else:
            # Show only todo yaks for done
            if yak.state != Done:
              echo yak.name
        else:
          # Show all yaks
          echo yak.name

    else:
      output.displayHelp()
      quit(0)

  except InvalidNameError as e:
    output.displayError(e.msg)
    quit(1)
  except NotFoundError as e:
    output.displayError(e.msg)
    quit(1)
  except AmbiguousNameError as e:
    output.displayError(e.msg)
    quit(1)
  except HasChildrenError as e:
    output.displayError(e.msg)
    quit(1)
  except CatchableError as e:
    output.displayError("Error: " & e.msg)
    quit(1)

when isMainModule:
  main()
