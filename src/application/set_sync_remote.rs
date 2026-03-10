// Use case: Set sync remote URL

use crate::adapters::views::Message;
use anyhow::Result;

use super::{Application, UseCase};

pub struct SetSyncRemote {
    url: String,
}

impl SetSyncRemote {
    pub fn new(url: String) -> Self {
        Self { url }
    }
}

impl UseCase for SetSyncRemote {
    fn execute(&self, app: &mut Application) -> Result<()> {
        // Get the repository path
        let repo_path = app
            .event_store
            .repo_path()
            .ok_or_else(|| anyhow::anyhow!("Cannot set sync remote: not in a git repository"))?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::event_store::git::GitEventStore;
    use crate::adapters::{
        make_test_display, InMemoryAuthentication, InMemoryInput, InMemoryStorage,
    };
    use crate::infrastructure::EventBus;

    #[test]
    fn fails_when_remote_is_unreachable() {
        let temp_dir = tempfile::tempdir().unwrap();
        let repo_path = temp_dir.path();

        // Initialize a git repository
        std::process::Command::new("git")
            .args(["init"])
            .current_dir(repo_path)
            .output()
            .unwrap();

        let mut event_store = GitEventStore::new(repo_path).unwrap();
        let mut event_bus = EventBus::new();
        let storage = InMemoryStorage::new();
        let (display, _) = make_test_display();
        let input = InMemoryInput::new();
        let auth = InMemoryAuthentication::new();

        let mut app = Application::new(
            &mut event_store,
            &mut event_bus,
            &storage,
            &display,
            &input,
            None,
            &auth,
        );

        // Try to set sync remote to a non-existent URL
        let use_case = SetSyncRemote::new(
            "https://invalid-url-that-does-not-exist.example/repo.git".to_string(),
        );
        let result = use_case.execute(&mut app);

        assert!(result.is_err(), "Should fail when remote is unreachable");
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Failed to connect to"),
            "Error should mention connection failure, got: {}",
            err_msg
        );
    }

    #[test]
    fn fails_when_not_in_git_repo() {
        use crate::adapters::event_store::memory::InMemoryEventStore;

        let mut event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new();
        let storage = InMemoryStorage::new();
        let (display, _) = make_test_display();
        let input = InMemoryInput::new();
        let auth = InMemoryAuthentication::new();

        let mut app = Application::new(
            &mut event_store,
            &mut event_bus,
            &storage,
            &display,
            &input,
            None,
            &auth,
        );

        let use_case = SetSyncRemote::new("https://example.com/repo.git".to_string());
        let result = use_case.execute(&mut app);

        assert!(result.is_err(), "Should fail when not in a git repository");
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("not in a git repository"),
            "Error should mention git repository requirement, got: {}",
            err_msg
        );
    }

    #[test]
    #[cfg(unix)]
    fn fails_when_git_config_fails() {
        use std::fs;
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = tempfile::tempdir().unwrap();
        let repo_path = temp_dir.path();

        // Initialize a git repository
        std::process::Command::new("git")
            .args(["init"])
            .current_dir(repo_path)
            .output()
            .unwrap();

        // Create a valid remote to pass the ls-remote check
        // We'll use a local path as a remote
        let remote_dir = tempfile::tempdir().unwrap();
        std::process::Command::new("git")
            .args(["init", "--bare"])
            .current_dir(remote_dir.path())
            .output()
            .unwrap();

        // Make the .git directory read-only to cause git config to fail
        let git_dir = repo_path.join(".git");
        let mut perms = fs::metadata(&git_dir).unwrap().permissions();
        perms.set_mode(0o555); // Read-only
        fs::set_permissions(&git_dir, perms).unwrap();

        let mut event_store = GitEventStore::new(repo_path).unwrap();
        let mut event_bus = EventBus::new();
        let storage = InMemoryStorage::new();
        let (display, _) = make_test_display();
        let input = InMemoryInput::new();
        let auth = InMemoryAuthentication::new();

        let mut app = Application::new(
            &mut event_store,
            &mut event_bus,
            &storage,
            &display,
            &input,
            None,
            &auth,
        );

        // Try to set sync remote - should fail when trying to write config
        let use_case = SetSyncRemote::new(remote_dir.path().to_str().unwrap().to_string());
        let result = use_case.execute(&mut app);

        // Restore permissions before assertions to ensure cleanup
        let mut perms = fs::metadata(&git_dir).unwrap().permissions();
        perms.set_mode(0o755);
        let _ = fs::set_permissions(&git_dir, perms);

        assert!(result.is_err(), "Should fail when git config command fails");
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Failed to set yaks.remote config"),
            "Error should mention config failure, got: {}",
            err_msg
        );
    }
}
