## Domain services - core business logic

import types, validation
import ../ports/[storage, git, output]
import std/[strutils, algorithm, sequtils]

type
  YakService* = object
    storage*: YakStorage
    git*: GitRepository

proc newYakService*(storage: YakStorage, git: GitRepository): YakService =
  YakService(storage: storage, git: git)

proc findYak*(self: YakService, searchTerm: string): string =
  ## Find a yak by exact or fuzzy match
  # Try exact match first
  if self.storage.exists(searchTerm):
    return searchTerm

  # Try fuzzy match
  var matches: seq[string] = @[]
  for yak in self.storage.findAll():
    if searchTerm in yak.name:
      matches.add(yak.name)

  if matches.len == 0:
    raise newException(NotFoundError, "Error: yak '" & searchTerm & "' not found")
  elif matches.len == 1:
    return matches[0]
  else:
    raise newException(AmbiguousNameError,
      "Error: yak name '" & searchTerm & "' is ambiguous")

proc addYak*(self: var YakService, name: string) =
  ## Add a new yak
  validateYakName(name)

  # Create parent yaks if needed
  let parts = name.split('/')
  var currentPath = ""
  for i, part in parts:
    if i == 0:
      currentPath = part
    else:
      currentPath = currentPath & "/" & part

    if not self.storage.exists(currentPath):
      let yak = Yak(
        name: currentPath,
        state: Todo,
        context: "",
        mtime: 0
      )
      self.storage.save(yak)

  self.git.logCommand("add " & name)

proc listYaks*(self: YakService): seq[Yak] =
  ## List all yaks
  self.storage.findAll()

proc markDone*(self: var YakService, name: string, recursive: bool = false) =
  ## Mark a yak as done
  let resolvedName = self.findYak(name)

  if not recursive and self.storage.hasChildren(resolvedName):
    # Check if all children are done
    var hasIncompletChildren = false
    for child in self.storage.getChildren(resolvedName):
      if child.state != Done:
        hasIncompletChildren = true
        break

    if hasIncompletChildren:
      raise newException(HasChildrenError,
        "Error: cannot mark '" & resolvedName &
        "' as done - it has incomplete children")

  self.storage.updateState(resolvedName, Done)

  if recursive:
    for child in self.storage.getChildren(resolvedName):
      self.markDone(child.name, recursive = true)

  let cmd = if recursive: "done --recursive " else: "done "
  self.git.logCommand(cmd & resolvedName)

proc markUndone*(self: var YakService, name: string) =
  ## Unmark a yak as done
  let resolvedName = self.findYak(name)
  self.storage.updateState(resolvedName, Todo)
  self.git.logCommand("done --undo " & resolvedName)

proc removeYak*(self: var YakService, name: string) =
  ## Remove a yak
  let resolvedName = self.findYak(name)
  self.storage.remove(resolvedName)
  self.git.logCommand("rm " & resolvedName)

proc pruneYaks*(self: var YakService) =
  ## Remove all done yaks
  let allYaks = self.storage.findAll()
  for yak in allYaks:
    if yak.state == Done:
      self.storage.remove(yak.name)

proc moveYak*(self: var YakService, oldName: string, newName: string) =
  ## Move/rename a yak
  let resolvedOld = self.findYak(oldName)
  validateYakName(newName)

  # Ensure parent yaks exist
  let parts = newName.split('/')
  if parts.len > 1:
    var parentPath = parts[0 .. ^2].join("/")
    if not self.storage.exists(parentPath):
      self.addYak(parentPath)

  self.storage.rename(resolvedOld, newName)
  self.git.logCommand("move " & resolvedOld & " " & newName)

proc updateContext*(self: var YakService, name: string, context: string) =
  ## Update yak context
  let resolvedName = self.findYak(name)
  self.storage.updateContext(resolvedName, context)
  self.git.logCommand("context " & resolvedName)

proc getContext*(self: YakService, name: string): (string, string) =
  ## Get yak name and context
  let resolvedName = self.findYak(name)
  let yak = self.storage.findByName(resolvedName)
  return (resolvedName, yak.context)
