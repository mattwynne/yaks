## Filesystem storage adapter - implements YakStorage using directory structure

import ../domain/types
import ../ports/storage
import std/[os, times, strutils, sequtils, algorithm]

type
  FilesystemStorage* = ref object of YakStorage
    basePath*: string

proc newFilesystemStorage*(basePath: string): FilesystemStorage =
  result = FilesystemStorage(basePath: basePath)

proc getYakPath(self: FilesystemStorage, name: string): string =
  self.basePath / name

proc getStatePath(self: FilesystemStorage, name: string): string =
  self.getYakPath(name) / "state"

proc getContextPath(self: FilesystemStorage, name: string): string =
  self.getYakPath(name) / "context.md"

proc readState(path: string): YakState =
  if fileExists(path):
    let content = readFile(path).strip()
    case content
    of "done": return Done
    else: return Todo
  else:
    return Todo

proc getMTime(path: string): int64 =
  try:
    let info = getFileInfo(path)
    return info.lastWriteTime.toUnix()
  except:
    return 0

method findAll*(self: FilesystemStorage): seq[Yak] =
  result = @[]
  if not dirExists(self.basePath):
    return

  for yakPath in walkDirRec(self.basePath, yieldFilter = {pcDir}):
    let relativePath = yakPath.relativePath(self.basePath)
    let statePath = self.getStatePath(relativePath)
    let contextPath = self.getContextPath(relativePath)

    var context = ""
    if fileExists(contextPath):
      context = readFile(contextPath)

    result.add(Yak(
      name: relativePath,
      state: readState(statePath),
      context: context,
      mtime: getMTime(yakPath)
    ))

method findByName*(self: FilesystemStorage, name: string): Yak =
  let yakPath = self.getYakPath(name)
  if not dirExists(yakPath):
    raise newException(NotFoundError, "Yak not found: " & name)

  let statePath = self.getStatePath(name)
  let contextPath = self.getContextPath(name)

  var context = ""
  if fileExists(contextPath):
    context = readFile(contextPath)

  return Yak(
    name: name,
    state: readState(statePath),
    context: context,
    mtime: getMTime(yakPath)
  )

method exists*(self: FilesystemStorage, name: string): bool =
  dirExists(self.getYakPath(name))

method save*(self: FilesystemStorage, yak: Yak) =
  let yakPath = self.getYakPath(yak.name)
  createDir(yakPath)

  let statePath = self.getStatePath(yak.name)
  writeFile(statePath, $yak.state)

  let contextPath = self.getContextPath(yak.name)
  writeFile(contextPath, yak.context)

method remove*(self: FilesystemStorage, name: string) =
  let yakPath = self.getYakPath(name)
  if dirExists(yakPath):
    removeDir(yakPath)

method updateState*(self: FilesystemStorage, name: string, state: YakState) =
  let statePath = self.getStatePath(name)
  writeFile(statePath, $state)

method updateContext*(self: FilesystemStorage, name: string, context: string) =
  let contextPath = self.getContextPath(name)
  writeFile(contextPath, context)

method rename*(self: FilesystemStorage, oldName: string, newName: string) =
  let oldPath = self.getYakPath(oldName)
  let newPath = self.getYakPath(newName)

  # Ensure parent directory exists
  let parentDir = parentDir(newPath)
  if parentDir != "" and not dirExists(parentDir):
    createDir(parentDir)

  moveDir(oldPath, newPath)

method hasChildren*(self: FilesystemStorage, name: string): bool =
  let yakPath = self.getYakPath(name)
  if not dirExists(yakPath):
    return false

  for kind, path in walkDir(yakPath):
    if kind == pcDir:
      return true
  return false

method getChildren*(self: FilesystemStorage, name: string): seq[Yak] =
  result = @[]
  let yakPath = self.getYakPath(name)
  if not dirExists(yakPath):
    return

  for kind, path in walkDir(yakPath):
    if kind == pcDir:
      let childName = name / path.splitPath.tail
      result.add(self.findByName(childName))
