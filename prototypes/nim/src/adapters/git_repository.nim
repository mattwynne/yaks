## Git repository adapter - implements GitRepository using git commands

import ../ports/git
import std/[osproc, strutils, os, tempfiles]

type
  ShellGitRepository* = ref object of GitRepository
    workTree*: string

proc newShellGitRepository*(workTree: string): ShellGitRepository =
  ShellGitRepository(workTree: workTree)

proc runGit(self: ShellGitRepository, args: string): (string, int) =
  ## Run a git command and return (output, exitCode)
  let cmd = "git -C " & quoteShell(self.workTree) & " " & args
  let (output, exitCode) = execCmdEx(cmd)
  return (output.strip(), exitCode)

method isRepository*(self: ShellGitRepository): bool =
  let (_, exitCode) = self.runGit("rev-parse --git-dir")
  return exitCode == 0

method hasOrigin*(self: ShellGitRepository): bool =
  let (_, exitCode) = self.runGit("remote get-url origin")
  return exitCode == 0

method checkIgnored*(self: ShellGitRepository, path: string): bool =
  let (_, exitCode) = self.runGit("check-ignore -q " & path)
  return exitCode == 0

method logCommand*(self: ShellGitRepository, message: string) =
  ## Log a yak command to git refs/notes/yaks
  let yaksPath = self.workTree / ".yaks"
  if not dirExists(yaksPath):
    return

  # Create temporary index
  let (tmpFile, tmpPath) = createTempFile("yaks_index_", "")
  close(tmpFile)
  defer: removeFile(tmpPath)

  try:
    # Read tree into temp index
    discard execCmd("GIT_INDEX_FILE=" & quoteShell(tmpPath) &
                   " GIT_WORK_TREE=" & quoteShell(yaksPath) &
                   " git -C " & quoteShell(self.workTree) &
                   " read-tree --empty")

    # Add all files
    discard execCmd("GIT_INDEX_FILE=" & quoteShell(tmpPath) &
                   " GIT_WORK_TREE=" & quoteShell(yaksPath) &
                   " git -C " & quoteShell(self.workTree) &
                   " add .")

    # Write tree
    let (tree, _) = execCmdEx("GIT_INDEX_FILE=" & quoteShell(tmpPath) &
                              " git -C " & quoteShell(self.workTree) &
                              " write-tree")

    # Check for parent
    var parentArgs = ""
    let (_, checkCode) = self.runGit("rev-parse refs/notes/yaks")
    if checkCode == 0:
      let (parentSha, _) = self.runGit("rev-parse refs/notes/yaks")
      parentArgs = "-p " & parentSha.strip()

    # Commit tree
    let commitCmd = "commit-tree " & tree.strip() &
                   " " & parentArgs &
                   " -m " & quoteShell(message)
    let (newCommit, _) = self.runGit(commitCmd)

    # Update ref
    discard self.runGit("update-ref refs/notes/yaks " & newCommit.strip())
  except:
    discard

method sync*(self: ShellGitRepository) =
  ## Sync yaks with remote via git refs
  let yaksPath = self.workTree / ".yaks"

  # Fetch from origin
  discard self.runGit("fetch origin refs/notes/yaks:refs/remotes/origin/yaks")

  # Get refs
  let (localRef, localCode) = self.runGit("rev-parse refs/notes/yaks")
  let (remoteRef, remoteCode) = self.runGit("rev-parse refs/remotes/origin/yaks")

  # Detect local changes
  var hasLocalChanges = false
  if dirExists(yaksPath):
    for file in walkDirRec(yaksPath):
      hasLocalChanges = true
      break

  # Merge logic
  if hasLocalChanges and remoteCode == 0:
    # Extract remote to temp dir
    let tmpDir = createTempDir("yaks_merge_", "")
    defer: removeDir(tmpDir)

    discard self.runGit("archive " & remoteRef.strip() &
                       " | tar -x -C " & quoteShell(tmpDir))

    # Copy local on top
    if dirExists(yaksPath):
      for kind, path in walkDir(yaksPath):
        let dest = tmpDir / path.extractFilename()
        if kind == pcDir:
          copyDir(path, dest)
        else:
          copyFile(path, dest)

    # Replace yaks with merged content
    removeDir(yaksPath)
    moveDir(tmpDir, yaksPath)

    # Commit with remote as parent
    self.logCommand("sync")

  elif not hasLocalChanges and remoteCode == 0:
    # Just use remote
    discard self.runGit("update-ref refs/notes/yaks " & remoteRef.strip())

  # Extract to working directory
  if localCode == 0:
    if dirExists(yaksPath):
      removeDir(yaksPath)
    createDir(yaksPath)
    discard self.runGit("archive refs/notes/yaks | tar -x -C " &
                       quoteShell(yaksPath))

  # Push to origin
  if localCode == 0:
    discard self.runGit("push origin refs/notes/yaks:refs/notes/yaks")

  # Clean up remote ref
  discard self.runGit("update-ref -d refs/remotes/origin/yaks")
