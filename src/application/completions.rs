pub fn complete(words: &[&str], yak_names: &[&str]) -> Vec<String> {
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
            yak_names
                .iter()
                .filter(|yak| yak.starts_with(prefix))
                .map(|s| s.to_string())
                .collect()
        } else {
            vec![]
        }
    }
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
}
