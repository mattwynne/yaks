// Local workspace adapter - implements LocalWorkspacePort with real git/filesystem operations
//
// This adapter provides concrete implementations of workspace operations:
// - Checking .gitignore via git check-ignore
// - Modifying .gitignore file
// - Committing changes to git

use crate::domain::ports::LocalWorkspacePort;
use anyhow::Result;
use std::path::PathBuf;

/// Local workspace adapter backed by real git and filesystem operations
pub struct LocalWorkspace {
    repo_root: PathBuf,
}

impl LocalWorkspace {
    /// Create a new LocalWorkspace for the given repository root
    pub fn new(repo_root: PathBuf) -> Self {
        Self { repo_root }
    }
}

impl LocalWorkspacePort for LocalWorkspace {
    fn is_yaks_gitignored(&self) -> Result<bool> {
        crate::infrastructure::is_yaks_gitignored(&self.repo_root)
    }

    fn add_yaks_to_gitignore(&self) -> Result<()> {
        let gitignore_path = self.repo_root.join(".gitignore");
        let mut content = if gitignore_path.exists() {
            std::fs::read_to_string(&gitignore_path)?
        } else {
            String::new()
        };

        // Check if .yaks is already in .gitignore (defensive check)
        // Match both ".yaks" and ".yaks/" to avoid adding duplicates
        if content
            .lines()
            .any(|line| line.trim() == ".yaks" || line.trim() == ".yaks/")
        {
            return Ok(());
        }

        // Add .yaks to .gitignore
        if !content.is_empty() && !content.ends_with('\n') {
            content.push('\n');
        }
        content.push_str(".yaks\n");
        std::fs::write(&gitignore_path, content)?;

        Ok(())
    }

    fn is_agent_session(&self) -> bool {
        std::env::var("CLAUDECODE").as_deref() == Ok("1")
    }

    fn commit_gitignore(&self) -> Result<()> {
        // Stage .gitignore
        let add_status = std::process::Command::new("git")
            .args(["add", ".gitignore"])
            .current_dir(&self.repo_root)
            .status()?;

        if !add_status.success() {
            anyhow::bail!("Failed to stage .gitignore");
        }

        // Commit .gitignore
        let commit_status = std::process::Command::new("git")
            .args(["commit", "-m", "Add .yaks to .gitignore"])
            .current_dir(&self.repo_root)
            .status()?;

        if !commit_status.success() {
            anyhow::bail!("Failed to commit .gitignore");
        }

        Ok(())
    }
}
