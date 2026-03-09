// Use case: Set sync target URL

use crate::adapters::views::Message;
use anyhow::Result;

use super::{Application, UseCase};

pub struct SetSyncTarget {
    url: String,
}

impl SetSyncTarget {
    pub fn new(url: String) -> Self {
        Self { url }
    }
}

impl UseCase for SetSyncTarget {
    fn execute(&self, app: &mut Application) -> Result<()> {
        // Get the repository path
        let repo_path = app
            .event_store
            .repo_path()
            .ok_or_else(|| anyhow::anyhow!("Cannot set sync target: not in a git repository"))?;

        // Verify the URL is reachable using git ls-remote
        let ls_remote_output = std::process::Command::new("git")
            .args(["ls-remote", "--heads", &self.url])
            .current_dir(&repo_path)
            .output()?;

        if !ls_remote_output.status.success() {
            let stderr = String::from_utf8_lossy(&ls_remote_output.stderr);
            anyhow::bail!("Failed to connect to {}: {}", self.url, stderr.trim());
        }

        // Store the URL in git config yaks.remote
        let config_output = std::process::Command::new("git")
            .args(["config", "yaks.remote", &self.url])
            .current_dir(&repo_path)
            .output()?;

        if !config_output.status.success() {
            let stderr = String::from_utf8_lossy(&config_output.stderr);
            anyhow::bail!("Failed to set yaks.remote config: {}", stderr.trim());
        }

        app.display
            .message(&Message::Info(format!("Connected to {}", self.url)));

        Ok(())
    }
}
