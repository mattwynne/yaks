pub fn complete(words: &[&str], _yak_names: &[&str]) -> Vec<String> {
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

    // If we're completing the first argument (subcommand position)
    if words.len() <= 2 {
        let prefix = if words.len() == 2 { words[1] } else { "" };

        commands
            .into_iter()
            .filter(|cmd| cmd.starts_with(prefix))
            .map(|s| s.to_string())
            .collect()
    } else {
        // For now, no completions for arguments beyond the subcommand
        vec![]
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
}
