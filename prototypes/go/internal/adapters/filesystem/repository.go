// Package filesystem implements the domain.YakRepository interface using
// the filesystem as storage. Each yak is stored as a directory under the
// configured yaks path, with state and context files.
package filesystem

import (
	"io/fs"
	"os"
	"path/filepath"
	"strings"

	"github.com/mattwynne/yaks/internal/domain"
)

// Repository implements domain.YakRepository using the filesystem
type Repository struct {
	yakPath string
}

// NewRepository creates a new filesystem repository
func NewRepository(yakPath string) *Repository {
	return &Repository{yakPath: yakPath}
}

// Add creates a new yak
func (r *Repository) Add(name string) error {
	if err := domain.ValidateYakName(name); err != nil {
		return err
	}

	yakDir := filepath.Join(r.yakPath, name)
	if err := os.MkdirAll(yakDir, 0755); err != nil {
		return err
	}

	stateFile := filepath.Join(yakDir, "state")
	if err := os.WriteFile(stateFile, []byte("todo"), 0644); err != nil {
		return err
	}

	contextFile := filepath.Join(yakDir, "context.md")
	if err := os.WriteFile(contextFile, []byte(""), 0644); err != nil {
		return err
	}

	return nil
}

// List returns all yaks
func (r *Repository) List() ([]domain.Yak, error) {
	if _, err := os.Stat(r.yakPath); os.IsNotExist(err) {
		return []domain.Yak{}, nil
	}

	var yaks []domain.Yak
	err := filepath.WalkDir(r.yakPath, func(path string, d fs.DirEntry, err error) error {
		if err != nil {
			return err
		}

		if !d.IsDir() {
			return nil
		}

		if path == r.yakPath {
			return nil
		}

		relPath, err := filepath.Rel(r.yakPath, path)
		if err != nil {
			return err
		}

		// Check if this is a yak directory (has state file)
		stateFile := filepath.Join(path, "state")
		if _, err := os.Stat(stateFile); os.IsNotExist(err) {
			return nil
		}

		yak, err := r.loadYak(relPath)
		if err != nil {
			return err
		}

		yaks = append(yaks, *yak)
		return nil
	})

	if err != nil {
		return nil, err
	}

	return yaks, nil
}

// Get retrieves a yak by name (supports fuzzy matching)
func (r *Repository) Get(name string) (*domain.Yak, error) {
	yaks, err := r.List()
	if err != nil {
		return nil, err
	}

	matcher := domain.NewFuzzyMatcher(r)
	resolvedName, err := matcher.FindYak(name, yaks)
	if err != nil {
		return nil, err
	}

	return r.loadYak(resolvedName)
}

// loadYak loads a yak from disk
func (r *Repository) loadYak(name string) (*domain.Yak, error) {
	yakDir := filepath.Join(r.yakPath, name)

	// Read state
	stateFile := filepath.Join(yakDir, "state")
	stateBytes, err := os.ReadFile(stateFile)
	if err != nil {
		return nil, err
	}
	state := domain.YakState(strings.TrimSpace(string(stateBytes)))

	// Read context
	contextFile := filepath.Join(yakDir, "context.md")
	contextBytes, err := os.ReadFile(contextFile)
	if err != nil && !os.IsNotExist(err) {
		return nil, err
	}
	context := string(contextBytes)

	// Get modification time
	info, err := os.Stat(yakDir)
	if err != nil {
		return nil, err
	}

	return &domain.Yak{
		Name:    name,
		State:   state,
		Context: context,
		MTime:   info.ModTime(),
	}, nil
}

// SetState updates the state of a yak
func (r *Repository) SetState(name string, state domain.YakState) error {
	yak, err := r.Get(name)
	if err != nil {
		return err
	}

	stateFile := filepath.Join(r.yakPath, yak.Name, "state")
	return os.WriteFile(stateFile, []byte(state), 0644)
}

// SetContext updates the context of a yak
func (r *Repository) SetContext(name string, context string) error {
	yak, err := r.Get(name)
	if err != nil {
		return err
	}

	contextFile := filepath.Join(r.yakPath, yak.Name, "context.md")
	return os.WriteFile(contextFile, []byte(context), 0644)
}

// Remove deletes a yak
func (r *Repository) Remove(name string) error {
	yak, err := r.Get(name)
	if err != nil {
		return err
	}

	yakDir := filepath.Join(r.yakPath, yak.Name)
	return os.RemoveAll(yakDir)
}

// Move renames a yak
func (r *Repository) Move(oldName, newName string) error {
	yak, err := r.Get(oldName)
	if err != nil {
		return err
	}

	if err := domain.ValidateYakName(newName); err != nil {
		return err
	}

	// Ensure parent directories exist
	if err := r.ensureParentYaksExist(newName); err != nil {
		return err
	}

	oldPath := filepath.Join(r.yakPath, yak.Name)
	newPath := filepath.Join(r.yakPath, newName)

	return os.Rename(oldPath, newPath)
}

// ensureParentYaksExist creates parent yaks if they don't exist
func (r *Repository) ensureParentYaksExist(name string) error {
	parentDir := filepath.Dir(name)
	if parentDir == "." {
		return nil
	}

	parts := strings.Split(parentDir, string(filepath.Separator))
	currentPath := ""
	for _, part := range parts {
		if currentPath == "" {
			currentPath = part
		} else {
			currentPath = filepath.Join(currentPath, part)
		}

		yakPath := filepath.Join(r.yakPath, currentPath)
		if _, err := os.Stat(yakPath); os.IsNotExist(err) {
			if err := os.MkdirAll(yakPath, 0755); err != nil {
				return err
			}
			stateFile := filepath.Join(yakPath, "state")
			if err := os.WriteFile(stateFile, []byte("todo"), 0644); err != nil {
				return err
			}
			contextFile := filepath.Join(yakPath, "context.md")
			if err := os.WriteFile(contextFile, []byte(""), 0644); err != nil {
				return err
			}
		}
	}

	return nil
}

// HasIncompleteChildren checks if a yak has incomplete children
func (r *Repository) HasIncompleteChildren(name string) (bool, error) {
	yak, err := r.Get(name)
	if err != nil {
		return false, err
	}

	yakPath := filepath.Join(r.yakPath, yak.Name)
	entries, err := os.ReadDir(yakPath)
	if err != nil {
		return false, err
	}

	hasChildren := false
	for _, entry := range entries {
		if !entry.IsDir() {
			continue
		}

		hasChildren = true
		childPath := filepath.Join(yak.Name, entry.Name())
		child, err := r.loadYak(childPath)
		if err != nil {
			continue
		}

		if child.State != domain.StateDone {
			return true, nil
		}
	}

	return hasChildren && false, nil
}

// MarkDoneRecursively marks a yak and all its children as done
func (r *Repository) MarkDoneRecursively(name string) error {
	yak, err := r.Get(name)
	if err != nil {
		return err
	}

	// Mark this yak as done
	if err := r.SetState(yak.Name, domain.StateDone); err != nil {
		return err
	}

	// Recursively mark all children as done
	yakPath := filepath.Join(r.yakPath, yak.Name)
	entries, err := os.ReadDir(yakPath)
	if err != nil {
		return err
	}

	for _, entry := range entries {
		if !entry.IsDir() {
			continue
		}

		childPath := filepath.Join(yak.Name, entry.Name())
		if err := r.MarkDoneRecursively(childPath); err != nil {
			return err
		}
	}

	return nil
}
