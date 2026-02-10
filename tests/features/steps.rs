use anyhow::{Context, Result};
use cucumber::{given, then, when};
use std::process::Command;

use super::world::{strip_ansi_codes, ExecutionMode, World};
use yx::cli::CommandHandler;

#[given(expr = "I have a clean git repository")]
async fn clean_git_repo(world: &mut World) -> Result<()> {
    // Only needed in full-stack mode
    match world.mode {
        ExecutionMode::FullStack => {
            // Initialize git repository
            let status = Command::new("git")
                .arg("init")
                .current_dir(&world.repo_path)
                .status()
                .context("Failed to run git init")?;

            if !status.success() {
                anyhow::bail!("git init failed");
            }

            // Configure git user for the test repo
            Command::new("git")
                .args(["config", "user.email", "test@example.com"])
                .current_dir(&world.repo_path)
                .status()
                .context("Failed to set git user.email")?;

            Command::new("git")
                .args(["config", "user.name", "Test User"])
                .current_dir(&world.repo_path)
                .status()
                .context("Failed to set git user.name")?;
        }
        ExecutionMode::InProcess => {
            // No git needed in in-process mode
        }
    }

    Ok(())
}

#[given(regex = r#"I have added the yak "(.+)""#)]
async fn add_yak(world: &mut World, yak_name: String) -> Result<()> {
    match world.mode {
        ExecutionMode::FullStack => {
            // Use the yx binary to add a yak
            let yx_path = env!("CARGO_BIN_EXE_yx");

            let output = Command::new(yx_path)
                .arg("add")
                .arg(&yak_name)
                .env("YAK_PATH", &world.repo_path)
                .env("YX_IGNORE_STDIN", "1") // Skip interactive editor
                .env("YX_SKIP_GIT_CHECKS", "1") // Skip git logging
                .current_dir(&world.repo_path)
                .output()
                .context("Failed to run yx add")?;

            if !output.status.success() {
                anyhow::bail!(
                    "yx add failed:\nstdout: {}\nstderr: {}",
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                );
            }
        }
        ExecutionMode::InProcess => {
            // Use CommandHandler with in-memory adapters
            let handler = CommandHandler::new(&world.storage, &world.output_adapter, &world.log);
            handler.handle_add(&yak_name)?;
        }
    }

    Ok(())
}

#[when(regex = r#"I run "(.+)""#)]
async fn run_command(world: &mut World, command: String) -> Result<()> {
    // Parse the command string
    let parts: Vec<&str> = command.split_whitespace().collect();

    if parts.is_empty() {
        anyhow::bail!("Empty command");
    }

    // For yx commands
    if parts[0] == "yx" {
        match world.mode {
            ExecutionMode::FullStack => {
                let yx_path = env!("CARGO_BIN_EXE_yx");

                let output = Command::new(yx_path)
                    .args(&parts[1..])
                    .env("YAK_PATH", &world.repo_path)
                    .env("YX_SKIP_GIT_CHECKS", "1") // Skip git logging
                    .current_dir(&world.repo_path)
                    .output()
                    .context("Failed to run yx command")?;

                world.exit_code = output.status.code().unwrap_or(-1);
                world.output = String::from_utf8_lossy(&output.stdout).to_string();
            }
            ExecutionMode::InProcess => {
                // Clear previous output
                world.output_adapter.clear();

                // Create handler and route command
                let handler =
                    CommandHandler::new(&world.storage, &world.output_adapter, &world.log);

                let result = route_command(&handler, &parts[1..]);

                // Set exit code based on result
                world.exit_code = if result.is_ok() { 0 } else { 1 };

                // Capture output from in-memory adapter
                let mut output_lines = Vec::new();
                output_lines.extend(world.output_adapter.get_success_messages());
                output_lines.extend(world.output_adapter.get_error_messages());
                output_lines.extend(world.output_adapter.get_info_messages());
                world.output = output_lines.join("\n");

                // Propagate errors
                result?;
            }
        }

        Ok(())
    } else {
        anyhow::bail!("Unsupported command: {}", parts[0])
    }
}

// Helper function to route commands to CommandHandler methods
fn route_command(handler: &CommandHandler, args: &[&str]) -> Result<()> {
    if args.is_empty() {
        anyhow::bail!("No command specified");
    }

    match args[0] {
        "add" => {
            if args.len() < 2 {
                anyhow::bail!("add requires a yak name");
            }
            let name = args[1..].join(" ");
            handler.handle_add(&name)
        }
        "ls" | "list" => {
            // Default format is "pretty" to match binary behavior
            let format = if args.len() > 1 && args[1] == "--format" && args.len() > 2 {
                args[2]
            } else {
                "pretty"
            };
            let only = None;
            handler.handle_list(format, only)
        }
        "done" => {
            if args.len() < 2 {
                anyhow::bail!("done requires a yak name");
            }
            let name = args[1..].join(" ");
            handler.handle_done(&name, false, false)
        }
        "rm" | "remove" => {
            if args.len() < 2 {
                anyhow::bail!("remove requires a yak name");
            }
            let name = args[1..].join(" ");
            handler.handle_remove(&name)
        }
        "prune" => handler.handle_prune(),
        "move" | "mv" => {
            if args.len() < 3 {
                anyhow::bail!("move requires from and to names");
            }
            let from = args[1];
            let to = args[2];
            handler.handle_move(from, to)
        }
        "state" => {
            if args.len() < 3 {
                anyhow::bail!("state requires a yak name and state");
            }
            let name = args[1];
            let state = args[2];
            handler.handle_state(name, state)
        }
        "context" => {
            if args.len() < 2 {
                anyhow::bail!("context requires a yak name");
            }
            // Check for --show flag
            if args.len() >= 3 && args[1] == "--show" {
                let name = args[2..].join(" ");
                handler.handle_context_show(&name)
            } else {
                let name = args[1..].join(" ");
                handler.handle_context_edit(&name)
            }
        }
        "field" => {
            if args.len() < 3 {
                anyhow::bail!("field requires a yak name and field name");
            }
            let name = args[1];
            let field = args[2];
            // Check for --show flag or default to show
            if args.len() >= 4 && args[3] == "--show" {
                handler.handle_field_show(name, field)
            } else {
                handler.handle_field_show(name, field)
            }
        }
        _ => anyhow::bail!("Unknown command: {}", args[0]),
    }
}

#[then(expr = "the output should be:")]
async fn output_should_be(world: &mut World, step: &cucumber::gherkin::Step) -> Result<()> {
    // Get the docstring from the step
    let expected = step
        .docstring
        .as_ref()
        .context("Expected docstring in step")?;

    let expected_text = expected.trim();
    let actual = world.output.trim();

    // Strip ANSI color codes from actual output for comparison
    let actual_no_ansi = strip_ansi_codes(actual);

    if actual_no_ansi != expected_text {
        anyhow::bail!(
            "\nExpected:\n{}\n\nActual:\n{}",
            expected_text,
            actual_no_ansi
        );
    }

    Ok(())
}
