mod adapters;
mod application;
mod domain;
mod infrastructure;
mod ports;

use adapters::cli::ConsoleDisplay;
use adapters::input::ConsoleInput;
use adapters::storage::DirectoryStorage;
use adapters::sync::GitRefSync;
use adapters::event_store::GitEventStore;
use anyhow::Result;
use std::path::PathBuf;
use application::{
    AddYak, Application, DoneYak, EditContext, ListYaks, MoveYak, PruneYaks, RemoveYak, SetState,
    ShowContext, ShowField, SyncYaks, WriteField,
};
use clap::{CommandFactory, Parser};
use infrastructure::EventBus;
use ports::EventStore;

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
}

fn main() -> Result<()> {
    // Check if help was requested (--help or no args)
    let args: Vec<_> = std::env::args().collect();
    if args.len() == 1 || args.contains(&"--help".to_string()) || args.contains(&"-h".to_string()) {
        let mut cmd = Cli::command();
        let mut help_output = Vec::new();
        cmd.write_help(&mut help_output).unwrap();
        let help_str = String::from_utf8(help_output).unwrap();
        // Replace "Usage:" with "USAGE:" to match bash version
        let help_str = help_str.replace("Usage:", "USAGE:");
        eprintln!("{help_str}");
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

    // Create application with injected dependencies
    let mut app = Application::new(&mut event_bus, &storage, &display, &input);

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
        Commands::Sync => {
            let sync = GitRefSync::new()?;
            let use_case = SyncYaks::new(&sync, &display);
            use_case.execute()
        }
        Commands::Log => {
            let repo_path = std::env::var("GIT_WORK_TREE")
                .map(PathBuf::from)
                .unwrap_or(std::env::current_dir()?);
            let reader = GitEventStore::new(&repo_path)?;
            let events = reader.get_all_events()?;
            for event in events {
                println!("{}", event.format_message());
            }
            Ok(())
        }
    }
}
