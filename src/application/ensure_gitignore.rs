// EnsureGitignore use case - handles first-time .yaks setup
//
// This use case ensures .yaks is gitignored, prompting interactively if needed.
// It orchestrates the onboarding flow through the LocalWorkspacePort.

use crate::application::{Application, UseCase};
use anyhow::Result;
use std::io::{self, Write};

/// Ensure .yaks is gitignored use case
///
/// This use case:
/// 1. Checks if .yaks is already gitignored
/// 2. If not, prompts interactively to add it
/// 3. Optionally commits the change
///
/// Returns Ok(()) if .yaks is gitignored (or was successfully added).
/// Returns Err if .yaks is not gitignored and cannot be added.
pub struct EnsureGitignore;

impl EnsureGitignore {
    pub fn new() -> Self {
        Self
    }

    /// Prompt user to add .yaks to .gitignore with [Y/n] default
    fn prompt_add_gitignore() -> Result<bool> {
        eprintln!();
        eprintln!("👋 It looks like you've never used yaks in this repo before!");
        eprintln!("   I need to add .yaks to your .gitignore to keep things tidy.");
        eprintln!();
        eprint!("   Add .yaks to .gitignore? [Y/n] ");
        io::stderr().flush()?;

        let mut response = String::new();
        io::stdin().read_line(&mut response)?;
        let add_to_gitignore =
            response.trim().is_empty() || response.trim().eq_ignore_ascii_case("y");

        Ok(add_to_gitignore)
    }

    /// Prompt user to commit .gitignore with [Y/n] default
    fn prompt_commit() -> Result<bool> {
        eprintln!("   ✅ Added .yaks to .gitignore");
        eprintln!();
        eprint!("   Commit this change now? [Y/n] ");
        io::stderr().flush()?;

        let mut response = String::new();
        io::stdin().read_line(&mut response)?;
        let should_commit = response.trim().is_empty() || response.trim().eq_ignore_ascii_case("y");

        Ok(should_commit)
    }
}

impl Default for EnsureGitignore {
    fn default() -> Self {
        Self::new()
    }
}

impl UseCase for EnsureGitignore {
    fn execute(&self, app: &mut Application) -> Result<()> {
        // Check if .yaks is already gitignored
        if app.local_workspace.is_yaks_gitignored()? {
            return Ok(());
        }

        // Not gitignored - check if we can prompt
        if !app.input.is_interactive() {
            anyhow::bail!("Error: .yaks folder is not gitignored");
        }

        // Interactive mode: offer to add .yaks to .gitignore
        if !Self::prompt_add_gitignore()? {
            // User declined
            anyhow::bail!("Error: .yaks folder is not gitignored");
        }

        // Add .yaks to .gitignore
        app.local_workspace.add_yaks_to_gitignore()?;

        // Ask about committing
        if Self::prompt_commit()? {
            app.local_workspace.commit_gitignore()?;
            eprintln!("   ✅ Committed!");
        } else {
            eprintln!("   Please remember to commit .gitignore");
        }

        Ok(())
    }
}
