package terminal

import (
	"fmt"
	"path/filepath"
	"sort"
	"strings"

	"github.com/mattwynne/yaks/internal/domain"
)

// OutputFormat defines the output format type
type OutputFormat string

const (
	FormatMarkdown OutputFormat = "markdown"
	FormatPlain    OutputFormat = "plain"
	FormatRaw      OutputFormat = "raw"
)

// FilterState defines the filter for listing yaks
type FilterState string

const (
	FilterAll     FilterState = ""
	FilterNotDone FilterState = "not-done"
	FilterDone    FilterState = "done"
)

// FormatYaks formats yaks for display
func FormatYaks(yaks []domain.Yak, format OutputFormat, filter FilterState) string {
	if len(yaks) == 0 {
		if format == FormatPlain || format == FormatRaw {
			return ""
		}
		return "You have no yaks. Are you done?"
	}

	// Filter yaks
	var filtered []domain.Yak
	for _, yak := range yaks {
		if shouldDisplay(yak, filter) {
			filtered = append(filtered, yak)
		}
	}

	// Sort yaks
	sorted := sortYaks(filtered)

	// Build hierarchical structure
	var lines []string
	displayed := make(map[string]bool)

	var displayDir func(parent string, depth int)
	displayDir = func(parent string, depth int) {
		children := getChildren(sorted, parent)

		for _, yak := range children {
			if displayed[yak.Name] {
				continue
			}
			displayed[yak.Name] = true

			line := formatYak(yak, depth, format)
			if line != "" {
				lines = append(lines, line)
			}

			displayDir(yak.Name, depth+1)
		}
	}

	displayDir("", 0)

	return strings.Join(lines, "\n")
}

func shouldDisplay(yak domain.Yak, filter FilterState) bool {
	switch filter {
	case FilterNotDone:
		return yak.State != domain.StateDone
	case FilterDone:
		return yak.State == domain.StateDone
	default:
		return true
	}
}

func sortYaks(yaks []domain.Yak) []domain.Yak {
	sorted := make([]domain.Yak, len(yaks))
	copy(sorted, yaks)

	sort.Slice(sorted, func(i, j int) bool {
		// Priority: done yaks first (0), then todo (1)
		priI := 1
		if sorted[i].State == domain.StateDone {
			priI = 0
		}
		priJ := 1
		if sorted[j].State == domain.StateDone {
			priJ = 0
		}

		if priI != priJ {
			return priI < priJ
		}

		// Then by modification time
		if !sorted[i].MTime.Equal(sorted[j].MTime) {
			return sorted[i].MTime.Before(sorted[j].MTime)
		}

		// Finally by name
		return sorted[i].Name < sorted[j].Name
	})

	return sorted
}

func getChildren(yaks []domain.Yak, parent string) []domain.Yak {
	var children []domain.Yak
	for _, yak := range yaks {
		yakParent := filepath.Dir(yak.Name)
		if parent == "" && yakParent == "." {
			children = append(children, yak)
		} else if yakParent == parent {
			children = append(children, yak)
		}
	}
	return children
}

func formatYak(yak domain.Yak, depth int, format OutputFormat) string {
	displayName := filepath.Base(yak.Name)

	switch format {
	case FormatPlain, FormatRaw:
		return yak.Name
	case FormatMarkdown:
		indent := strings.Repeat("  ", depth)
		if yak.State == domain.StateDone {
			return fmt.Sprintf("\033[90m%s- [x] %s\033[0m", indent, displayName)
		}
		return fmt.Sprintf("%s- [ ] %s", indent, displayName)
	default:
		return ""
	}
}

// FormatYakWithContext formats a yak with its context
func FormatYakWithContext(yak *domain.Yak) string {
	var sb strings.Builder
	sb.WriteString(yak.Name)
	if yak.Context != "" {
		sb.WriteString("\n\n")
		sb.WriteString(yak.Context)
	}
	return sb.String()
}
