package main

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"

	"github.com/mattwynne/yaks/internal/adapters/cli"
	"github.com/mattwynne/yaks/internal/adapters/filesystem"
	"github.com/mattwynne/yaks/internal/adapters/git"
)

func main() {
	// Get GIT_WORK_TREE
	gitWorkTree := os.Getenv("GIT_WORK_TREE")
	if gitWorkTree == "" {
		gitWorkTree = "."
	}

	// Convert to absolute path
	if !filepath.IsAbs(gitWorkTree) {
		wd, err := os.Getwd()
		if err != nil {
			fmt.Fprintf(os.Stderr, "Error getting working directory: %v\n", err)
			os.Exit(1)
		}
		gitWorkTree = filepath.Join(wd, gitWorkTree)
	}

	yaksPath := filepath.Join(gitWorkTree, ".yaks")

	// Check if help command
	args := os.Args[1:]
	if len(args) > 0 && args[0] == "--help" {
		repo := filesystem.NewRepository(yaksPath)
		gitSync := git.NewSync(gitWorkTree, yaksPath)
		app := cli.NewCLI(repo, gitSync)
		os.Exit(app.Run(args))
	}

	// Check if git is available (except for help)
	if len(args) > 0 {
		if _, err := os.Stat(filepath.Join(gitWorkTree, ".git")); os.IsNotExist(err) {
			fmt.Fprintln(os.Stderr, "Error: not in a git repository")
			fmt.Fprintln(os.Stderr, "yx must be run from within a git repository")
			os.Exit(1)
		}
	}

	// Check if .yaks is gitignored
	if len(args) > 0 {
		gitSync := git.NewSync(gitWorkTree, yaksPath)
		if gitSync.IsGitRepository() {
			// Use git check-ignore to verify
			if !isGitIgnored(gitWorkTree, ".yaks") {
				fmt.Fprintln(os.Stderr, "Error: .yaks folder is not gitignored")
				fmt.Fprintln(os.Stderr, "Please add .yaks to your .gitignore file")
				os.Exit(1)
			}
		}
	}

	// Migrate done files to state files if needed
	migrateOldDoneFiles(yaksPath)

	// Create adapters
	repo := filesystem.NewRepository(yaksPath)
	gitSync := git.NewSync(gitWorkTree, yaksPath)
	app := cli.NewCLI(repo, gitSync)

	// Run command
	os.Exit(app.Run(args))
}

func isGitIgnored(gitWorkTree, path string) bool {
	// Simple check: see if .gitignore exists and contains .yaks
	gitignorePath := filepath.Join(gitWorkTree, ".gitignore")
	content, err := os.ReadFile(gitignorePath)
	if err != nil {
		return false
	}

	return strings.Contains(string(content), ".yaks")
}

func migrateOldDoneFiles(yaksPath string) {
	if _, err := os.Stat(yaksPath); os.IsNotExist(err) {
		return
	}

	_ = filepath.Walk(yaksPath, func(path string, info os.FileInfo, err error) error {
		if err != nil {
			return err
		}

		if !info.IsDir() && info.Name() == "done" {
			yakDir := filepath.Dir(path)
			stateFile := filepath.Join(yakDir, "state")
			_ = os.WriteFile(stateFile, []byte("done"), 0644)
			_ = os.Remove(path)
		}

		return nil
	})
}
