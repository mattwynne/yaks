use anyhow::Result;
use clap::{Parser, Subcommand};
use std::env;
use std::path::{Path, PathBuf};
use yx::adapters::{FilesystemStorage, GitAdapter, TerminalFormatter};
use yx::ports::{OutputFormat, YakFilter};
use yx::YakApp;

#[derive(Parser)]
#[command(name = "yx")]
#[command(about = "A CLI tool for managing TODO lists as a DAG", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    Add {
        name: Vec<String>,
    },
    #[command(alias = "ls")]
    List {
        #[arg(long)]
        format: Option<String>,
        #[arg(long)]
        only: Option<String>,
    },
    Done {
        #[arg(long)]
        undo: bool,
        #[arg(long)]
        recursive: bool,
        name: Vec<String>,
    },
    Rm {
        name: Vec<String>,
    },
    Prune,
    #[command(alias = "mv")]
    Move {
        old_name: String,
        new_name: Vec<String>,
    },
    Context {
        #[arg(long)]
        show: bool,
        #[arg(long)]
        edit: bool,
        name: Vec<String>,
    },
    Sync,
    Completions {
        cmd: Option<String>,
        #[arg(allow_hyphen_values = true)]
        flag: Option<String>,
    },
}

fn get_work_tree() -> PathBuf {
    env::var("GIT_WORK_TREE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| env::current_dir().expect("Failed to get current directory"))
}

fn get_yaks_path(work_tree: &Path) -> PathBuf {
    work_tree.join(".yaks")
}

fn parse_format(format_str: Option<String>) -> OutputFormat {
    match format_str.as_deref() {
        Some("plain") | Some("raw") => OutputFormat::Plain,
        _ => OutputFormat::Markdown,
    }
}

fn parse_filter(filter_str: Option<String>) -> YakFilter {
    match filter_str.as_deref() {
        Some("not-done") => YakFilter::NotDone,
        Some("done") => YakFilter::Done,
        _ => YakFilter::All,
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    if cli.command.is_none() {
        println!(
            "Usage: yx <command> [arguments]

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
  --help                          Show this help message"
        );
        return Ok(());
    }

    let work_tree = get_work_tree();
    let yaks_path = get_yaks_path(&work_tree);

    let storage = FilesystemStorage::new(yaks_path.clone());
    let git = GitAdapter::new(work_tree.clone(), yaks_path);
    let formatter = TerminalFormatter::new();

    let mut app = YakApp::new(storage, git, formatter);

    app.check_preconditions()?;

    match cli.command.unwrap() {
        Commands::Add { name } => {
            let name_str = if name.is_empty() {
                None
            } else {
                Some(name.join(" "))
            };
            app.add(name_str)?;
        }
        Commands::List { format, only } => {
            let fmt = parse_format(format);
            let filter = parse_filter(only);
            println!("{}", app.list(fmt, filter)?);
        }
        Commands::Done { undo, recursive, name } => {
            let name_str = name.join(" ");
            app.done(&name_str, undo, recursive)?;
        }
        Commands::Rm { name } => {
            let name_str = name.join(" ");
            app.remove(&name_str)?;
        }
        Commands::Prune => {
            app.prune()?;
        }
        Commands::Move { old_name, new_name } => {
            let new_name_str = new_name.join(" ");
            app.rename(&old_name, &new_name_str)?;
        }
        Commands::Context { show, edit: _, name } => {
            let name_str = name.join(" ");
            if show {
                println!("{}", app.show_context(&name_str)?);
            } else {
                app.edit_context(&name_str, None)?;
            }
        }
        Commands::Sync => {
            app.sync()?;
        }
        Commands::Completions { cmd, flag } => {
            // Skip preconditions for completions to allow tab-complete anywhere
            let work_tree = get_work_tree();
            let yaks_path = get_yaks_path(&work_tree);
            let storage = FilesystemStorage::new(yaks_path.clone());
            let git = GitAdapter::new(work_tree.clone(), yaks_path);
            let formatter = TerminalFormatter::new();
            let app = YakApp::new(storage, git, formatter);

            let completions = app.completions(cmd.as_deref(), flag.as_deref())?;
            for completion in completions {
                println!("{completion}");
            }
            return Ok(());
        }
    }

    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
