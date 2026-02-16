// Directory-based storage adapter - implements .yaks/ directory structure

use crate::domain::ports::{ReadYakStore, WriteYakStore};
use crate::domain::{Yak, CONTEXT_FIELD, STATE_FIELD};
use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use walkdir::WalkDir;

#[derive(Clone)]
pub struct DirectoryStorage {
    base_path: PathBuf,
}

impl DirectoryStorage {
    pub fn new() -> Result<Self> {
        // Skip git checks if YX_SKIP_GIT_CHECKS is set (for mutation testing and test environments)
        let skip_git_checks = std::env::var("YX_SKIP_GIT_CHECKS").is_ok();

        if !skip_git_checks {
            // Check 1: Is git command available?
            Self::check_git_available()?;

            // Check 2: Are we in a git repository?
            Self::check_in_git_repo()?;

            // Check 3: Is .yaks gitignored?
            Self::check_yaks_gitignored()?;
        }

        // Priority: YAK_PATH env var, then GIT_WORK_TREE/.yaks, then .yaks
        // This matches bash version behavior: YAKS_PATH="$GIT_WORK_TREE/.yaks"
        let base_path = if let Ok(yak_path) = std::env::var("YAK_PATH") {
            yak_path.into()
        } else if let Ok(git_work_tree) = std::env::var("GIT_WORK_TREE") {
            PathBuf::from(git_work_tree).join(".yaks")
        } else {
            ".yaks".into()
        };

        Ok(Self { base_path })
    }

    /// Creates a DirectoryStorage with an explicit path, bypassing all checks.
    /// This is intended for testing only, where we want to use isolated temp
    /// directories without environment variable pollution.
    #[cfg(test)]
    pub(crate) fn from_path_unchecked(base_path: PathBuf) -> Self {
        Self { base_path }
    }

    fn check_git_available() -> Result<()> {
        // Try to run "git --version" to check if git command exists
        let output = Command::new("git").arg("--version").output();

        match output {
            Ok(_) => Ok(()),
            Err(_) => anyhow::bail!("Error: git command not found"),
        }
    }

    fn check_in_git_repo() -> Result<()> {
        // Run "git rev-parse --git-dir" to check if we're in a git repository
        let output = Command::new("git")
            .arg("rev-parse")
            .arg("--git-dir")
            .output()
            .context("Failed to check git repository")?;

        if !output.status.success() {
            anyhow::bail!("Error: not in a git repository");
        }

        Ok(())
    }

    fn check_yaks_gitignored() -> Result<()> {
        // Run "git check-ignore .yaks" to verify .yaks is gitignored
        let output = Command::new("git")
            .arg("check-ignore")
            .arg(".yaks")
            .output()
            .context("Failed to check .yaks gitignore status")?;

        // git check-ignore returns exit code 0 if the path is ignored
        if !output.status.success() {
            anyhow::bail!("Error: .yaks folder is not gitignored");
        }

        Ok(())
    }

    fn yak_dir(&self, name: &str) -> PathBuf {
        self.base_path.join(name)
    }

    fn field_path(&self, name: &str, field_name: &str) -> PathBuf {
        self.yak_dir(name).join(field_name)
    }
}

impl WriteYakStore for DirectoryStorage {
    fn create_yak(&self, name: &str) -> Result<()> {
        if self.field_path(name, CONTEXT_FIELD).exists() {
            anyhow::bail!("Yak '{}' already exists", name);
        }

        let dir = self.yak_dir(name);
        fs::create_dir_all(&dir)
            .with_context(|| format!("Failed to create yak directory: {name}"))?;

        // Create empty context.md file by default
        let context_file = self.field_path(name, CONTEXT_FIELD);
        fs::write(&context_file, "")
            .with_context(|| format!("Failed to create context.md for yak: {name}"))?;

        Ok(())
    }

    fn delete_yak(&self, name: &str) -> Result<()> {
        let dir = self.yak_dir(name);
        if dir.exists() {
            fs::remove_dir_all(&dir).with_context(|| format!("Failed to remove yak '{name}'"))?;
        }
        Ok(())
    }

    fn rename_yak(&self, from: &str, to: &str) -> Result<()> {
        let from_dir = self.yak_dir(from);
        let to_dir = self.yak_dir(to);

        if !from_dir.exists() {
            anyhow::bail!("yak '{from}' not found");
        }

        if to_dir.exists() {
            anyhow::bail!("Yak '{to}' already exists");
        }

        // Create implicit parent directories if needed
        if let Some(parent) = to_dir.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("Failed to create parent directories for '{to}'"))?;
            }
        }

        fs::rename(&from_dir, &to_dir)
            .with_context(|| format!("Failed to rename '{from}' to '{to}'"))?;

        Ok(())
    }

    fn write_field(&self, yak_name: &str, field_name: &str, content: &str) -> Result<()> {
        let dir = self.yak_dir(yak_name);
        if !dir.exists() {
            anyhow::bail!("yak '{}' not found", yak_name);
        }
        let field_path = self.field_path(yak_name, field_name);
        fs::write(&field_path, content)
            .with_context(|| format!("Failed to write field '{field_name}' for '{yak_name}'"))
    }
}

impl ReadYakStore for DirectoryStorage {
    fn get_yak(&self, name: &str) -> Result<Yak> {
        let dir = self.yak_dir(name);
        let context_file = self.field_path(name, CONTEXT_FIELD);

        if !dir.exists() || !context_file.exists() {
            anyhow::bail!("yak '{name}' not found");
        }

        // Read context field
        let context = ReadYakStore::read_field(self, name, CONTEXT_FIELD)
            .ok()
            .and_then(|c| if c.is_empty() { None } else { Some(c) });

        // Read state field, default to "todo" if not present
        let state = ReadYakStore::read_field(self, name, STATE_FIELD)
            .unwrap_or_else(|_| "todo".to_string())
            .trim()
            .to_string();

        // Derive done from state
        Ok(Yak {
            id: name.to_string(),
            name: name.to_string(),
            state,
            context,
        })
    }

    fn list_yaks(&self) -> Result<Vec<Yak>> {
        let mut yaks = Vec::new();

        if !self.base_path.exists() {
            return Ok(yaks);
        }

        // Use WalkDir to recursively find all directories (yaks)
        for entry in WalkDir::new(&self.base_path)
            .min_depth(1)
            .into_iter()
            .filter_entry(|e| e.file_type().is_dir())
        {
            let entry = entry?;
            // Get relative path from base_path
            if let Ok(rel_path) = entry.path().strip_prefix(&self.base_path) {
                if let Some(name) = rel_path.to_str() {
                    // Only add if we can successfully read it as a yak
                    if let Ok(yak) = ReadYakStore::get_yak(self, name) {
                        yaks.push(yak);
                    }
                }
            }
        }

        Ok(yaks)
    }

    fn yak_exists(&self, name: &str) -> bool {
        let context_file = self.field_path(name, CONTEXT_FIELD);
        context_file.exists()
    }

    fn find_yak(&self, name: &str) -> Result<String> {
        // First, try exact match - verify it's a real yak (has context.md)
        if self.field_path(name, CONTEXT_FIELD).exists() {
            return Ok(name.to_string());
        }

        // If not found, try fuzzy match on the leaf node only
        let yaks = ReadYakStore::list_yaks(self)?;
        let matches: Vec<&Yak> = yaks
            .iter()
            .filter(|yak| {
                // Extract leaf node (last segment after /)
                let leaf = yak.name.rsplit('/').next().unwrap_or(&yak.name);
                leaf.to_lowercase().contains(&name.to_lowercase())
            })
            .collect();

        match matches.len() {
            0 => anyhow::bail!("yak '{name}' not found"),
            1 => Ok(matches[0].name.clone()),
            _ => anyhow::bail!("yak name '{name}' is ambiguous"),
        }
    }

    fn read_field(&self, yak_name: &str, field_name: &str) -> Result<String> {
        let field_path = self.field_path(yak_name, field_name);
        fs::read_to_string(&field_path)
            .with_context(|| format!("Failed to read field '{field_name}' for '{yak_name}'"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::events::*;
    use crate::domain::ports::EventListener;
    use crate::domain::YakEvent;
    use tempfile::TempDir;

    fn setup_test_storage() -> (DirectoryStorage, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let storage = DirectoryStorage::from_path_unchecked(temp_dir.path().to_path_buf());
        (storage, temp_dir)
    }

    #[test]
    fn test_skip_git_checks_with_env_var() {
        // Save original env var state
        let original = std::env::var("YX_SKIP_GIT_CHECKS").ok();

        // Set YX_SKIP_GIT_CHECKS and YAK_PATH to use a temp directory
        let temp_dir = TempDir::new().unwrap();
        std::env::set_var("YX_SKIP_GIT_CHECKS", "1");
        std::env::set_var("YAK_PATH", temp_dir.path());

        // This should succeed even though we're not in a git repo
        let result = DirectoryStorage::new();
        assert!(result.is_ok());

        // Cleanup
        std::env::remove_var("YX_SKIP_GIT_CHECKS");
        std::env::remove_var("YAK_PATH");
        if let Some(val) = original {
            std::env::set_var("YX_SKIP_GIT_CHECKS", val);
        }
    }

    #[test]
    fn test_directory_storage_handles_added_event() {
        let (mut storage, _temp) = setup_test_storage();

        let event = YakEvent::Added(AddedEvent {
            name: "test".to_string(),
            id: String::new(),
        });

        storage.on_event(&event).unwrap();

        assert!(storage.yak_dir("test").exists());
        let yak = ReadYakStore::get_yak(&storage, "test").unwrap();
        assert_eq!(yak.state, "todo");
    }

    #[test]
    fn test_directory_storage_handles_context_updated_event() {
        let (mut storage, _temp) = setup_test_storage();

        // First add the yak
        storage
            .on_event(&YakEvent::Added(AddedEvent {
                name: "test".to_string(),
                id: String::new(),
            }))
            .unwrap();

        // Then update context
        storage
            .on_event(&YakEvent::ContextUpdated(ContextUpdatedEvent {
                name: "test".to_string(),
                content: "new context".to_string(),
            }))
            .unwrap();

        let yak = ReadYakStore::get_yak(&storage, "test").unwrap();
        assert_eq!(yak.context, Some("new context".to_string()));
    }

    #[test]
    fn test_directory_storage_handles_state_updated_event() {
        let (mut storage, _temp) = setup_test_storage();

        storage
            .on_event(&YakEvent::Added(AddedEvent {
                name: "test".to_string(),
                id: String::new(),
            }))
            .unwrap();

        storage
            .on_event(&YakEvent::StateUpdated(StateUpdatedEvent {
                name: "test".to_string(),
                state: "wip".to_string(),
            }))
            .unwrap();

        let yak = ReadYakStore::get_yak(&storage, "test").unwrap();
        assert_eq!(yak.state, "wip");
    }

    #[test]
    fn test_directory_storage_read_yak_store_get_yak() {
        let (mut storage, _temp) = setup_test_storage();

        storage
            .on_event(&YakEvent::Added(AddedEvent {
                name: "test".to_string(),
                id: String::new(),
            }))
            .unwrap();

        storage
            .on_event(&YakEvent::ContextUpdated(ContextUpdatedEvent {
                name: "test".to_string(),
                content: "context".to_string(),
            }))
            .unwrap();

        let yak = ReadYakStore::get_yak(&storage, "test").unwrap();
        assert_eq!(yak.name, "test");
        assert_eq!(yak.state, "todo");
        assert_eq!(yak.context, Some("context".to_string()));
    }

    #[test]
    fn test_directory_storage_read_yak_store_yak_exists() {
        let (mut storage, _temp) = setup_test_storage();

        storage
            .on_event(&YakEvent::Added(AddedEvent {
                name: "test".to_string(),
                id: String::new(),
            }))
            .unwrap();

        assert!(ReadYakStore::yak_exists(&storage, "test"));
        assert!(!ReadYakStore::yak_exists(&storage, "missing"));
    }

    #[test]
    fn test_directory_storage_read_yak_store_list_yaks() {
        let (mut storage, _temp) = setup_test_storage();

        storage
            .on_event(&YakEvent::Added(AddedEvent {
                name: "test1".to_string(),
                id: String::new(),
            }))
            .unwrap();

        storage
            .on_event(&YakEvent::Added(AddedEvent {
                name: "test2".to_string(),
                id: String::new(),
            }))
            .unwrap();

        let yaks = ReadYakStore::list_yaks(&storage).unwrap();
        assert_eq!(yaks.len(), 2);
    }
}
