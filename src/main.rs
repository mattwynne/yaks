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
    AddTag, AddYak, Application, CommandHandler, CompactEvents, DoneYak, EditContext, EditField,
    GenerateCompletions, ListTags, ListYaks, MoveYak, PruneYaks, RemoveTag, RemoveYak, RenameYak,
    ResetDiskFromGit, ResetGitFromDisk, SetState, ShowContext, ShowField, ShowLog, ShowYak,
    StartYak, SyncYaks, WriteContext, WriteField,
};
use yx::domain::normalize_tag;
use yx::domain::ports::EventStore;
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
            help = "Output format: pretty (default), markdown/md, plain/raw, json",
            long_help = "Output format:\n  - pretty: Unicode box-drawing with colored status dots (default)\n  - markdown (or md): Checkbox-style list with indentation\n  - plain (or raw): Just yak names, one per line\n  - json: Full yak tree as JSON array (for agents/scripts)"
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
        /// Current yak name (quote multi-word names)
        from: String,
        /// New name (quote multi-word names)
        to: String,
    },
    /// Show yak details
    Show {
        /// The yak name (space-separated words)
        name: Vec<String>,
        #[arg(
            long,
            default_value = "pretty",
            help = "Output format: pretty (default), json",
            long_help = "Output format:\n  - pretty: Human-readable display with tree structure\n  - json: Full yak details as JSON (for agents/scripts)"
        )]
        format: String,
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
        /// Skip confirmation prompt
        #[arg(long)]
        force: bool,
    },
    /// Manage tags on a yak
    #[command(alias = "tags")]
    Tag {
        #[command(subcommand)]
        action: TagAction,
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

#[derive(Parser, Debug)]
enum TagAction {
    /// Add tags to a yak
    Add {
        /// The yak name
        name: String,
        /// Tags to add
        #[arg(required = true)]
        tags: Vec<String>,
    },
    /// Remove tags from a yak
    #[command(alias = "remove")]
    Rm {
        /// The yak name
        name: String,
        /// Tags to remove
        #[arg(required = true)]
        tags: Vec<String>,
    },
    /// List tags on a yak
    List {
        /// The yak name
        name: String,
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

/// Pre-computed stdin state, determined once in main() before routing.
///
/// This lets route_command make stdin-dependent decisions without
/// touching any adapter types.
struct StdinState {
    /// True when stdin is connected to a pipe or file (even if empty).
    /// Used to distinguish "no stdin" (show) from "empty stdin" (no-op).
    is_piped: bool,
    /// Pre-read stdin content (consumed once). Available for commands
    /// that need it as initial editor content (e.g. context --edit
    /// with piped stdin).
    content: Option<String>,
}

/// Route a CLI command to its use case via CommandHandler.
///
/// This function physically cannot access Application internals,
/// adapter types, or domain types — it only sees `CommandHandler`
/// with a single `handle()` method. The compiler enforces the
/// architectural boundary from ADR 0013.
fn route_context(
    handler: &mut impl CommandHandler,
    name: &str,
    show: bool,
    edit: bool,
    stdin: &StdinState,
) -> Result<()> {
    if show && edit {
        anyhow::bail!("Cannot use both --show and --edit");
    }
    if show && stdin.content.is_some() {
        anyhow::bail!("Cannot use --show when piping input (stdin would be ignored)");
    }
    if show {
        return handler.handle(ShowContext::new(name));
    }
    if edit {
        let mut use_case = EditContext::new(name);
        if let Some(ref content) = stdin.content {
            use_case = use_case.with_initial_content(content);
        }
        handler.handle(use_case)
    } else if let Some(ref content) = stdin.content {
        handler.handle(WriteContext::new(name, content))
    } else if stdin.is_piped {
        Ok(()) // Piped but empty — no-op
    } else {
        handler.handle(ShowContext::new(name))
    }
}

fn route_field(
    handler: &mut impl CommandHandler,
    name: &str,
    field: &str,
    show: bool,
    edit: bool,
    stdin: &StdinState,
) -> Result<()> {
    if show && edit {
        anyhow::bail!("Cannot use both --show and --edit");
    }
    if show && stdin.content.is_some() {
        anyhow::bail!("Cannot use --show when piping input (stdin would be ignored)");
    }
    if show {
        return handler.handle(ShowField::new(name, field));
    }
    if edit {
        let mut use_case = EditField::new(name, field);
        if let Some(ref content) = stdin.content {
            use_case = use_case.with_initial_content(content);
        }
        handler.handle(use_case)
    } else if let Some(ref content) = stdin.content {
        handler.handle(WriteField::new(name, field).with_content(content))
    } else if stdin.is_piped {
        Ok(()) // Piped but empty — no-op
    } else {
        handler.handle(ShowField::new(name, field))
    }
}

fn route_command(
    cmd: Commands,
    handler: &mut impl CommandHandler,
    stdin: StdinState,
) -> Result<()> {
    match cmd {
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
            let has_explicit_context = context.is_some();
            // Resolve context: --context flag > --edit (editor) > piped stdin
            let resolved_context = if has_explicit_context {
                context
            } else if edit {
                // Editor mode — pass no context, let the use case open editor
                None
            } else {
                // Try stdin content
                stdin.content.clone().filter(|c| !c.trim().is_empty())
            };
            let mut use_case = AddYak::new(&name_str)
                .with_parent(under.as_deref())
                .with_state(state.as_deref())
                .with_context(resolved_context.as_deref())
                .with_id(id.as_deref())
                .with_edit(edit && !has_explicit_context);
            for (key, value) in &fields {
                use_case = use_case.with_field(key, value);
            }
            handler.handle(use_case)
        }
        Commands::List { format, only } => handler.handle(ListYaks::new(&format, only.as_deref())),
        Commands::Done { name, recursive } => {
            let name_str = name.join(" ");
            handler.handle(DoneYak::new(&name_str, recursive))
        }
        Commands::Start { name, recursive } => {
            let name_str = name.join(" ");
            handler.handle(StartYak::new(&name_str, recursive))
        }
        Commands::Remove { name, recursive } => {
            let name_str = name.join(" ");
            handler.handle(RemoveYak::new(&name_str).with_recursive(recursive))
        }
        Commands::Prune => handler.handle(PruneYaks::new()),
        Commands::Move {
            name,
            under,
            to_root,
        } => {
            let name_str = name.join(" ");
            if to_root {
                handler.handle(MoveYak::to_root(&name_str))
            } else {
                let parent_str = under.unwrap().join(" ");
                handler.handle(MoveYak::under(&name_str, &parent_str))
            }
        }
        Commands::Rename { from, to } => handler.handle(RenameYak::new(&from, &to)),
        Commands::Show { name, format } => {
            let name_str = name.join(" ");
            handler.handle(ShowYak::new(&name_str, &format))
        }
        Commands::Context { name, show, edit } => {
            route_context(handler, &name.join(" "), show, edit, &stdin)
        }
        Commands::State {
            name,
            state,
            recursive,
        } => {
            let name_str = name.join(" ");
            handler.handle(SetState::new(&name_str, &state).with_recursive(recursive))
        }
        Commands::Field {
            name,
            field,
            show,
            edit,
        } => route_field(handler, &name.join(" "), &field, show, edit, &stdin),
        Commands::Reset {
            disk_from_git,
            git_from_disk,
            force,
        } => {
            if disk_from_git && git_from_disk {
                anyhow::bail!("Cannot use both --disk-from-git and --git-from-disk");
            }

            if git_from_disk {
                handler.handle(ResetGitFromDisk::new().with_force(force))
            } else {
                handler.handle(ResetDiskFromGit::new())
            }
        }
        Commands::Tag { action } => match action {
            TagAction::Add { name, tags } => {
                let normalized: Vec<String> = tags
                    .iter()
                    .map(|t| normalize_tag(t))
                    .collect::<Result<_>>()?;
                handler.handle(AddTag::new(&name, normalized))
            }
            TagAction::Rm { name, tags } => {
                let normalized: Vec<String> = tags
                    .iter()
                    .map(|t| normalize_tag(t))
                    .collect::<Result<_>>()?;
                handler.handle(RemoveTag::new(&name, normalized))
            }
            TagAction::List { name } => handler.handle(ListTags::new(&name)),
        },
        Commands::Compact { yes } => handler.handle(CompactEvents::new().with_skip_confirm(yes)),
        Commands::Sync => handler.handle(SyncYaks::new()),
        Commands::Log => handler.handle(ShowLog::new()),
        Commands::Completions { words } => handler.handle(GenerateCompletions::new(words)),
    }
}

#[allow(clippy::cognitive_complexity)]
fn main() -> Result<()> {
    // Show help on stderr when run with no arguments
    let args: Vec<_> = std::env::args().collect();
    if args.len() == 1 {
        let mut cmd = Cli::command();
        let help = cmd.render_help();
        eprintln!("{help}");
        std::process::exit(2);
    }

    let cli = Cli::parse();

    let skip_git = std::env::var("YX_SKIP_GIT_CHECKS").is_ok();

    // Pre-compute stdin state before any adapter construction.
    // This is the only place main() touches an adapter directly — it
    // passes the result into route_command so routing stays pure.
    let is_piped = ConsoleInput::stdin_is_piped();
    let stdin_content = if is_piped && ConsoleInput::stdin_has_readable_data() {
        let input = ConsoleInput;
        input.read_stdin_content().ok().flatten()
    } else {
        None
    };
    let stdin = StdinState {
        is_piped,
        content: stdin_content,
    };

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

    // Route command through CommandHandler trait — main() cannot
    // accidentally bypass use cases because route_command only
    // sees `&mut impl CommandHandler`.
    route_command(cli.command, &mut app, stdin)
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
