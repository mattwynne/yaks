// Package git implements the domain.GitSync interface using git commands.
// It provides synchronization of yaks through git refs/notes/yaks, allowing
// distributed collaboration on task lists.
package git

import (
	"fmt"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
)

// Sync implements domain.GitSync using git commands
type Sync struct {
	gitWorkTree string
	yaksPath    string
}

// NewSync creates a new git sync adapter
func NewSync(gitWorkTree, yaksPath string) *Sync {
	return &Sync{
		gitWorkTree: gitWorkTree,
		yaksPath:    yaksPath,
	}
}

// IsGitRepository checks if we're in a git repository
func (s *Sync) IsGitRepository() bool {
	cmd := exec.Command("git", "-C", s.gitWorkTree, "rev-parse", "--git-dir")
	return cmd.Run() == nil
}

// HasOriginRemote checks if origin remote is configured
func (s *Sync) HasOriginRemote() bool {
	cmd := exec.Command("git", "-C", s.gitWorkTree, "remote", "get-url", "origin")
	return cmd.Run() == nil
}

// LogCommand logs a command to git refs/notes/yaks
func (s *Sync) LogCommand(message string) error {
	if !s.IsGitRepository() {
		return nil
	}

	if _, err := os.Stat(s.yaksPath); os.IsNotExist(err) {
		return nil
	}

	// Create temporary index
	tempIndex, err := os.CreateTemp("", "git-index-*")
	if err != nil {
		return err
	}
	tempIndexPath := tempIndex.Name()
	tempIndex.Close()
	defer os.Remove(tempIndexPath)

	// Read empty tree
	cmd := exec.Command("git", "-C", s.gitWorkTree, "read-tree", "--empty")
	cmd.Env = append(os.Environ(), "GIT_INDEX_FILE="+tempIndexPath, "GIT_WORK_TREE="+s.yaksPath)
	if err := cmd.Run(); err != nil {
		return err
	}

	// Add all files
	cmd = exec.Command("git", "-C", s.gitWorkTree, "add", ".")
	cmd.Env = append(os.Environ(), "GIT_INDEX_FILE="+tempIndexPath, "GIT_WORK_TREE="+s.yaksPath)
	if err := cmd.Run(); err != nil {
		return err
	}

	// Write tree
	cmd = exec.Command("git", "-C", s.gitWorkTree, "write-tree")
	cmd.Env = append(os.Environ(), "GIT_INDEX_FILE="+tempIndexPath)
	treeBytes, err := cmd.Output()
	if err != nil {
		return err
	}
	tree := strings.TrimSpace(string(treeBytes))

	// Get parent commit if exists
	var parentArgs []string
	cmd = exec.Command("git", "-C", s.gitWorkTree, "rev-parse", "refs/notes/yaks")
	if parentBytes, err := cmd.Output(); err == nil {
		parent := strings.TrimSpace(string(parentBytes))
		parentArgs = []string{"-p", parent}
	}

	// Create commit
	cmdArgs := []string{"-C", s.gitWorkTree, "commit-tree", tree}
	cmdArgs = append(cmdArgs, parentArgs...)
	cmdArgs = append(cmdArgs, "-m", message)
	cmd = exec.Command("git", cmdArgs...)
	commitBytes, err := cmd.Output()
	if err != nil {
		return err
	}
	commit := strings.TrimSpace(string(commitBytes))

	// Update ref
	cmd = exec.Command("git", "-C", s.gitWorkTree, "update-ref", "refs/notes/yaks", commit)
	return cmd.Run()
}

// Sync performs push/pull synchronization
func (s *Sync) Sync() error {
	if !s.IsGitRepository() {
		return fmt.Errorf("not in a git repository")
	}

	if !s.HasOriginRemote() {
		return fmt.Errorf("no origin remote configured")
	}

	// Fetch remote yaks
	cmd := exec.Command("git", "-C", s.gitWorkTree, "fetch", "origin", "refs/notes/yaks:refs/remotes/origin/yaks")
	cmd.Run() // Ignore error if remote ref doesn't exist

	remoteRef := s.getRemoteRef()
	localRef := s.getLocalRef()

	hasLocalChanges := s.detectLocalChanges(localRef)

	if hasLocalChanges && remoteRef != "" {
		// Rebase-style: overlay local changes on remote base
		if err := s.mergeRemoteIntoLocal(remoteRef); err != nil {
			return err
		}

		// Commit with remote as parent
		tempIndex, err := os.CreateTemp("", "git-index-*")
		if err != nil {
			return err
		}
		tempIndexPath := tempIndex.Name()
		tempIndex.Close()
		defer os.Remove(tempIndexPath)

		cmd = exec.Command("git", "-C", s.gitWorkTree, "read-tree", "--empty")
		cmd.Env = append(os.Environ(), "GIT_INDEX_FILE="+tempIndexPath, "GIT_WORK_TREE="+s.yaksPath)
		if err := cmd.Run(); err != nil {
			return err
		}

		cmd = exec.Command("git", "-C", s.gitWorkTree, "add", ".")
		cmd.Env = append(os.Environ(), "GIT_INDEX_FILE="+tempIndexPath, "GIT_WORK_TREE="+s.yaksPath)
		if err := cmd.Run(); err != nil {
			return err
		}

		cmd = exec.Command("git", "-C", s.gitWorkTree, "write-tree")
		cmd.Env = append(os.Environ(), "GIT_INDEX_FILE="+tempIndexPath)
		treeBytes, err := cmd.Output()
		if err != nil {
			return err
		}
		tree := strings.TrimSpace(string(treeBytes))

		cmd = exec.Command("git", "-C", s.gitWorkTree, "commit-tree", tree, "-p", remoteRef, "-m", "sync")
		commitBytes, err := cmd.Output()
		if err != nil {
			return err
		}
		commit := strings.TrimSpace(string(commitBytes))

		cmd = exec.Command("git", "-C", s.gitWorkTree, "update-ref", "refs/notes/yaks", commit)
		if err := cmd.Run(); err != nil {
			return err
		}
	} else if hasLocalChanges {
		// Local changes but no remote - just commit local
		if err := s.LogCommand("sync"); err != nil {
			return err
		}
	} else {
		// No local changes - use standard merge logic
		if err := s.mergeLocalAndRemote(localRef, remoteRef); err != nil {
			return err
		}
	}

	// Push to origin
	cmd = exec.Command("git", "-C", s.gitWorkTree, "rev-parse", "refs/notes/yaks")
	if cmd.Run() == nil {
		cmd = exec.Command("git", "-C", s.gitWorkTree, "push", "origin", "refs/notes/yaks:refs/notes/yaks")
		cmd.Run() // Ignore error
	}

	// Extract yaks to working directory
	if err := s.extractYaksToWorkingDir(); err != nil {
		return err
	}

	// Clean up remote ref
	cmd = exec.Command("git", "-C", s.gitWorkTree, "update-ref", "-d", "refs/remotes/origin/yaks")
	cmd.Run() // Ignore error

	return nil
}

func (s *Sync) getRemoteRef() string {
	cmd := exec.Command("git", "-C", s.gitWorkTree, "rev-parse", "refs/remotes/origin/yaks")
	if output, err := cmd.Output(); err == nil {
		return strings.TrimSpace(string(output))
	}
	return ""
}

func (s *Sync) getLocalRef() string {
	cmd := exec.Command("git", "-C", s.gitWorkTree, "rev-parse", "refs/notes/yaks")
	if output, err := cmd.Output(); err == nil {
		return strings.TrimSpace(string(output))
	}
	return ""
}

func (s *Sync) detectLocalChanges(localRef string) bool {
	if localRef == "" {
		// No local ref yet, check if yaks path has content
		if _, err := os.Stat(s.yaksPath); os.IsNotExist(err) {
			return false
		}
		entries, err := os.ReadDir(s.yaksPath)
		return err == nil && len(entries) > 0
	}

	if _, err := os.Stat(s.yaksPath); os.IsNotExist(err) {
		return false
	}

	// Create temp directories for comparison
	checkDir, err := os.MkdirTemp("", "check-*")
	if err != nil {
		return false
	}
	defer os.RemoveAll(checkDir)

	// Copy current yaks to check dir
	if err := copyDir(s.yaksPath, checkDir); err != nil {
		return false
	}

	// Extract ref to another temp dir
	refDir, err := os.MkdirTemp("", "ref-*")
	if err != nil {
		return false
	}
	defer os.RemoveAll(refDir)

	cmd := exec.Command("git", "-C", s.gitWorkTree, "archive", localRef)
	tarCmd := exec.Command("tar", "-x", "-C", refDir)
	pipe, err := cmd.StdoutPipe()
	if err != nil {
		return false
	}
	tarCmd.Stdin = pipe

	if err := tarCmd.Start(); err != nil {
		return false
	}
	if err := cmd.Run(); err != nil {
		return false
	}
	if err := tarCmd.Wait(); err != nil {
		return false
	}

	// Compare directories
	return !dirsEqual(checkDir, refDir)
}

func (s *Sync) mergeRemoteIntoLocal(remoteRef string) error {
	tempDir, err := os.MkdirTemp("", "merge-*")
	if err != nil {
		return err
	}
	defer os.RemoveAll(tempDir)

	// Extract remote to temp dir
	cmd := exec.Command("git", "-C", s.gitWorkTree, "archive", remoteRef)
	tarCmd := exec.Command("tar", "-x", "-C", tempDir)
	pipe, err := cmd.StdoutPipe()
	if err != nil {
		return err
	}
	tarCmd.Stdin = pipe

	if err := tarCmd.Start(); err != nil {
		return err
	}
	if err := cmd.Run(); err != nil {
		return err
	}
	if err := tarCmd.Wait(); err != nil {
		return err
	}

	// Copy local over remote
	if err := copyDir(s.yaksPath, tempDir); err != nil {
		return err
	}

	// Replace local with merged
	if err := os.RemoveAll(s.yaksPath); err != nil {
		return err
	}
	if err := os.MkdirAll(s.yaksPath, 0755); err != nil {
		return err
	}
	return copyDir(tempDir, s.yaksPath)
}

func (s *Sync) mergeLocalAndRemote(localRef, remoteRef string) error {
	if localRef == "" && remoteRef != "" {
		// Use remote only
		cmd := exec.Command("git", "-C", s.gitWorkTree, "update-ref", "refs/notes/yaks", remoteRef)
		return cmd.Run()
	}

	if localRef != "" && remoteRef == "" {
		// Use local only
		return nil
	}

	if localRef != "" && remoteRef != "" && localRef != remoteRef {
		// Check if local is ahead of remote
		cmd := exec.Command("git", "-C", s.gitWorkTree, "merge-base", "--is-ancestor", remoteRef, localRef)
		if cmd.Run() == nil {
			return nil
		}

		// Check if remote is ahead of local
		cmd = exec.Command("git", "-C", s.gitWorkTree, "merge-base", "--is-ancestor", localRef, remoteRef)
		if cmd.Run() == nil {
			cmd = exec.Command("git", "-C", s.gitWorkTree, "update-ref", "refs/notes/yaks", remoteRef)
			return cmd.Run()
		}

		// Neither is ahead - do a merge
		return s.mergeWithGitMergeTree(localRef, remoteRef)
	}

	return nil
}

func (s *Sync) mergeWithGitMergeTree(localRef, remoteRef string) error {
	cmd := exec.Command("git", "-C", s.gitWorkTree, "merge-tree", "--write-tree", "--allow-unrelated-histories", localRef, remoteRef)
	output, err := cmd.Output()
	if err != nil {
		return fmt.Errorf("ERROR: git merge-tree failed unexpectedly")
	}

	mergedTree := strings.TrimSpace(string(output))

	// Create merge commit
	cmd = exec.Command("git", "-C", s.gitWorkTree, "commit-tree", mergedTree, "-p", localRef, "-p", remoteRef, "-m", "Merge yaks")
	commitBytes, err := cmd.Output()
	if err != nil {
		return err
	}
	commit := strings.TrimSpace(string(commitBytes))

	cmd = exec.Command("git", "-C", s.gitWorkTree, "update-ref", "refs/notes/yaks", commit)
	return cmd.Run()
}

func (s *Sync) extractYaksToWorkingDir() error {
	if err := os.RemoveAll(s.yaksPath); err != nil {
		return err
	}
	if err := os.MkdirAll(s.yaksPath, 0755); err != nil {
		return err
	}

	cmd := exec.Command("git", "-C", s.gitWorkTree, "rev-parse", "refs/notes/yaks")
	if cmd.Run() != nil {
		return nil
	}

	cmd = exec.Command("git", "-C", s.gitWorkTree, "archive", "refs/notes/yaks")
	tarCmd := exec.Command("tar", "-x", "-C", s.yaksPath)
	pipe, err := cmd.StdoutPipe()
	if err != nil {
		return err
	}
	tarCmd.Stdin = pipe

	if err := tarCmd.Start(); err != nil {
		return err
	}
	if err := cmd.Run(); err != nil {
		return err
	}
	return tarCmd.Wait()
}

// Helper functions

func copyDir(src, dst string) error {
	return filepath.Walk(src, func(path string, info os.FileInfo, err error) error {
		if err != nil {
			return err
		}

		relPath, err := filepath.Rel(src, path)
		if err != nil {
			return err
		}

		dstPath := filepath.Join(dst, relPath)

		if info.IsDir() {
			return os.MkdirAll(dstPath, info.Mode())
		}

		srcFile, err := os.Open(path)
		if err != nil {
			return err
		}
		defer srcFile.Close()

		dstFile, err := os.Create(dstPath)
		if err != nil {
			return err
		}
		defer dstFile.Close()

		_, err = io.Copy(dstFile, srcFile)
		return err
	})
}

func dirsEqual(dir1, dir2 string) bool {
	cmd := exec.Command("diff", "-r", dir1, dir2)
	return cmd.Run() == nil
}
