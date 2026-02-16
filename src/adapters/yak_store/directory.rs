// Directory-based storage adapter - implements .yaks/ directory structure

use crate::domain::ports::{ReadYakStore, WriteYakStore};
use crate::domain::{Yak, CONTEXT_FIELD, NAME_FIELD, STATE_FIELD};
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
        // Try direct path first (backward compat: dir name == yak name)
        let direct = self.base_path.join(name);
        if direct.exists() {
            return direct;
        }

        // Scan for id-based directory whose name file matches
        if let Some(dir) = self.resolve_by_name(name) {
            return dir;
        }

        // Fallback to direct path (will fail later with "not found")
        direct
    }

    /// Scan top-level directories for one whose name file matches the given name.
    fn resolve_by_name(&self, name: &str) -> Option<PathBuf> {
        let base = &self.base_path;
        if !base.exists() {
            return None;
        }
        for entry in fs::read_dir(base).ok()?.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let name_file = path.join(NAME_FIELD);
                if name_file.exists() {
                    if let Ok(stored_name) = fs::read_to_string(&name_file) {
                        if stored_name == name {
                            return Some(path);
                        }
                    }
                }
            }
        }
        None
    }

    fn field_path(&self, name: &str, field_name: &str) -> PathBuf {
        self.yak_dir(name).join(field_name)
    }
}

impl WriteYakStore for DirectoryStorage {
    fn create_yak(&self, name: &str, id: &str) -> Result<()> {
        // Use id as directory name if available, otherwise fall back to name
        let dir_name = if id.is_empty() { name } else { id };

        let dir = self.base_path.join(dir_name);
        if dir.join(CONTEXT_FIELD).exists() {
            anyhow::bail!("Yak '{}' already exists", name);
        }

        fs::create_dir_all(&dir)
            .with_context(|| format!("Failed to create yak directory: {dir_name}"))?;

        // Create empty context.md file by default
        fs::write(dir.join(CONTEXT_FIELD), "")
            .with_context(|| format!("Failed to create context.md for yak: {name}"))?;

        // Write name file for name→directory resolution
        fs::write(dir.join(NAME_FIELD), name)
            .with_context(|| format!("Failed to write name file for yak: {name}"))?;

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
        let to_dir = self.base_path.join(to);

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

        // Update name file to reflect new name
        fs::write(to_dir.join(NAME_FIELD), to)
            .with_context(|| format!("Failed to update name file for '{to}'"))?;

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

        if !dir.exists() || !dir.join(CONTEXT_FIELD).exists() {
            anyhow::bail!("yak '{name}' not found");
        }

        // Read display name from name file, fall back to directory name
        let display_name =
            fs::read_to_string(dir.join(NAME_FIELD)).unwrap_or_else(|_| name.to_string());

        // Derive id from directory name (last component of path)
        let id = dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(name)
            .to_string();

        // Read context field
        let context = fs::read_to_string(dir.join(CONTEXT_FIELD))
            .ok()
            .and_then(|c| if c.is_empty() { None } else { Some(c) });

        // Read state field, default to "todo" if not present
        let state = fs::read_to_string(dir.join(STATE_FIELD))
            .unwrap_or_else(|_| "todo".to_string())
            .trim()
            .to_string();

        Ok(Yak {
            id,
            name: display_name,
            state,
            context,
        })
    }

    fn list_yaks(&self) -> Result<Vec<Yak>> {
        let mut yaks = Vec::new();

        if !self.base_path.exists() {
            return Ok(yaks);
        }

        // Use WalkDir to recursively find all directories that are yaks
        for entry in WalkDir::new(&self.base_path)
            .min_depth(1)
            .into_iter()
            .filter_entry(|e| e.file_type().is_dir())
        {
            let entry = entry?;
            let path = entry.path();

            // Only process directories that have a context.md (are actual yaks)
            if !path.join(CONTEXT_FIELD).exists() {
                continue;
            }

            // Read display name from name file, fall back to relative path
            let display_name = fs::read_to_string(path.join(NAME_FIELD)).unwrap_or_else(|_| {
                path.strip_prefix(&self.base_path)
                    .unwrap_or(path)
                    .to_string_lossy()
                    .to_string()
            });

            // Derive id from directory name (last component)
            let id = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&display_name)
                .to_string();

            let context = fs::read_to_string(path.join(CONTEXT_FIELD))
                .ok()
                .and_then(|c| if c.is_empty() { None } else { Some(c) });

            let state = fs::read_to_string(path.join(STATE_FIELD))
                .unwrap_or_else(|_| "todo".to_string())
                .trim()
                .to_string();

            yaks.push(Yak {
                id,
                name: display_name,
                state,
                context,
            });
        }

        Ok(yaks)
    }

    fn yak_exists(&self, name: &str) -> bool {
        let context_file = self.field_path(name, CONTEXT_FIELD);
        context_file.exists()
    }

    fn find_yak(&self, name: &str) -> Result<String> {
        // First, try exact match via resolution (handles both old and new format)
        let dir = self.yak_dir(name);
        if dir.exists() && dir.join(CONTEXT_FIELD).exists() {
            // Read the actual display name from the name file
            let display_name =
                fs::read_to_string(dir.join(NAME_FIELD)).unwrap_or_else(|_| name.to_string());
            return Ok(display_name);
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
            parent_id: None,
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
                parent_id: None,
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
                parent_id: None,
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
                parent_id: None,
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
                parent_id: None,
            }))
            .unwrap();

        assert!(ReadYakStore::yak_exists(&storage, "test"));
        assert!(!ReadYakStore::yak_exists(&storage, "missing"));
    }

    #[test]
    fn test_added_event_with_id_creates_id_based_directory() {
        let (mut storage, _temp) = setup_test_storage();

        let event = YakEvent::Added(AddedEvent {
            name: "my yak".to_string(),
            id: "my-yak-a1b2".to_string(),
            parent_id: None,
        });

        storage.on_event(&event).unwrap();

        // Directory should be named by id, not name
        assert!(
            storage.base_path.join("my-yak-a1b2").exists(),
            "Expected directory 'my-yak-a1b2' to exist"
        );
        // get_yak should resolve by name
        let yak = ReadYakStore::get_yak(&storage, "my yak").unwrap();
        assert_eq!(yak.id, "my-yak-a1b2");
        assert_eq!(yak.name, "my yak");
    }

    #[test]
    fn test_directory_storage_read_yak_store_list_yaks() {
        let (mut storage, _temp) = setup_test_storage();

        storage
            .on_event(&YakEvent::Added(AddedEvent {
                name: "test1".to_string(),
                id: String::new(),
                parent_id: None,
            }))
            .unwrap();

        storage
            .on_event(&YakEvent::Added(AddedEvent {
                name: "test2".to_string(),
                id: String::new(),
                parent_id: None,
            }))
            .unwrap();

        let yaks = ReadYakStore::list_yaks(&storage).unwrap();
        assert_eq!(yaks.len(), 2);
    }
}
