use anyhow::Result;
use clap::{CommandFactory, Parser};
use std::path::PathBuf;
use yx::adapters::cli::ConsoleDisplay;
use yx::adapters::event_store::GitEventStore;
use yx::adapters::input::ConsoleInput;
use yx::adapters::storage::DirectoryStorage;
use yx::adapters::sync::GitRefSync;
use yx::application::{
    complete_with_state, AddYak, Application, DoneYak, EditContext, ListYaks, MoveYak, PruneYaks,
    RemoveYak, SetState, ShowContext, ShowField, ShowLog, SyncYaks, WriteField,
};
use yx::infrastructure::EventBus;
use yx::ports::ReadYakStore;

/// DAG-based TODO list CLI for software teams
#[derive(Parser, Debug)]
#[command(name = "yx")]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Parser, Debug)]
enum Commands {
    /// Add a new yak
    Add {
        /// The yak name (space-separated words)
        name: Vec<String>,
    },
    /// List yaks
    #[command(alias = "ls")]
    List {
        #[arg(
            long,
            default_value = "pretty",
            help = "Output format: pretty (default), markdown, plain",
            long_help = "Output format:\n  - pretty: Unicode box-drawing with colored status dots\n  - markdown: Checkbox-style list with indentation\n  - plain: Just yak names, one per line"
        )]
        format: String,
        /// Filter by completion status (done, not-done)
        #[arg(long)]
        only: Option<String>,
    },
    /// Mark yak as done
    #[command(alias = "finish")]
    Done {
        /// The yak name (space-separated words)
        name: Vec<String>,
        #[arg(long)]
        undo: bool,
        /// Mark yak and all children as done recursively
        #[arg(long)]
        recursive: bool,
    },
    /// Remove a yak
    #[command(alias = "rm")]
    Remove {
        /// The yak name (space-separated words)
        name: Vec<String>,
    },
    /// Remove all done yaks
    Prune,
    /// Move/rename a yak
    #[command(alias = "mv")]
    Move { from: String, to: String },
    /// Edit or show yak context
    Context {
        /// The yak name (space-separated words)
        name: Vec<String>,
        #[arg(long)]
        show: bool,
    },
    /// Set the state of a yak
    State {
        /// The yak name (space-separated words)
        name: Vec<String>,
        /// The state to set (e.g., "todo", "wip", "done")
        state: String,
    },
    /// Write or show custom field for a yak
    Field {
        /// The yak name (space-separated words)
        name: Vec<String>,
        /// The field name (e.g., "notes", "priority", "notes.txt")
        field: String,
        #[arg(long)]
        show: bool,
    },
    /// Sync yaks with git refs
    Sync,
    /// Show event log from refs/notes/yaks
    Log,
    /// Generate shell completions (hidden)
    #[command(hide = true)]
    Completions {
        #[arg(last = true)]
        words: Vec<String>,
    },
}

fn main() -> Result<()> {
    // Show help on stderr when run with no arguments
    let args: Vec<_> = std::env::args().collect();
    if args.len() == 1 {
        Cli::command().print_help()?;
        return Ok(());
    }

    let cli = Cli::parse();

    // Initialize event infrastructure
    // Determine repo path: GIT_WORK_TREE env var, then current dir
    let repo_path = std::env::var("GIT_WORK_TREE")
        .map(PathBuf::from)
        .unwrap_or(std::env::current_dir()?);
    let event_store = GitEventStore::new(&repo_path)?;
    let mut event_bus = EventBus::new(Box::new(event_store));

    // Initialize storage and register as projection
    let storage = DirectoryStorage::new()?;
    event_bus.register(Box::new(storage.clone()));

    // Initialize other adapters
    let display = ConsoleDisplay;
    let input = ConsoleInput;
    let sync = GitRefSync::new()?;
    let event_reader = GitEventStore::new(&repo_path)?;

    // Create application with injected dependencies
    let mut app = Application::new(
        &mut event_bus,
        &storage,
        &display,
        &input,
        Some(&sync),
        Some(&event_reader),
    );

    match cli.command {
        Commands::Add { name } => {
            let name_str = name.join(" ");
            app.handle(AddYak::new(&name_str))
        }
        Commands::List { format, only } => app.handle(ListYaks::new(&format, only.as_deref())),
        Commands::Done {
            name,
            undo,
            recursive,
        } => {
            let name_str = name.join(" ");
            app.handle(DoneYak::new(&name_str, undo, recursive))
        }
        Commands::Remove { name } => {
            let name_str = name.join(" ");
            app.handle(RemoveYak::new(&name_str))
        }
        Commands::Prune => app.handle(PruneYaks::new()),
        Commands::Move { from, to } => app.handle(MoveYak::new(&from, &to)),
        Commands::Context { name, show } => {
            let name_str = name.join(" ");
            if show {
                app.handle(ShowContext::new(&name_str))
            } else {
                app.handle(EditContext::new(&name_str))
            }
        }
        Commands::State { name, state } => {
            let name_str = name.join(" ");
            app.handle(SetState::new(&name_str, &state))
        }
        Commands::Field { name, field, show } => {
            let name_str = name.join(" ");
            if show {
                app.handle(ShowField::new(&name_str, &field))
            } else {
                app.handle(WriteField::new(&name_str, &field))
            }
        }
        Commands::Sync => app.handle(SyncYaks::new()),
        Commands::Log => app.handle(ShowLog::new()),
        Commands::Completions { words } => {
            // Get yaks with state from storage
            let yaks = storage.list_yaks()?;

            // Build tuples of (name, is_done)
            let yak_name_strings: Vec<String> = yaks.iter().map(|y| y.name.clone()).collect();
            let yaks_with_state: Vec<(&str, bool)> = yak_name_strings
                .iter()
                .zip(yaks.iter())
                .map(|(name, yak)| (name.as_str(), yak.is_done()))
                .collect();

            // Convert words to &str slice
            let word_refs: Vec<&str> = words.iter().map(|s| s.as_str()).collect();

            // Call the complete_with_state function
            let results = complete_with_state(&word_refs, &yaks_with_state);

            // Print each result on a separate line
            for result in results {
                println!("{}", result);
            }

            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use yx::application::COMMANDS;

    #[test]
    fn completions_match_cli_commands() {
        let cli = Cli::command();
        let mut clap_names: BTreeSet<String> = BTreeSet::new();
        for sub in cli.get_subcommands() {
            clap_names.insert(sub.get_name().to_string());
            for alias in sub.get_all_aliases() {
                clap_names.insert(alias.to_string());
            }
        }

        let completion_names: BTreeSet<String> = COMMANDS.iter().map(|s| s.to_string()).collect();

        let missing: Vec<_> = clap_names.difference(&completion_names).collect();
        let extra: Vec<_> = completion_names.difference(&clap_names).collect();

        assert!(
            missing.is_empty() && extra.is_empty(),
            "Completion commands out of sync with CLI!\n  \
             Missing from completions: {:?}\n  \
             Extra in completions: {:?}",
            missing,
            extra,
        );
    }
}
