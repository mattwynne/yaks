/// All available commands including aliases.
/// Kept in sync with the Commands enum in main.rs
/// via the `completions_match_cli_commands` test.
pub const COMMANDS: &[&str] = &[
    "add",
    "list",
    "ls",
    "done",
    "finish",
    "start",
    "wip",
    "remove",
    "rm",
    "move",
    "mv",
    "rename",
    "prune",
    "compact",
    "show",
    "context",
    "state",
    "field",
    "reset",
    "sync",
    "log",
    "tag",
    "tags",
    "completions",
];

pub fn complete_with_state(words: &[&str], yaks: &[(&str, bool)]) -> Vec<String> {
    let commands = COMMANDS;

    // Commands that take yak names as arguments
    let commands_with_yak_args = vec![
        "add", "done", "finish", "start", "wip", "remove", "rm", "move", "mv", "rename", "context",
        "state", "field", "show",
    ];

    // Flags for each command
    let command_flags = |cmd: &str| -> Vec<&str> {
        match cmd {
            "done" | "finish" | "start" | "wip" => vec!["--recursive"],
            "state" => vec!["--recursive"],
            "context" => vec!["--show"],
            "field" => vec!["--show"],
            "list" | "ls" => vec!["--format", "--only"],
            "show" => vec!["--format"],
            _ => vec![],
        }
    };

    // If we're completing the first argument (subcommand position)
    if words.len() <= 2 {
        let prefix = if words.len() == 2 { words[1] } else { "" };

        commands
            .iter()
            .filter(|cmd| cmd.starts_with(prefix))
            .map(|s| s.to_string())
            .collect()
    } else {
        // For arguments beyond the subcommand
        let subcommand = words[1];
        let prefix = words.last().unwrap_or(&"");
        let mut completions = Vec::new();

        // Get available flags for this command
        let flags = command_flags(subcommand);

        // Filter out flags that are already present in words
        let available_flags: Vec<_> = flags
            .into_iter()
            .filter(|flag| !words.contains(flag))
            .filter(|flag| flag.starts_with(prefix))
            .map(|s| s.to_string())
            .collect();

        completions.extend(available_flags);

        // If the command takes yak names, also offer yak completions
        if commands_with_yak_args.contains(&subcommand) {
            // Apply smart filtering for done/finish commands
            let filtered_yaks: Vec<_> = if subcommand == "done" || subcommand == "finish" {
                // Show only incomplete yaks for done operations
                yaks.iter().filter(|(_, is_done)| !*is_done).collect()
            } else {
                // For other commands, show all yaks
                yaks.iter().collect()
            };

            let yak_completions: Vec<String> = filtered_yaks
                .iter()
                .map(|(name, _)| *name)
                .filter(|yak| yak.starts_with(prefix))
                .map(|s| s.to_string())
                .collect();

            completions.extend(yak_completions);
        }

        completions
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
    fn add_offers_yak_names() {
        let yaks = &["my-yak"];
        let result = complete(&["yx", "add", ""], yaks);
        assert!(result.contains(&"my-yak".to_string()));
    }

    #[test]
    fn add_filters_yak_names_by_prefix() {
        let yaks = &["fix-bug", "write-docs"];
        let result = complete(&["yx", "add", "fix"], yaks);
        assert!(result.contains(&"fix-bug".to_string()));
        assert!(!result.contains(&"write-docs".to_string()));
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
    fn offers_flags_for_done() {
        let result = complete_with_state(&["yx", "done", "--"], &[]);
        assert!(result.contains(&"--recursive".to_string()));
        assert!(!result.contains(&"--undo".to_string()));
    }

    #[test]
    fn offers_show_flag_for_context() {
        let result = complete_with_state(&["yx", "context", "--"], &[]);
        assert!(result.contains(&"--show".to_string()));
    }

    #[test]
    fn offers_flags_and_yaks_together() {
        let yaks = &[("my-yak", false)];
        let result = complete_with_state(&["yx", "done", ""], yaks);
        assert!(result.contains(&"my-yak".to_string()));
        assert!(result.contains(&"--recursive".to_string()));
    }

    #[test]
    fn filters_flags_by_prefix() {
        let result = complete_with_state(&["yx", "done", "--r"], &[]);
        assert!(result.contains(&"--recursive".to_string()));
    }

    #[test]
    fn offers_recursive_flag_for_state() {
        let result = complete_with_state(&["yx", "state", "--"], &[]);
        assert!(result.contains(&"--recursive".to_string()));
    }

    #[test]
    fn offers_show_flag_for_field() {
        let result = complete_with_state(&["yx", "field", "--"], &[]);
        assert!(result.contains(&"--show".to_string()));
    }

    #[test]
    fn offers_format_and_only_flags_for_list() {
        let result = complete_with_state(&["yx", "list", "--"], &[]);
        assert!(result.contains(&"--format".to_string()));
        assert!(result.contains(&"--only".to_string()));
    }

    #[test]
    fn offers_format_and_only_flags_for_ls() {
        let result = complete_with_state(&["yx", "ls", "--"], &[]);
        assert!(result.contains(&"--format".to_string()));
        assert!(result.contains(&"--only".to_string()));
    }

    #[test]
    fn finish_shows_only_incomplete_yaks() {
        let yaks = &[("todo-yak", false), ("done-yak", true)];
        let result = complete_with_state(&["yx", "finish", ""], yaks);
        assert!(result.contains(&"todo-yak".to_string()));
        assert!(!result.contains(&"done-yak".to_string()));
    }
}
