// Use case: Show current sync remote

use crate::adapters::views::Message;
use anyhow::Result;

use super::{Application, UseCase};

pub struct ShowSyncRemote;

impl ShowSyncRemote {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ShowSyncRemote {
    fn default() -> Self {
        Self::new()
    }
}

impl UseCase for ShowSyncRemote {
    fn execute(&self, app: &mut Application) -> Result<()> {
        // Get the repository path
        let repo_path = app
            .event_store
            .repo_path()
            .ok_or_else(|| anyhow::anyhow!("Cannot show sync remote: not in a git repository"))?;

        // Try to read git config yaks.remote
        let config_output = std::process::Command::new("git")
            .args(["config", "--get", "yaks.remote"])
            .current_dir(&repo_path)
            .output()?;

        if config_output.status.success() {
            // yaks.remote is configured
            let url = String::from_utf8_lossy(&config_output.stdout)
                .trim()
                .to_string();
            app.display.message(&Message::Info(url));
        } else {
            // Fall back to origin remote
            let origin_output = std::process::Command::new("git")
                .args(["config", "--get", "remote.origin.url"])
                .current_dir(&repo_path)
                .output()?;

            if origin_output.status.success() {
                let url = String::from_utf8_lossy(&origin_output.stdout)
                    .trim()
                    .to_string();
                app.display
                    .message(&Message::Info(format!("{} (origin)", url)));
            } else {
                // No origin either
                app.display
                    .message(&Message::Info("Not connected".to_string()));
            }
        }

        Ok(())
    }
}
