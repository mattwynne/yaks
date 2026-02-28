use anyhow::Result;
use clap::{CommandFactory, Parser};
use std::path::PathBuf;
use yx::adapters::authentication::GitAuthentication;
use yx::adapters::event_store::migration::Migrator;
use yx::adapters::event_store::{GitEventStore, NoOpEventStore};
use yx::adapters::user_display::ConsoleDisplay;
use yx::adapters::user_input::ConsoleInput;
use yx::adapters::yak_store::DirectoryStorage;
use yx::application::{
    complete_with_state, AddYak, Application, CompactEvents, DoneYak, EditContext, ListYaks, MoveYak, PruneYaks,
    RemoveYak, RenameYak, SetState, ShowContext, ShowField, ShowLog, ShowYak, StartYak, SyncYaks,
    WriteField,
};
use yx::domain::ports::{EventListener, EventStore, ReadYakStore};
use yx::infrastructure::EventBus;

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
        /// Nest under this parent yak
        #[arg(long, aliases = ["below", "in", "into", "blocks"])]
        under: Option<String>,
        /// Initial state (todo, wip, done)
        #[arg(long)]
        state: Option<String>,
        /// Set context directly
        #[arg(long, conflicts_with = "edit")]
        context: Option<String>,
        /// Launch $EDITOR for initial context
        #[arg(long, conflicts_with = "context")]
        edit: bool,
        /// Use a specific ID instead of auto-generating
        #[arg(long)]
        id: Option<String>,
        /// Set a custom field (format: key=value, repeatable)
        #[arg(long = "field", value_parser = parse_field_arg)]
        fields: Vec<(String, String)>,
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
        /// Mark yak and all children as done recursively
        #[arg(long)]
        recursive: bool,
    },
    /// Start working on a yak (set state to wip)
    #[command(alias = "wip")]
    Start {
        /// The yak name (space-separated words)
        name: Vec<String>,
        /// Start yak and all children recursively
        #[arg(long)]
        recursive: bool,
    },
    /// Remove a yak
    #[command(alias = "rm")]
    Remove {
        /// The yak name (space-separated words)
        name: Vec<String>,
        /// Remove yak and all its children recursively
        #[arg(long)]
        recursive: bool,
    },
    /// Remove all done yaks
    Prune,
    /// Move a yak in the hierarchy
    #[command(alias = "mv")]
    Move {
        /// The yak to move (space-separated words)
        name: Vec<String>,
        /// Move under this parent yak
        #[arg(
            long,
            aliases = ["below", "in", "into", "blocks"],
            conflicts_with = "to_root",
            required_unless_present = "to_root"
        )]
        under: Option<Vec<String>>,
        /// Move to root level (un-nest)
        #[arg(long, conflicts_with = "under", required_unless_present = "under")]
        to_root: bool,
    },
    /// Rename a yak (change name without moving)
    Rename {
        /// Current yak name
        from: String,
        /// New name
        to: String,
    },
    /// Show yak details
    Show {
        /// The yak name (space-separated words)
        name: Vec<String>,
    },
    /// Show or edit yak context
    Context {
        /// The yak name (space-separated words)
        name: Vec<String>,
        /// Show context (default when no stdin is piped)
        #[arg(long)]
        show: bool,
        /// Edit context interactively ($EDITOR)
        #[arg(long)]
        edit: bool,
    },
    /// Set the state of a yak
    State {
        /// The yak name (space-separated words)
        #[arg(required = true)]
        name: Vec<String>,
        /// The state to set (e.g., "todo", "wip", "done")
        state: String,
        /// Apply state change recursively to all descendants
        #[arg(long)]
        recursive: bool,
    },
    /// Show or edit custom field for a yak
    Field {
        /// The yak name (space-separated words)
        #[arg(required = true)]
        name: Vec<String>,
        /// The field name (e.g., "notes", "priority", "notes.txt")
        field: String,
        /// Show field (default when no stdin is piped)
        #[arg(long)]
        show: bool,
        /// Edit field interactively ($EDITOR)
        #[arg(long)]
        edit: bool,
    },
    /// Rebuild yaks from the git event store tree
    Reset {
        /// Rebuild .yaks directory from git tree (default)
        #[arg(long)]
        disk_from_git: bool,
        /// Wipe git history and replay yaks from disk through Application layer
        #[arg(long)]
        git_from_disk: bool,
    },
    /// Compact the event stream into a snapshot
    Compact {
        /// Skip confirmation prompt
        #[arg(long)]
        yes: bool,
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

fn parse_field_arg(s: &str) -> Result<(String, String), String> {
    let (key, value) = s
        .split_once('=')
        .ok_or_else(|| format!("invalid field format '{}', expected key=value", s))?;
    Ok((key.to_string(), value.to_string()))
}

/// Fallback authentication adapter used when not in a git repository
struct UnknownAuthentication;

impl yx::domain::ports::AuthenticationPort for UnknownAuthentication {
    fn current_author(&self) -> yx::domain::event_metadata::Author {
        yx::domain::event_metadata::Author::unknown()
    }
}

#[allow(clippy::cognitive_complexity)]
fn main() -> Result<()> {
    // Show help on stderr when run with no arguments
    let args: Vec<_> = std::env::args().collect();
    if args.len() == 1 {
        Cli::command().print_help()?;
        return Ok(());
    }

    let cli = Cli::parse();

    let skip_git = std::env::var("YX_SKIP_GIT_CHECKS").is_ok();

    // Initialize event infrastructure
    // Discover git repo root using libgit2
    let repo_root = yx::infrastructure::discover_git_root().ok();

    // Resolve yaks path once: YAK_PATH env var, or <repo_root>/.yaks, or .yaks fallback
    let yaks_path: PathBuf = if let Ok(yak_path) = std::env::var("YAK_PATH") {
        PathBuf::from(yak_path)
    } else if let Some(ref root) = repo_root {
        root.join(".yaks")
    } else {
        PathBuf::from(".yaks")
    };

    let needs_projection_reset;
    let mut event_store: Box<dyn EventStore> = if let Some(ref root) = repo_root {
        // Run schema migration before using the event store.
        // Returns true if migrations ran (projection needs rebuilding).
        needs_projection_reset = Migrator::for_current_version().run(root, "refs/notes/yaks")?;
        Box::new(GitEventStore::new(root)?)
    } else if skip_git {
        needs_projection_reset = false;
        // Outside a git repo but skipping git checks: use a no-op store
        Box::new(NoOpEventStore)
    } else {
        // Outside a git repo and not skipping checks: error out
        anyhow::bail!("Error: not in a git repository");
    };

    let mut event_bus = EventBus::new();

    // Initialize storage and register as projection
    let storage = if let Some(ref root) = repo_root {
        DirectoryStorage::new(root, &yaks_path)?
    } else {
        // skip_git is true (otherwise we bailed above)
        DirectoryStorage::without_git(&yaks_path)?
    };
    event_bus.register(Box::new(storage.clone()));

    // After migration, rebuild the disk projection from the compacted event store.
    // This clears old files (e.g. .metadata.json) and writes the current format.
    if needs_projection_reset {
        let all_events = event_store.get_all_events()?;
        event_bus.rebuild(&all_events)?;
    }

    // Initialize other adapters
    let display = ConsoleDisplay::stdout();
    let input = ConsoleInput;

    let git_event_reader = if let Some(ref root) = repo_root {
        GitEventStore::new(root).ok()
    } else {
        None
    };

    // Initialize authentication: use git config when in a repo, fallback otherwise
    let auth: Box<dyn yx::domain::ports::AuthenticationPort> = if let Some(ref root) = repo_root {
        Box::new(GitAuthentication::new(root)?)
    } else {
        // skip_git mode: no git repo available, use unknown author
        Box::new(UnknownAuthentication)
    };

    // Create application with injected dependencies
    let mut app = Application::new(
        event_store.as_mut(),
        &mut event_bus,
        &storage,
        &display,
        &input,
        git_event_reader
            .as_ref()
            .map(|r| r as &dyn yx::domain::ports::EventStoreReader),
        auth.as_ref(),
    );

    match cli.command {
        Commands::Add {
            name,
            under,
            state,
            context,
            edit,
            id,
            fields,
        } => {
            let name_str = name.join(" ");
            // Resolve context: --context, --edit (editor), piped stdin
            let context = if context.is_some() {
                context
            } else if edit {
                let input = ConsoleInput;
                let template = format!("# {}\n\n", name_str);
                input
                    .edit_content(None, Some(&template))?
                    .filter(|c| !c.trim().is_empty())
            } else {
                let input = ConsoleInput;
                input.read_stdin_content().ok().flatten()
            };
            let mut use_case = AddYak::new(&name_str)
                .with_parent(under.as_deref())
                .with_state(state.as_deref())
                .with_context(context.as_deref())
                .with_id(id.as_deref());
            for (key, value) in &fields {
                use_case = use_case.with_field(key, value);
            }
            app.handle(use_case)
        }
        Commands::List { format, only } => app.handle(ListYaks::new(&format, only.as_deref())),
        Commands::Done { name, recursive } => {
            let name_str = name.join(" ");
            app.handle(DoneYak::new(&name_str, recursive))
        }
        Commands::Start { name, recursive } => {
            let name_str = name.join(" ");
            app.handle(StartYak::new(&name_str, recursive))
        }
        Commands::Remove { name, recursive } => {
            let name_str = name.join(" ");
            app.handle(RemoveYak::new(&name_str).with_recursive(recursive))
        }
        Commands::Prune => app.handle(PruneYaks::new()),
        Commands::Move {
            name,
            under,
            to_root,
        } => {
            let name_str = name.join(" ");
            if to_root {
                app.handle(MoveYak::to_root(&name_str))
            } else {
                let parent_str = under.unwrap().join(" ");
                app.handle(MoveYak::under(&name_str, &parent_str))
            }
        }
        Commands::Rename { from, to } => app.handle(RenameYak::new(&from, &to)),
        Commands::Show { name } => {
            let name_str = name.join(" ");
            app.handle(ShowYak::new(&name_str))
        }
        Commands::Context {
            name,
            show: _,
            edit,
        } => {
            let name_str = name.join(" ");
            if edit {
                let mut use_case = EditContext::new(&name_str);
                // If stdin has data, use it as initial content for the editor
                if ConsoleInput::stdin_has_readable_data() {
                    let input = ConsoleInput;
                    if let Some(stdin_content) = input.read_stdin_content()? {
                        use_case = use_case.with_initial_content(&stdin_content);
                    }
                }
                app.handle(use_case)
            } else if ConsoleInput::stdin_has_readable_data() {
                app.handle(EditContext::new(&name_str))
            } else {
                // Default (no piped data, no --edit): show
                // --show kept for backward compat
                app.handle(ShowContext::new(&name_str))
            }
        }
        Commands::State {
            name,
            state,
            recursive,
        } => {
            let name_str = name.join(" ");
            app.handle(SetState::new(&name_str, &state).with_recursive(recursive))
        }
        Commands::Field {
            name,
            field,
            show: _,
            edit,
        } => {
            let name_str = name.join(" ");
            if edit {
                let input = ConsoleInput;
                // If stdin has data, use it as initial content; otherwise use existing field
                let initial = if ConsoleInput::stdin_has_readable_data() {
                    input.read_stdin_content()?.unwrap_or_default()
                } else {
                    let yak = app
                        .store
                        .get_yak(&app.store.fuzzy_find_yak_id(&name_str)?)?;
                    yak.fields.get(&field).cloned().unwrap_or_default()
                };
                if let Some(content) = input.edit_content(Some(&initial), None)? {
                    app.handle(WriteField::new(&name_str, &field).with_content(&content))
                } else {
                    Ok(())
                }
            } else if ConsoleInput::stdin_has_readable_data() {
                app.handle(WriteField::new(&name_str, &field))
            } else {
                // Default (no piped data, no --edit): show
                app.handle(ShowField::new(&name_str, &field))
            }
        }
        Commands::Reset {
            disk_from_git,
            git_from_disk,
        } => {
            // Validate flags
            if disk_from_git && git_from_disk {
                anyhow::bail!("Cannot use both --disk-from-git and --git-from-disk");
            }

            let root = repo_root
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("Error: not in a git repository"))?;

            if git_from_disk {
                // Wipe git history and replay through Application
                let yaks = storage.list_yaks()?;

                // Delete refs/notes/yaks to wipe git event history
                {
                    let repo = git2::Repository::open(root)?;
                    let delete_result = repo.find_reference("refs/notes/yaks");
                    if let Ok(mut reference) = delete_result {
                        reference.delete()?;
                    }
                }

                // Clear disk
                storage.clear()?;

                // Create fresh event infrastructure for replay
                let mut replay_store = GitEventStore::new(root)?;
                let mut replay_bus = EventBus::new();
                replay_bus.register(Box::new(storage.clone()));

                let replay_display = ConsoleDisplay::stdout();
                let replay_input = yx::adapters::user_input::NullInput;
                let replay_auth = GitAuthentication::new(root)?;
                let mut replay_app = Application::new(
                    &mut replay_store,
                    &mut replay_bus,
                    &storage,
                    &replay_display,
                    &replay_input,
                    None,
                    &replay_auth,
                );

                // Build index for topological traversal
                use std::collections::HashMap;
                let yak_index: HashMap<&yx::domain::slug::YakId, &yx::domain::Yak> =
                    yaks.iter().map(|y| (&y.id, y)).collect();

                // Find roots (yaks not appearing in any children list)
                let mut child_ids = std::collections::HashSet::new();
                for yak in &yaks {
                    for child_id in &yak.children {
                        child_ids.insert(child_id);
                    }
                }
                let roots: Vec<&yx::domain::Yak> =
                    yaks.iter().filter(|y| !child_ids.contains(&y.id)).collect();

                // Replay each yak through AddYak in topological order
                fn replay_yak(
                    app: &mut Application,
                    yak: &yx::domain::Yak,
                    yak_index: &HashMap<&yx::domain::slug::YakId, &yx::domain::Yak>,
                    parent_id: Option<&str>,
                ) -> Result<()> {
                    let has_real_metadata = yak.created_at != yx::domain::Timestamp::zero();
                    let mut use_case = AddYak::new(yak.name.as_str())
                        .with_id(Some(yak.id.as_str()))
                        .with_context(yak.context.as_deref())
                        .with_author(if has_real_metadata {
                            Some(yak.created_by.clone())
                        } else {
                            None
                        })
                        .with_timestamp(if has_real_metadata {
                            Some(yak.created_at)
                        } else {
                            None
                        });
                    if yak.state != "todo" {
                        use_case = use_case.with_state(Some(&yak.state));
                    }
                    if let Some(pid) = parent_id {
                        use_case = use_case.with_parent(Some(pid));
                    }
                    for (key, value) in &yak.fields {
                        use_case = use_case.with_field(key, value);
                    }
                    app.handle(use_case)?;

                    for child_id in &yak.children {
                        if let Some(child) = yak_index.get(child_id) {
                            replay_yak(app, child, yak_index, Some(yak.id.as_str()))?;
                        }
                    }
                    Ok(())
                }

                for root_yak in &roots {
                    replay_yak(&mut replay_app, root_yak, &yak_index, None)?;
                }

                // Stamp schema version on the new event history
                {
                    let repo = git2::Repository::open(root)?;
                    if repo.find_reference("refs/notes/yaks").is_ok() {
                        let location = yx::adapters::event_store::migration::EventStoreLocation {
                            repo: &repo,
                            ref_name: "refs/notes/yaks",
                        };
                        yx::adapters::event_store::migration::write_schema_version(
                            &location,
                            yx::adapters::event_store::migration::CURRENT_SCHEMA_VERSION,
                        )?;
                    }
                }

                println!("Reset from disk: {} yaks", yaks.len());
                println!();
                println!("To update the remote, run:");
                println!("  git push origin refs/notes/yaks --force");
                println!();
                println!("Collaborators must then run:");
                println!("  git fetch origin refs/notes/yaks:refs/notes/yaks --force");
            } else {
                // Default: rebuild .yaks directory from git tree
                let event_store = GitEventStore::new(root)?;
                let events = event_store.snapshot_events()?;
                let mut store = storage.clone();
                store.clear()?;
                for event in &events {
                    store.on_event(event)?;
                }
            }
            Ok(())
        }
        Commands::Compact { yes } => {
            // 1. Auto-sync first
            match app.sync_events() {
                Ok(()) => {}
                Err(e) => {
                    eprintln!("Warning: sync failed: {}", e);
                }
            }

            // 2. Confirmation prompt (unless --yes)
            if !yes {
                eprintln!(
                    "Warning: collaborators with unsynced local events \
                     will lose them. Ask them to run 'yx sync' first."
                );
                eprint!("Proceed? [y/N] ");
                let mut answer = String::new();
                std::io::BufRead::read_line(&mut std::io::stdin().lock(), &mut answer)?;
                if answer.trim().to_lowercase() != "y" {
                    return Ok(());
                }
            }

            // 3. Compact the event store
            app.handle(CompactEvents::new())?;

            // 4. Report success
            println!("Compacted event stream.");
            Ok(())
        }
        Commands::Sync => app.handle(SyncYaks::new()),
        Commands::Log => app.handle(ShowLog::new()),
        Commands::Completions { words } => {
            // Get yaks with state from storage
            let yaks = storage.list_yaks()?;

            // Build tuples of (name, is_done)
            let yak_name_strings: Vec<String> = yaks.iter().map(|y| y.name.to_string()).collect();
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
    fn add_joins_multiple_args_into_yak_name() {
        let cli = Cli::try_parse_from(["yx", "add", "this", "is", "a", "test"]).unwrap();
        match cli.command {
            Commands::Add { name, .. } => assert_eq!(name.join(" "), "this is a test"),
            other => panic!("Expected Add, got {:?}", other),
        }
    }

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
