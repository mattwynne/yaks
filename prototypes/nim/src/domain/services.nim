## Domain services - core business logic
##
## Orchestrates yak operations by coordinating between storage and git ports.
## Contains all business logic for yak management.

import types, validation
import ../ports/[storage, git]
import std/[strutils, strformat]

type
  YakService* = object
    ## Service that orchestrates yak operations
    storage*: YakStorage      ## Storage port for persistence
    git*: GitRepository       ## Git port for command logging

proc newYakService*(storage: YakStorage, git: GitRepository): YakService =
  ## Creates a new YakService with the given dependencies
  YakService(storage: storage, git: git)

proc findYak*(self: YakService, searchTerm: string): string =
  ## Finds a yak by exact or fuzzy match.
  ##
  ## First attempts an exact name match. If that fails, searches for
  ## yaks containing the search term as a substring.
  ##
  ## Returns:
  ##   The exact yak name
  ##
  ## Raises:
  ##   NotFoundError: If no matches found
  ##   AmbiguousNameError: If multiple matches found
  # Try exact match first
  if self.storage.exists(searchTerm):
    return searchTerm

  # Try fuzzy match
  var matches: seq[string] = @[]
  for yak in self.storage.findAll():
    if searchTerm in yak.name:
      matches.add(yak.name)

  if matches.len == 0:
    raise newException(NotFoundError, fmt"Error: yak '{searchTerm}' not found")
  elif matches.len == 1:
    return matches[0]
  else:
    raise newException(AmbiguousNameError,
      fmt"Error: yak name '{searchTerm}' is ambiguous")

proc addYak*(self: var YakService, name: string) =
  ## Adds a new yak to the system.
  ##
  ## Automatically creates parent yaks if the name contains slashes.
  ## For example, adding "parent/child" will create both "parent" and "parent/child"
  ## if "parent" doesn't already exist.
  ##
  ## Raises:
  ##   InvalidNameError: If the name contains forbidden characters
  validateYakName(name)

  # Create parent yaks if needed
  let parts = name.split('/')
  var currentPath = ""
  for i, part in parts:
    currentPath = if i == 0: part else: fmt"{currentPath}/{part}"

    if not self.storage.exists(currentPath):
      let yak = Yak(
        name: currentPath,
        state: Todo,
        context: "",
        mtime: 0
      )
      self.storage.save(yak)

  self.git.logCommand(fmt"add {name}")

proc listYaks*(self: YakService): seq[Yak] =
  ## Lists all yaks in the system.
  ##
  ## Returns all yaks regardless of state or hierarchy.
  self.storage.findAll()

proc markDone*(self: var YakService, name: string, recursive: bool = false) =
  ## Marks a yak as done.
  ##
  ## By default, prevents marking a parent as done if it has incomplete children.
  ## Use recursive=true to mark the yak and all its children as done.
  ##
  ## Args:
  ##   name: Name or fuzzy match term for the yak
  ##   recursive: If true, also marks all children as done
  ##
  ## Raises:
  ##   NotFoundError: If yak not found
  ##   AmbiguousNameError: If fuzzy match is ambiguous
  ##   HasChildrenError: If parent has incomplete children and recursive=false
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
        fmt"Error: cannot mark '{resolvedName}' as done - it has incomplete children")

  self.storage.updateState(resolvedName, Done)

  if recursive:
    for child in self.storage.getChildren(resolvedName):
      self.markDone(child.name, recursive = true)

  if recursive:
    self.git.logCommand(fmt"done --recursive {resolvedName}")
  else:
    self.git.logCommand(fmt"done {resolvedName}")

proc markUndone*(self: var YakService, name: string) =
  ## Marks a done yak as todo (unmarks it as done).
  ##
  ## Raises:
  ##   NotFoundError: If yak not found
  ##   AmbiguousNameError: If fuzzy match is ambiguous
  let resolvedName = self.findYak(name)
  self.storage.updateState(resolvedName, Todo)
  self.git.logCommand(fmt"done --undo {resolvedName}")

proc removeYak*(self: var YakService, name: string) =
  ## Removes a yak from the system.
  ##
  ## Raises:
  ##   NotFoundError: If yak not found
  ##   AmbiguousNameError: If fuzzy match is ambiguous
  let resolvedName = self.findYak(name)
  self.storage.remove(resolvedName)
  self.git.logCommand(fmt"rm {resolvedName}")

proc pruneYaks*(self: var YakService) =
  ## Removes all yaks that are marked as done.
  ##
  ## Useful for cleaning up completed work.
  let allYaks = self.storage.findAll()
  for yak in allYaks:
    if yak.state == Done:
      self.storage.remove(yak.name)

proc moveYak*(self: var YakService, oldName: string, newName: string) =
  ## Moves or renames a yak.
  ##
  ## Can be used to rename a yak or move it to a different parent.
  ## Automatically creates parent yaks if the new name requires them.
  ##
  ## Raises:
  ##   NotFoundError: If old yak not found
  ##   AmbiguousNameError: If fuzzy match is ambiguous
  ##   InvalidNameError: If new name contains forbidden characters
  let resolvedOld = self.findYak(oldName)
  validateYakName(newName)

  # Ensure parent yaks exist
  let parts = newName.split('/')
  if parts.len > 1:
    var parentPath = parts[0 .. ^2].join("/")
    if not self.storage.exists(parentPath):
      self.addYak(parentPath)

  self.storage.rename(resolvedOld, newName)
  self.git.logCommand(fmt"move {resolvedOld} {newName}")

proc updateContext*(self: var YakService, name: string, context: string) =
  ## Updates the markdown context for a yak.
  ##
  ## Raises:
  ##   NotFoundError: If yak not found
  ##   AmbiguousNameError: If fuzzy match is ambiguous
  let resolvedName = self.findYak(name)
  self.storage.updateContext(resolvedName, context)
  self.git.logCommand(fmt"context {resolvedName}")

proc getContext*(self: YakService, name: string): (string, string) =
  ## Gets the yak's resolved name and context.
  ##
  ## Returns:
  ##   A tuple of (resolved_name, context)
  ##
  ## Raises:
  ##   NotFoundError: If yak not found
  ##   AmbiguousNameError: If fuzzy match is ambiguous
  let resolvedName = self.findYak(name)
  let yak = self.storage.findByName(resolvedName)
  return (resolvedName, yak.context)
