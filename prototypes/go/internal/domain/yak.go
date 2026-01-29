package domain

import (
	"fmt"
	"regexp"
	"strings"
	"time"
)

// YakState represents the state of a yak
type YakState string

const (
	StateTodo YakState = "todo"
	StateDone YakState = "done"
)

// Yak represents a task in the system
type Yak struct {
	Name    string
	State   YakState
	Context string
	MTime   time.Time
}

// ValidateYakName checks if a yak name is valid
func ValidateYakName(name string) error {
	// Check for forbidden characters: \ : * ? | < > "
	forbidden := regexp.MustCompile(`[\\:*?|<>"]`)
	if forbidden.MatchString(name) {
		return fmt.Errorf("Invalid yak name: contains forbidden characters (\\ : * ? | < > \")")
	}
	return nil
}

// YakRepository defines the interface for storing and retrieving yaks
type YakRepository interface {
	// Add creates a new yak
	Add(name string) error

	// List returns all yaks
	List() ([]Yak, error)

	// Get retrieves a yak by name (supports fuzzy matching)
	Get(name string) (*Yak, error)

	// SetState updates the state of a yak
	SetState(name string, state YakState) error

	// SetContext updates the context of a yak
	SetContext(name string, context string) error

	// Remove deletes a yak
	Remove(name string) error

	// Move renames a yak
	Move(oldName, newName string) error

	// HasIncompleteChildren checks if a yak has incomplete children
	HasIncompleteChildren(name string) (bool, error)

	// MarkDoneRecursively marks a yak and all its children as done
	MarkDoneRecursively(name string) error
}

// GitSync defines the interface for git synchronization
type GitSync interface {
	// LogCommand logs a command to git refs
	LogCommand(message string) error

	// Sync performs push/pull synchronization
	Sync() error

	// IsGitRepository checks if we're in a git repository
	IsGitRepository() bool

	// HasOriginRemote checks if origin remote is configured
	HasOriginRemote() bool
}

// FuzzyMatcher handles finding yaks by partial name
type FuzzyMatcher struct {
	repo YakRepository
}

// NewFuzzyMatcher creates a new fuzzy matcher
func NewFuzzyMatcher(repo YakRepository) *FuzzyMatcher {
	return &FuzzyMatcher{repo: repo}
}

// FindYak finds a yak by exact or fuzzy match
func (f *FuzzyMatcher) FindYak(searchTerm string, yaks []Yak) (string, error) {
	// Try exact match first
	for _, yak := range yaks {
		if yak.Name == searchTerm {
			return yak.Name, nil
		}
	}

	// Try fuzzy match
	var matches []string
	for _, yak := range yaks {
		if strings.Contains(yak.Name, searchTerm) {
			matches = append(matches, yak.Name)
		}
	}

	if len(matches) == 0 {
		return "", fmt.Errorf("Error: yak '%s' not found", searchTerm)
	} else if len(matches) == 1 {
		return matches[0], nil
	} else {
		return "", fmt.Errorf("Error: yak name '%s' is ambiguous", searchTerm)
	}
}
