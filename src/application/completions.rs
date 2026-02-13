pub fn complete_with_state(words: &[&str], yaks: &[(&str, bool)]) -> Vec<String> {
    // All available commands (including aliases)
    let commands = vec![
        "add",
        "list",
        "ls",
        "done",
        "finish",
        "remove",
        "rm",
        "move",
        "mv",
        "prune",
        "context",
        "state",
        "field",
        "sync",
        "log",
        "completions",
    ];

    // Commands that take yak names as arguments
    let commands_with_yak_args = vec![
        "done", "finish", "remove", "rm", "move", "mv", "context", "state", "field",
    ];

    // If we're completing the first argument (subcommand position)
    if words.len() <= 2 {
        let prefix = if words.len() == 2 { words[1] } else { "" };

        commands
            .into_iter()
            .filter(|cmd| cmd.starts_with(prefix))
            .map(|s| s.to_string())
            .collect()
    } else {
        // For arguments beyond the subcommand, check if the subcommand takes yak names
        let subcommand = words[1];
        if commands_with_yak_args.contains(&subcommand) {
            let prefix = words.last().unwrap_or(&"");

            // Apply smart filtering for done/finish commands
            let filtered_yaks: Vec<_> = if subcommand == "done" || subcommand == "finish" {
                // Check if --undo is present in the words
                let has_undo = words.contains(&"--undo");

                if has_undo {
                    // Show only done yaks for undo operations
                    yaks.iter().filter(|(_, is_done)| *is_done).collect()
                } else {
                    // Show only incomplete yaks for normal done operations
                    yaks.iter().filter(|(_, is_done)| !*is_done).collect()
                }
            } else {
                // For other commands, show all yaks
                yaks.iter().collect()
            };

            filtered_yaks
                .iter()
                .map(|(name, _)| *name)
                .filter(|yak| yak.starts_with(prefix))
                .map(|s| s.to_string())
                .collect()
        } else {
            vec![]
        }
    }
}

pub fn complete(words: &[&str], yak_names: &[&str]) -> Vec<String> {
    // Delegate to complete_with_state by converting yak names to (name, false) tuples
    let yaks_with_state: Vec<(&str, bool)> = yak_names.iter().map(|name| (*name, false)).collect();
    complete_with_state(words, &yaks_with_state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_all_commands_when_no_subcommand() {
        let result = complete(&["yx", ""], &[]);
        assert!(result.contains(&"add".to_string()));
        assert!(result.contains(&"list".to_string()));
        assert!(result.contains(&"ls".to_string()));
        assert!(result.contains(&"done".to_string()));
        assert!(result.contains(&"finish".to_string()));
        assert!(result.contains(&"remove".to_string()));
        assert!(result.contains(&"rm".to_string()));
        assert!(result.contains(&"context".to_string()));
        assert!(result.contains(&"state".to_string()));
        assert!(result.contains(&"field".to_string()));
        assert!(result.contains(&"sync".to_string()));
        assert!(result.contains(&"log".to_string()));
    }

    #[test]
    fn filters_commands_by_prefix() {
        let result = complete(&["yx", "re"], &[]);
        assert!(result.contains(&"remove".to_string()));
        assert!(!result.contains(&"add".to_string()));
    }

    #[test]
    fn completes_yak_names_for_rm() {
        let yaks = &["fix-bug", "write-docs"];
        let result = complete(&["yx", "rm", ""], yaks);
        assert!(result.contains(&"fix-bug".to_string()));
        assert!(result.contains(&"write-docs".to_string()));
    }

    #[test]
    fn filters_yak_names_by_prefix() {
        let yaks = &["fix-bug", "write-docs"];
        let result = complete(&["yx", "rm", "fix"], yaks);
        assert!(result.contains(&"fix-bug".to_string()));
        assert!(!result.contains(&"write-docs".to_string()));
    }

    #[test]
    fn completes_yak_names_for_context() {
        let yaks = &["my-yak"];
        let result = complete(&["yx", "context", ""], yaks);
        assert!(result.contains(&"my-yak".to_string()));
    }

    #[test]
    fn no_yak_names_for_add() {
        let yaks = &["my-yak"];
        let result = complete(&["yx", "add", ""], yaks);
        assert!(!result.contains(&"my-yak".to_string()));
    }

    #[test]
    fn no_yak_names_for_prune() {
        let yaks = &["my-yak"];
        let result = complete(&["yx", "prune", ""], yaks);
        assert!(!result.contains(&"my-yak".to_string()));
    }

    #[test]
    fn done_shows_only_incomplete_yaks() {
        let yaks = &[("todo-yak", false), ("done-yak", true)];
        let result = complete_with_state(&["yx", "done", ""], yaks);
        assert!(result.contains(&"todo-yak".to_string()));
        assert!(!result.contains(&"done-yak".to_string()));
    }

    #[test]
    fn done_undo_shows_only_done_yaks() {
        let yaks = &[("todo-yak", false), ("done-yak", true)];
        let result = complete_with_state(&["yx", "done", "--undo", ""], yaks);
        assert!(result.contains(&"done-yak".to_string()));
        assert!(!result.contains(&"todo-yak".to_string()));
    }
}
