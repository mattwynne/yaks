// Package cli provides the command-line interface adapter for the yaks system.
// It handles command parsing, user interaction, and output formatting.
package cli

import (
	"bufio"
	"fmt"
	"io"
	"os"
	"os/exec"
	"strings"

	"github.com/mattwynne/yaks/internal/adapters/terminal"
	"github.com/mattwynne/yaks/internal/domain"
)

// CLI handles command-line interface operations
type CLI struct {
	repo    domain.YakRepository
	gitSync domain.GitSync
	stdin   io.Reader
	stdout  io.Writer
	stderr  io.Writer
}

// NewCLI creates a new CLI
func NewCLI(repo domain.YakRepository, gitSync domain.GitSync) *CLI {
	return &CLI{
		repo:    repo,
		gitSync: gitSync,
		stdin:   os.Stdin,
		stdout:  os.Stdout,
		stderr:  os.Stderr,
	}
}

// Run executes a command
func (c *CLI) Run(args []string) int {
	if len(args) == 0 || args[0] == "--help" {
		c.showHelp()
		return 0
	}

	command := args[0]
	commandArgs := args[1:]

	switch command {
	case "add":
		return c.cmdAdd(commandArgs)
	case "list", "ls":
		return c.cmdList(commandArgs)
	case "done":
		return c.cmdDone(commandArgs)
	case "rm":
		return c.cmdRemove(commandArgs)
	case "prune":
		return c.cmdPrune(commandArgs)
	case "move", "mv":
		return c.cmdMove(commandArgs)
	case "context":
		return c.cmdContext(commandArgs)
	case "sync":
		return c.cmdSync(commandArgs)
	case "completions":
		return c.cmdCompletions(commandArgs)
	default:
		c.showHelp()
		return 0
	}
}

func (c *CLI) showHelp() {
	help := `Usage: yx <command> [arguments]

Commands:
  add <name>                      Add a new yak
  list, ls [--format FMT]         List all yaks
           [--only STATE]
                          --format: Output format
                                    markdown (or md): Checkbox format (default)
                                    plain (or raw): Simple list of names
                          --only: Show only yaks in a specific state
                                  not-done: Show only incomplete yaks
                                  done: Show only completed yaks
  context [--show] <name>         Edit context (uses $EDITOR) or set from stdin
                          --show: Display yak with context
                          --edit: Edit context (default)
  done <name>                     Mark a yak as done
  done --undo <name>              Unmark a yak as done
  rm <name>                       Remove a yak by name
  move <old> <new>                Rename a yak
  mv <old> <new>                  Alias for move
  prune                           Remove all done yaks
  sync                            Push and pull yaks to/from origin via git ref
  completions [cmd]               Output yak names for shell completion
  --help                          Show this help message
`
	fmt.Fprint(c.stdout, help)
}

func (c *CLI) cmdAdd(args []string) int {
	if len(args) == 0 {
		return c.addInteractive()
	}

	name := strings.Join(args, " ")
	if err := c.repo.Add(name); err != nil {
		fmt.Fprintf(c.stderr, "%v\n", err)
		return 1
	}

	if err := c.gitSync.LogCommand(fmt.Sprintf("add %s", name)); err != nil {
		fmt.Fprintf(c.stderr, "Warning: failed to log command: %v\n", err)
	}

	return 0
}

func (c *CLI) addInteractive() int {
	fmt.Fprintln(c.stdout, "Enter yaks (empty line to finish):")

	scanner := bufio.NewScanner(c.stdin)
	for scanner.Scan() {
		line := scanner.Text()
		if line == "" {
			break
		}

		if err := c.repo.Add(line); err != nil {
			fmt.Fprintf(c.stderr, "%v\n", err)
			return 1
		}

		if err := c.gitSync.LogCommand(fmt.Sprintf("add %s", line)); err != nil {
			fmt.Fprintf(c.stderr, "Warning: failed to log command: %v\n", err)
		}
	}

	if err := scanner.Err(); err != nil {
		fmt.Fprintf(c.stderr, "Error reading input: %v\n", err)
		return 1
	}

	return 0
}

func (c *CLI) cmdList(args []string) int {
	format := terminal.FormatMarkdown
	filter := terminal.FilterAll

	// Parse arguments
	for i := 0; i < len(args); i++ {
		switch args[i] {
		case "--format":
			if i+1 < len(args) {
				switch args[i+1] {
				case "markdown", "md":
					format = terminal.FormatMarkdown
				case "plain", "raw":
					format = terminal.FormatPlain
				}
				i++
			}
		case "--only":
			if i+1 < len(args) {
				switch args[i+1] {
				case "not-done":
					filter = terminal.FilterNotDone
				case "done":
					filter = terminal.FilterDone
				}
				i++
			}
		}
	}

	yaks, err := c.repo.List()
	if err != nil {
		fmt.Fprintf(c.stderr, "Error listing yaks: %v\n", err)
		return 1
	}

	output := terminal.FormatYaks(yaks, format, filter)
	if output != "" {
		fmt.Fprintln(c.stdout, output)
	}

	return 0
}

func (c *CLI) cmdDone(args []string) int {
	if len(args) == 0 {
		fmt.Fprintln(c.stderr, "Error: yak name required")
		return 1
	}

	if args[0] == "--undo" {
		if len(args) < 2 {
			fmt.Fprintln(c.stderr, "Error: yak name required")
			return 1
		}
		name := strings.Join(args[1:], " ")
		if err := c.repo.SetState(name, domain.StateTodo); err != nil {
			fmt.Fprintf(c.stderr, "%v\n", err)
			return 1
		}

		yak, _ := c.repo.Get(name)
		if yak != nil {
			if err := c.gitSync.LogCommand(fmt.Sprintf("done --undo %s", yak.Name)); err != nil {
				fmt.Fprintf(c.stderr, "Warning: failed to log command: %v\n", err)
			}
		}
		return 0
	}

	if args[0] == "--recursive" {
		if len(args) < 2 {
			fmt.Fprintln(c.stderr, "Error: yak name required")
			return 1
		}
		name := strings.Join(args[1:], " ")
		if err := c.repo.MarkDoneRecursively(name); err != nil {
			fmt.Fprintf(c.stderr, "%v\n", err)
			return 1
		}

		yak, _ := c.repo.Get(name)
		if yak != nil {
			if err := c.gitSync.LogCommand(fmt.Sprintf("done --recursive %s", yak.Name)); err != nil {
				fmt.Fprintf(c.stderr, "Warning: failed to log command: %v\n", err)
			}
		}
		return 0
	}

	name := strings.Join(args, " ")

	// Check for incomplete children
	hasIncomplete, err := c.repo.HasIncompleteChildren(name)
	if err != nil {
		fmt.Fprintf(c.stderr, "%v\n", err)
		return 1
	}

	if hasIncomplete {
		yak, _ := c.repo.Get(name)
		if yak != nil {
			fmt.Fprintf(c.stderr, "Error: cannot mark '%s' as done - it has incomplete children\n", yak.Name)
		} else {
			fmt.Fprintf(c.stderr, "Error: cannot mark '%s' as done - it has incomplete children\n", name)
		}
		return 1
	}

	if err := c.repo.SetState(name, domain.StateDone); err != nil {
		fmt.Fprintf(c.stderr, "%v\n", err)
		return 1
	}

	yak, _ := c.repo.Get(name)
	if yak != nil {
		if err := c.gitSync.LogCommand(fmt.Sprintf("done %s", yak.Name)); err != nil {
			fmt.Fprintf(c.stderr, "Warning: failed to log command: %v\n", err)
		}
	}

	return 0
}

func (c *CLI) cmdRemove(args []string) int {
	if len(args) == 0 {
		fmt.Fprintln(c.stderr, "Error: yak name required")
		return 1
	}

	name := strings.Join(args, " ")

	// Get the yak first to get resolved name
	yak, err := c.repo.Get(name)
	if err != nil {
		fmt.Fprintf(c.stderr, "%v\n", err)
		return 1
	}

	if err := c.repo.Remove(name); err != nil {
		fmt.Fprintf(c.stderr, "%v\n", err)
		return 1
	}

	if err := c.gitSync.LogCommand(fmt.Sprintf("rm %s", yak.Name)); err != nil {
		fmt.Fprintf(c.stderr, "Warning: failed to log command: %v\n", err)
	}

	return 0
}

func (c *CLI) cmdPrune(args []string) int {
	yaks, err := c.repo.List()
	if err != nil {
		fmt.Fprintf(c.stderr, "Error listing yaks: %v\n", err)
		return 1
	}

	for _, yak := range yaks {
		if yak.State == domain.StateDone {
			if err := c.repo.Remove(yak.Name); err != nil {
				fmt.Fprintf(c.stderr, "Error removing %s: %v\n", yak.Name, err)
			}
		}
	}

	return 0
}

func (c *CLI) cmdMove(args []string) int {
	if len(args) < 2 {
		fmt.Fprintln(c.stderr, "Error: old and new names required")
		return 1
	}

	oldName := args[0]
	newName := strings.Join(args[1:], " ")

	// Get the yak first to get resolved old name
	yak, err := c.repo.Get(oldName)
	if err != nil {
		fmt.Fprintf(c.stderr, "%v\n", err)
		return 1
	}

	if err := c.repo.Move(oldName, newName); err != nil {
		fmt.Fprintf(c.stderr, "%v\n", err)
		return 1
	}

	if err := c.gitSync.LogCommand(fmt.Sprintf("move %s %s", yak.Name, newName)); err != nil {
		fmt.Fprintf(c.stderr, "Warning: failed to log command: %v\n", err)
	}

	return 0
}

func (c *CLI) cmdContext(args []string) int {
	if len(args) == 0 {
		fmt.Fprintln(c.stderr, "Error: yak name required")
		return 1
	}

	if args[0] == "--show" {
		if len(args) < 2 {
			fmt.Fprintln(c.stderr, "Error: yak name required")
			return 1
		}
		name := strings.Join(args[1:], " ")
		yak, err := c.repo.Get(name)
		if err != nil {
			fmt.Fprintf(c.stderr, "%v\n", err)
			return 1
		}
		fmt.Fprintln(c.stdout, terminal.FormatYakWithContext(yak))
		return 0
	}

	// Handle --edit flag
	startIdx := 0
	if args[0] == "--edit" {
		startIdx = 1
	}

	if startIdx >= len(args) {
		fmt.Fprintln(c.stderr, "Error: yak name required")
		return 1
	}

	name := strings.Join(args[startIdx:], " ")

	// Check if stdin is a terminal
	stdinStat, _ := os.Stdin.Stat()
	if (stdinStat.Mode() & os.ModeCharDevice) != 0 {
		// Interactive mode - use editor
		yak, err := c.repo.Get(name)
		if err != nil {
			fmt.Fprintf(c.stderr, "%v\n", err)
			return 1
		}

		editor := os.Getenv("EDITOR")
		if editor == "" {
			editor = "vi"
		}

		// Create temp file with current context
		tmpfile, err := os.CreateTemp("", "yak-context-*.md")
		if err != nil {
			fmt.Fprintf(c.stderr, "Error creating temp file: %v\n", err)
			return 1
		}
		tmpfileName := tmpfile.Name()
		defer os.Remove(tmpfileName)

		if _, err := tmpfile.WriteString(yak.Context); err != nil {
			fmt.Fprintf(c.stderr, "Error writing temp file: %v\n", err)
			return 1
		}
		tmpfile.Close()

		// Open editor
		cmd := exec.Command(editor, tmpfileName)
		cmd.Stdin = os.Stdin
		cmd.Stdout = os.Stdout
		cmd.Stderr = os.Stderr
		if err := cmd.Run(); err != nil {
			fmt.Fprintf(c.stderr, "Error running editor: %v\n", err)
			return 1
		}

		// Read updated content
		content, err := os.ReadFile(tmpfileName)
		if err != nil {
			fmt.Fprintf(c.stderr, "Error reading temp file: %v\n", err)
			return 1
		}

		if err := c.repo.SetContext(name, string(content)); err != nil {
			fmt.Fprintf(c.stderr, "%v\n", err)
			return 1
		}

		if err := c.gitSync.LogCommand(fmt.Sprintf("context %s", yak.Name)); err != nil {
			fmt.Fprintf(c.stderr, "Warning: failed to log command: %v\n", err)
		}
	} else {
		// Pipe mode - read from stdin
		scanner := bufio.NewScanner(c.stdin)
		var lines []string
		for scanner.Scan() {
			lines = append(lines, scanner.Text())
		}
		if err := scanner.Err(); err != nil {
			fmt.Fprintf(c.stderr, "Error reading input: %v\n", err)
			return 1
		}

		content := strings.Join(lines, "\n")
		if err := c.repo.SetContext(name, content); err != nil {
			fmt.Fprintf(c.stderr, "%v\n", err)
			return 1
		}

		yak, _ := c.repo.Get(name)
		if yak != nil {
			if err := c.gitSync.LogCommand(fmt.Sprintf("context %s", yak.Name)); err != nil {
				fmt.Fprintf(c.stderr, "Warning: failed to log command: %v\n", err)
			}
		}
	}

	return 0
}

func (c *CLI) cmdSync(args []string) int {
	if err := c.gitSync.Sync(); err != nil {
		fmt.Fprintf(c.stderr, "%v\n", err)
		return 1
	}
	return 0
}

func (c *CLI) cmdCompletions(args []string) int {
	cmd := ""
	flag := ""
	if len(args) > 0 {
		cmd = args[0]
	}
	if len(args) > 1 {
		flag = args[1]
	}

	yaks, err := c.repo.List()
	if err != nil {
		return 1
	}

	for _, yak := range yaks {
		switch cmd {
		case "done":
			if flag == "--undo" {
				if yak.State == domain.StateDone {
					fmt.Fprintln(c.stdout, yak.Name)
				}
			} else {
				if yak.State != domain.StateDone {
					fmt.Fprintln(c.stdout, yak.Name)
				}
			}
		default:
			fmt.Fprintln(c.stdout, yak.Name)
		}
	}

	return 0
}
