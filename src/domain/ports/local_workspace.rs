// Port for local workspace operations (git and filesystem)
//
// This port abstracts git and filesystem operations needed for
// workspace management. Different adapters can implement this for:
// - Real git/filesystem operations
// - Test fixtures with mocked git
// - Dry-run mode for testing

use anyhow::Result;

/// Port for local workspace git and filesystem operations
pub trait LocalWorkspacePort {
    /// Check whether .yaks is in .gitignore
    ///
    /// Returns true if .yaks is already gitignored, false otherwise.
    fn is_yaks_gitignored(&self) -> Result<bool>;

    /// Add .yaks to .gitignore (create the file if needed)
    ///
    /// Appends ".yaks" to .gitignore, creating the file if it doesn't exist.
    /// If .yaks is already in .gitignore, this is a no-op.
    fn add_yaks_to_gitignore(&self) -> Result<()>;

    /// Commit .gitignore with a standard message
    ///
    /// Stages and commits .gitignore with the message "Add .yaks to .gitignore".
    /// Fails if the git commit fails.
    fn commit_gitignore(&self) -> Result<()>;
}
