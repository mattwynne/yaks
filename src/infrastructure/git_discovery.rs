// Git repository discovery using libgit2
//
// Provides a single function to discover the git repo root from cwd,
// replacing scattered shell-outs to `git rev-parse` and env var lookups.

use anyhow::Result;
use std::path::PathBuf;
use std::process::Command;

/// Discover the git repository root from the current working directory.
///
/// Uses `git2::Repository::discover(".")` which walks up the directory
/// tree looking for a `.git` directory, exactly like `git rev-parse
/// --show-toplevel`.
///
/// Returns the working directory (workdir) of the repository.
/// Errors if not inside a git repo or the repo is bare.
pub fn discover_git_root() -> Result<PathBuf> {
    let repo = git2::Repository::discover(".")
        .map_err(|_| anyhow::anyhow!("Error: not in a git repository"))?;

    let workdir = repo
        .workdir()
        .ok_or_else(|| anyhow::anyhow!("Error: not in a git working tree"))?;

    Ok(workdir.to_path_buf())
}

/// Check whether `.yaks` is gitignored, returning a boolean.
///
/// Returns true if .yaks is gitignored, false otherwise.
/// Errors only on command execution failures.
pub fn is_yaks_gitignored(repo_root: &std::path::Path) -> Result<bool> {
    let output = match Command::new("git")
        .arg("check-ignore")
        .arg(".yaks")
        .current_dir(repo_root)
        .output()
    {
        Ok(output) => output,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            anyhow::bail!("Error: git command not found");
        }
        Err(e) => {
            return Err(anyhow::Error::new(e).context("Failed to check .yaks gitignore status"));
        }
    };

    Ok(output.status.success())
}
