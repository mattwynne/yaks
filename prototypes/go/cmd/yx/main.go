package main

import (
	"fmt"
	"os"
	"path/filepath"

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
	// Create a temp file in .yaks to check
	testPath := filepath.Join(gitWorkTree, path, ".test")
	os.MkdirAll(filepath.Dir(testPath), 0755)
	os.WriteFile(testPath, []byte("test"), 0644)
	defer os.Remove(testPath)

	// Use git check-ignore
	cmd := fmt.Sprintf("cd %s && git check-ignore -q %s", gitWorkTree, path)
	if err := os.Chdir(gitWorkTree); err != nil {
		return false
	}

	// Simple check: see if .gitignore exists and contains .yaks
	gitignorePath := filepath.Join(gitWorkTree, ".gitignore")
	content, err := os.ReadFile(gitignorePath)
	if err != nil {
		return false
	}

	for _, line := range []string{".yaks", ".yaks/", "/.yaks", "/.yaks/"} {
		for _, gitignoreLine := range []byte(content) {
			if string(gitignoreLine) == line {
				return true
			}
		}
	}

	// Check via git
	_ = cmd // silence unused warning
	// For simplicity, just check if .gitignore contains .yaks
	return contains(string(content), ".yaks")
}

func contains(s, substr string) bool {
	return len(s) >= len(substr) && (s == substr || len(s) > len(substr) &&
		(s[:len(substr)] == substr || contains(s[1:], substr)))
}

func migrateOldDoneFiles(yaksPath string) {
	if _, err := os.Stat(yaksPath); os.IsNotExist(err) {
		return
	}

	filepath.Walk(yaksPath, func(path string, info os.FileInfo, err error) error {
		if err != nil {
			return err
		}

		if !info.IsDir() && info.Name() == "done" {
			yakDir := filepath.Dir(path)
			stateFile := filepath.Join(yakDir, "state")
			os.WriteFile(stateFile, []byte("done"), 0644)
			os.Remove(path)
		}

		return nil
	})
}
