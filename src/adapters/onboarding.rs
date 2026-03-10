// Onboarding adapter - handles first-time setup of yaks in a repository
//
// This adapter encapsulates the onboarding flow for new users:
// - Checking if .yaks is gitignored
// - Detecting interactive mode (TTY or YX_FORCE_INTERACTIVE)
// - Prompting to add .yaks to .gitignore
// - Offering to commit the change
//
// Per ADR 0008, this keeps main.rs thin by moving the onboarding
// logic out of main() and into a proper adapter.

use anyhow::Result;
use std::io::{self, IsTerminal, Write};
use std::path::Path;

/// Ensure .yaks is gitignored, prompting interactively if needed.
///
/// This is the main entry point for onboarding. It:
/// 1. Checks if .yaks is already gitignored
/// 2. If not, and we're in interactive mode, prompts to add it
/// 3. If not interactive, returns an error
///
/// Returns Ok(()) if .yaks is gitignored (or was successfully added).
/// Returns Err if .yaks is not gitignored and user declined or non-interactive.
pub fn ensure_yaks_gitignored(repo_root: &Path) -> Result<()> {
    // Check if .yaks is already gitignored
    if crate::infrastructure::is_yaks_gitignored(repo_root)? {
        return Ok(());
    }

    // Not gitignored - check if we can prompt
    if !is_interactive_mode() {
        anyhow::bail!("Error: .yaks folder is not gitignored");
    }

    // Interactive mode: offer to add .yaks to .gitignore
    if !prompt_add_yaks_to_gitignore(repo_root)? {
        // User declined
        anyhow::bail!("Error: .yaks folder is not gitignored");
    }

    // Success - user accepted and .yaks was added
    Ok(())
}

/// Prompt user to add .yaks to .gitignore, and optionally commit it.
///
/// Returns true if .yaks was added to .gitignore (regardless of commit decision).
/// Returns false if the user declined to add .yaks.
fn prompt_add_yaks_to_gitignore(repo_root: &Path) -> Result<bool> {
    // Welcome message for first-time users
    eprintln!();
    eprintln!("👋 It looks like you've never used yaks in this repo before!");
    eprintln!("   I need to add .yaks to your .gitignore to keep things tidy.");
    eprintln!();
    eprint!("   Add .yaks to .gitignore? [Y/n] ");
    io::stderr().flush()?;

    let mut response = String::new();
    io::stdin().read_line(&mut response)?;
    let add_to_gitignore = response.trim().is_empty() || response.trim().eq_ignore_ascii_case("y");

    if !add_to_gitignore {
        return Ok(false);
    }

    // Add .yaks to .gitignore
    let gitignore_path = repo_root.join(".gitignore");
    let mut content = if gitignore_path.exists() {
        std::fs::read_to_string(&gitignore_path)?
    } else {
        String::new()
    };

    // Check if .yaks is already in .gitignore (shouldn't happen, but be safe)
    if !content.lines().any(|line| line.trim() == ".yaks") {
        if !content.is_empty() && !content.ends_with('\n') {
            content.push('\n');
        }
        content.push_str(".yaks\n");
        std::fs::write(&gitignore_path, content)?;
    }

    eprintln!("   ✅ Added .yaks to .gitignore");
    eprintln!();
    eprint!("   Commit this change now? [Y/n] ");
    io::stderr().flush()?;

    let mut commit_response = String::new();
    io::stdin().read_line(&mut commit_response)?;
    let should_commit =
        commit_response.trim().is_empty() || commit_response.trim().eq_ignore_ascii_case("y");

    if should_commit {
        // Stage and commit .gitignore
        let add_status = std::process::Command::new("git")
            .args(["add", ".gitignore"])
            .current_dir(repo_root)
            .status()?;

        if !add_status.success() {
            anyhow::bail!("Failed to stage .gitignore");
        }

        let commit_status = std::process::Command::new("git")
            .args(["commit", "-m", "Add .yaks to .gitignore"])
            .current_dir(repo_root)
            .status()?;

        if !commit_status.success() {
            anyhow::bail!("Failed to commit .gitignore");
        }

        eprintln!("   ✅ Committed!");
    } else {
        eprintln!("   Please remember to commit .gitignore");
    }

    Ok(true)
}

/// Check if we're in interactive mode for prompting.
///
/// Returns true if:
/// - stdout is a TTY, OR
/// - YX_FORCE_INTERACTIVE=1 is set (for testing)
fn is_interactive_mode() -> bool {
    std::io::stdout().is_terminal() || std::env::var("YX_FORCE_INTERACTIVE").as_deref() == Ok("1")
}
