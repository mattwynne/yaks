// Directory-based storage adapter - implements .yaks/ directory structure

use crate::domain::field::RESERVED_FIELDS;
use crate::domain::ports::{ReadYakStore, WriteYakStore};
use crate::domain::slug::{slugify, Name, YakId};
use crate::domain::{Yak, CONTEXT_FIELD, ID_FIELD, NAME_FIELD, STATE_FIELD};
use anyhow::{Context, Result};
use std::collections::HashMap;
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

    /// Resolve a yak's directory by name or id.
    /// Tries: direct path, resolve by id, resolve by name,
    /// resolve hierarchical name (in that order).
    fn yak_dir(&self, key: &str) -> PathBuf {
        // Try direct path first (backward compat: dir name == yak name)
        let direct = self.base_path.join(key);
        if direct.exists() {
            return direct;
        }

        // Try resolve by id (finds nested id-based dirs)
        if let Some(dir) = self.resolve_by_id(key) {
            return dir;
        }

        // Try resolve by leaf name (scans name files)
        if let Some(dir) = self.resolve_by_name(key) {
            return dir;
        }

        // Try resolve hierarchical name by walking segments
        if key.contains('/') {
            if let Some(dir) = self.resolve_by_hierarchical_name(key) {
                return dir;
            }
        }

        // Fallback to direct path (will fail later with "not found")
        direct
    }

    /// Find a yak directory by its id, searching recursively.
    /// Reads the `id` file inside each yak directory and matches against that.
    /// Falls back to directory name matching for backward compat (yaks without id files).
    fn resolve_by_id(&self, id: &str) -> Option<PathBuf> {
        if !self.base_path.exists() {
            return None;
        }
        let mut fallback: Option<PathBuf> = None;
        for entry in WalkDir::new(&self.base_path)
            .min_depth(1)
            .into_iter()
            .filter_entry(|e| e.file_type().is_dir())
        {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };
            let path = entry.path();
            if !path.join(CONTEXT_FIELD).exists() {
                continue;
            }
            // Primary: match against id file contents
            let id_file = path.join(ID_FIELD);
            if id_file.exists() {
                if let Ok(stored_id) = fs::read_to_string(&id_file) {
                    if stored_id.trim() == id {
                        return Some(path.to_path_buf());
                    }
                }
            }
            // Fallback: match against directory name (backward compat)
            if fallback.is_none() && path.file_name().and_then(|n| n.to_str()) == Some(id) {
                fallback = Some(path.to_path_buf());
            }
        }
        fallback
    }

    /// Resolve a hierarchical name like "parent/child" by walking the segments,
    /// matching leaf name files at each level.
    fn resolve_by_hierarchical_name(&self, name: &str) -> Option<PathBuf> {
        let segments: Vec<&str> = name.split('/').collect();
        let mut current_dir = self.base_path.clone();

        for segment in &segments {
            let mut found = false;
            if let Ok(entries) = fs::read_dir(&current_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if !path.is_dir() {
                        continue;
                    }
                    let name_file = path.join(NAME_FIELD);
                    if name_file.exists() {
                        if let Ok(stored) = fs::read_to_string(&name_file) {
                            let leaf = stored.rsplit('/').next().unwrap_or(&stored);
                            if leaf == *segment {
                                current_dir = path;
                                found = true;
                                break;
                            }
                        }
                    }
                }
            }
            if !found {
                return None;
            }
        }

        if current_dir.join(CONTEXT_FIELD).exists() {
            Some(current_dir)
        } else {
            None
        }
    }

    /// Scan directories recursively for one whose name file matches the given name.
    fn resolve_by_name(&self, name: &str) -> Option<PathBuf> {
        if !self.base_path.exists() {
            return None;
        }
        for entry in WalkDir::new(&self.base_path)
            .min_depth(1)
            .into_iter()
            .filter_entry(|e| e.file_type().is_dir())
        {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };
            let path = entry.path();
            let name_file = path.join(NAME_FIELD);
            if name_file.exists() {
                if let Ok(stored_name) = fs::read_to_string(&name_file) {
                    if stored_name == name {
                        return Some(path.to_path_buf());
                    }
                }
            }
        }
        None
    }

    /// Read the yak ID from a directory's id file, falling back to dir name.
    fn read_id_from_dir(&self, dir: &std::path::Path, fallback: &str) -> YakId {
        fs::read_to_string(dir.join(ID_FIELD))
            .map(|s| YakId::from(s.trim().to_string()))
            .unwrap_or_else(|_| {
                YakId::from(
                    dir.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or(fallback)
                        .to_string(),
                )
            })
    }

    /// Read custom fields (non-reserved files) from a yak directory.
    fn read_custom_fields(&self, dir: &std::path::Path) -> HashMap<String, String> {
        let mut fields = HashMap::new();
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if !RESERVED_FIELDS.contains(&name) {
                        if let Ok(content) = fs::read_to_string(&path) {
                            fields.insert(name.to_string(), content);
                        }
                    }
                }
            }
        }
        fields
    }

    /// Read direct child yak IDs from subdirectories of a yak directory.
    fn read_children(&self, dir: &std::path::Path) -> Vec<YakId> {
        let mut children = Vec::new();
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }
                if !path.join(CONTEXT_FIELD).exists() {
                    continue;
                }
                let id = self.read_id_from_dir(
                    &path,
                    path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown"),
                );
                children.push(id);
            }
        }
        children
    }

    fn field_path(&self, name: &str, field_name: &str) -> PathBuf {
        self.yak_dir(name).join(field_name)
    }

    /// Build the full hierarchical name for a yak at the given path.
    /// Walks up parent directories, collecting leaf names from name files,
    /// so the directory structure determines hierarchy.
    fn build_full_name(&self, path: &std::path::Path) -> String {
        let mut parts = Vec::new();
        let mut current = path.to_path_buf();

        loop {
            let name_content = fs::read_to_string(current.join(NAME_FIELD)).unwrap_or_else(|_| {
                current
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string()
            });
            // Always extract just the leaf - the name file may contain a full
            // path (old format) or just the leaf (new format after rename/move).
            let leaf_name = name_content
                .rsplit('/')
                .next()
                .unwrap_or(&name_content)
                .to_string();
            parts.push(leaf_name);

            // Move up to parent directory
            match current.parent() {
                Some(parent) if parent != self.base_path && parent.join(CONTEXT_FIELD).exists() => {
                    current = parent.to_path_buf();
                }
                _ => break,
            }
        }

        parts.reverse();
        parts.join("/")
    }
}

impl WriteYakStore for DirectoryStorage {
    fn create_yak(&self, name: &str, id: &str, parent_id: Option<&str>) -> Result<()> {
        // Use slug (from name) as directory name for human readability.
        // Fall back to name directly for backward compat (empty id = legacy).
        let dir_name = if id.is_empty() {
            name.to_string()
        } else {
            slugify(name).to_string()
        };

        // Determine parent directory: base_path or parent's directory
        let parent_dir = match parent_id {
            Some(pid) => self
                .resolve_by_id(pid)
                .ok_or_else(|| anyhow::anyhow!("Parent yak '{}' not found", pid))?,
            None => self.base_path.clone(),
        };

        let dir = parent_dir.join(&dir_name);
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

        // Write id file so the immutable ID is stored inside the directory
        if !id.is_empty() {
            fs::write(dir.join(ID_FIELD), id)
                .with_context(|| format!("Failed to write id file for yak: {name}"))?;
        }

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

        if !from_dir.exists() {
            anyhow::bail!("yak '{from}' not found");
        }

        // Compute new slug-based directory name
        let new_slug = slugify(to).to_string();

        // Target directory is in the same parent as the current directory
        let parent_dir = from_dir
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Cannot determine parent directory for '{from}'"))?;
        let to_dir = parent_dir.join(&new_slug);

        if to_dir.exists() {
            anyhow::bail!("Yak '{to}' already exists");
        }

        fs::rename(&from_dir, &to_dir)
            .with_context(|| format!("Failed to rename '{from}' to '{to}'"))?;

        // Update name file to reflect new name
        fs::write(to_dir.join(NAME_FIELD), to)
            .with_context(|| format!("Failed to update name file for '{to}'"))?;

        Ok(())
    }

    fn reparent_yak(&self, id: &str, new_parent_id: Option<&str>) -> Result<()> {
        let current_dir = self
            .resolve_by_id(id)
            .ok_or_else(|| anyhow::anyhow!("yak '{}' not found", id))?;

        let new_parent_dir = match new_parent_id {
            Some(pid) => self
                .resolve_by_id(pid)
                .ok_or_else(|| anyhow::anyhow!("parent yak '{}' not found", pid))?,
            None => self.base_path.clone(),
        };

        // Preserve the existing slug-based directory name when moving
        let dir_name = current_dir
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("Cannot determine directory name for '{}'", id))?;
        let new_dir = new_parent_dir.join(dir_name);
        if new_dir.exists() {
            anyhow::bail!("Target location already exists for '{}'", id);
        }

        fs::rename(&current_dir, &new_dir)
            .with_context(|| format!("Failed to move yak '{}' to new parent", id))?;

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
    fn get_yak(&self, id: &YakId) -> Result<Yak> {
        let dir = self
            .resolve_by_id(id.as_str())
            .or_else(|| {
                // Fallback: try yak_dir resolution for backward compat
                let d = self.yak_dir(id.as_str());
                if d.exists() && d.join(CONTEXT_FIELD).exists() {
                    Some(d)
                } else {
                    None
                }
            })
            .ok_or_else(|| anyhow::anyhow!("yak '{}' not found", id))?;

        let display_name = self.build_full_name(&dir);

        let context = fs::read_to_string(dir.join(CONTEXT_FIELD))
            .ok()
            .and_then(|c| if c.is_empty() { None } else { Some(c) });

        let state = fs::read_to_string(dir.join(STATE_FIELD))
            .unwrap_or_else(|_| "todo".to_string())
            .trim()
            .to_string();

        let fields = self.read_custom_fields(&dir);
        let children = self.read_children(&dir);

        Ok(Yak {
            id: id.clone(),
            name: Name::from(display_name),
            state,
            context,
            fields,
            children,
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

            // Build hierarchical name from directory structure and leaf name files
            let display_name = self.build_full_name(path);

            // Read id from id file, fall back to directory name (backward compat)
            let id = fs::read_to_string(path.join(ID_FIELD))
                .unwrap_or_else(|_| {
                    path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or(&display_name)
                        .to_string()
                })
                .trim()
                .to_string();

            let context = fs::read_to_string(path.join(CONTEXT_FIELD))
                .ok()
                .and_then(|c| if c.is_empty() { None } else { Some(c) });

            let state = fs::read_to_string(path.join(STATE_FIELD))
                .unwrap_or_else(|_| "todo".to_string())
                .trim()
                .to_string();

            let fields = self.read_custom_fields(path);
            let children = self.read_children(path);

            yaks.push(Yak {
                id: YakId::from(id),
                name: Name::from(display_name),
                state,
                context,
                fields,
                children,
            });
        }

        Ok(yaks)
    }

    fn yak_exists(&self, name: &str) -> bool {
        let context_file = self.field_path(name, CONTEXT_FIELD);
        context_file.exists()
    }

    fn fuzzy_find_yak_id(&self, query: &str) -> Result<YakId> {
        // First, try exact match via resolution (handles both old and new format)
        let dir = self.yak_dir(query);
        if dir.exists() && dir.join(CONTEXT_FIELD).exists() {
            let id = self.read_id_from_dir(&dir, query);
            return Ok(id);
        }

        // If not found, try fuzzy match on the leaf node only
        let yaks = ReadYakStore::list_yaks(self)?;
        let matches: Vec<&Yak> = yaks
            .iter()
            .filter(|yak| {
                let yak_name_str = yak.name.as_str();
                let leaf = yak_name_str.rsplit('/').next().unwrap_or(yak_name_str);
                leaf.to_lowercase().contains(&query.to_lowercase())
            })
            .collect();

        match matches.len() {
            0 => anyhow::bail!("yak '{query}' not found"),
            1 => Ok(matches[0].id.clone()),
            _ => anyhow::bail!("yak name '{query}' is ambiguous"),
        }
    }

    fn read_field(&self, id: &YakId, field_name: &str) -> Result<String> {
        let dir = self
            .resolve_by_id(id.as_str())
            .or_else(|| {
                let d = self.yak_dir(id.as_str());
                if d.exists() {
                    Some(d)
                } else {
                    None
                }
            })
            .ok_or_else(|| anyhow::anyhow!("yak '{}' not found", id))?;

        let field_path = dir.join(field_name);
        fs::read_to_string(&field_path)
            .with_context(|| format!("Failed to read field '{field_name}' for '{id}'"))
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
            name: Name::from("test"),
            id: YakId::from(""),
            parent_id: None,
        });

        storage.on_event(&event).unwrap();

        assert!(storage.yak_dir("test").exists());
        let yak = ReadYakStore::get_yak(&storage, &YakId::from("test")).unwrap();
        assert_eq!(yak.state, "todo");
    }

    #[test]
    fn test_directory_storage_handles_context_updated_event() {
        let (mut storage, _temp) = setup_test_storage();

        // First add the yak
        storage
            .on_event(&YakEvent::Added(AddedEvent {
                name: Name::from("test"),
                id: YakId::from(""),
                parent_id: None,
            }))
            .unwrap();

        // Then update context
        storage
            .on_event(&YakEvent::ContextUpdated(ContextUpdatedEvent {
                id: YakId::from("test"),
                content: "new context".to_string(),
            }))
            .unwrap();

        let yak = ReadYakStore::get_yak(&storage, &YakId::from("test")).unwrap();
        assert_eq!(yak.context, Some("new context".to_string()));
    }

    #[test]
    fn test_directory_storage_handles_state_updated_event() {
        let (mut storage, _temp) = setup_test_storage();

        storage
            .on_event(&YakEvent::Added(AddedEvent {
                name: Name::from("test"),
                id: YakId::from(""),
                parent_id: None,
            }))
            .unwrap();

        storage
            .on_event(&YakEvent::StateUpdated(StateUpdatedEvent {
                id: YakId::from("test"),
                state: "wip".to_string(),
            }))
            .unwrap();

        let yak = ReadYakStore::get_yak(&storage, &YakId::from("test")).unwrap();
        assert_eq!(yak.state, "wip");
    }

    #[test]
    fn test_directory_storage_read_yak_store_get_yak() {
        let (mut storage, _temp) = setup_test_storage();

        storage
            .on_event(&YakEvent::Added(AddedEvent {
                name: Name::from("test"),
                id: YakId::from(""),
                parent_id: None,
            }))
            .unwrap();

        storage
            .on_event(&YakEvent::ContextUpdated(ContextUpdatedEvent {
                id: YakId::from("test"),
                content: "context".to_string(),
            }))
            .unwrap();

        let yak = ReadYakStore::get_yak(&storage, &YakId::from("test")).unwrap();
        assert_eq!(yak.name, Name::from("test"));
        assert_eq!(yak.state, "todo");
        assert_eq!(yak.context, Some("context".to_string()));
    }

    #[test]
    fn test_directory_storage_read_yak_store_yak_exists() {
        let (mut storage, _temp) = setup_test_storage();

        storage
            .on_event(&YakEvent::Added(AddedEvent {
                name: Name::from("test"),
                id: YakId::from(""),
                parent_id: None,
            }))
            .unwrap();

        assert!(ReadYakStore::yak_exists(&storage, "test"));
        assert!(!ReadYakStore::yak_exists(&storage, "missing"));
    }

    #[test]
    fn test_added_event_with_id_creates_slug_based_directory() {
        let (mut storage, _temp) = setup_test_storage();

        let event = YakEvent::Added(AddedEvent {
            name: Name::from("my yak"),
            id: YakId::from("my-yak-a1b2"),
            parent_id: None,
        });

        storage.on_event(&event).unwrap();

        // Directory should be named by slug (from name), not id
        assert!(
            storage.base_path.join("my-yak").exists(),
            "Expected directory 'my-yak' (slug of 'my yak') to exist"
        );
        // get_yak should resolve by name
        let yak = ReadYakStore::get_yak(&storage, &YakId::from("my-yak-a1b2")).unwrap();
        assert_eq!(yak.id, YakId::from("my-yak-a1b2"));
        assert_eq!(yak.name, Name::from("my yak"));
    }

    #[test]
    fn test_directory_storage_read_yak_store_list_yaks() {
        let (mut storage, _temp) = setup_test_storage();

        storage
            .on_event(&YakEvent::Added(AddedEvent {
                name: Name::from("test1"),
                id: YakId::from(""),
                parent_id: None,
            }))
            .unwrap();

        storage
            .on_event(&YakEvent::Added(AddedEvent {
                name: Name::from("test2"),
                id: YakId::from(""),
                parent_id: None,
            }))
            .unwrap();

        let yaks = ReadYakStore::list_yaks(&storage).unwrap();
        assert_eq!(yaks.len(), 2);
    }

    #[test]
    fn test_state_update_by_id() {
        let (mut storage, _temp) = setup_test_storage();

        // Add yak with id
        storage
            .on_event(&YakEvent::Added(AddedEvent {
                name: Name::from("my yak"),
                id: YakId::from("my-yak-a1b2"),
                parent_id: None,
            }))
            .unwrap();

        // Update state using id
        storage
            .on_event(&YakEvent::StateUpdated(StateUpdatedEvent {
                id: YakId::from("my-yak-a1b2"),
                state: "wip".to_string(),
            }))
            .unwrap();

        // Verify
        let yak = ReadYakStore::get_yak(&storage, &YakId::from("my-yak-a1b2")).unwrap();
        assert_eq!(yak.state, "wip");
    }

    #[test]
    fn test_child_yak_nested_under_parent_directory() {
        let (mut storage, _temp) = setup_test_storage();

        // Add parent
        storage
            .on_event(&YakEvent::Added(AddedEvent {
                name: Name::from("parent"),
                id: YakId::from("parent-a1b2"),
                parent_id: None,
            }))
            .unwrap();

        // Add child under parent
        storage
            .on_event(&YakEvent::Added(AddedEvent {
                name: Name::from("child"),
                id: YakId::from("child-c3d4"),
                parent_id: Some(YakId::from("parent-a1b2")),
            }))
            .unwrap();

        // Child directory should be nested under parent's slug-based directory
        assert!(
            storage.base_path.join("parent").join("child").exists(),
            "Expected child directory nested under parent"
        );

        // Both yaks should be retrievable
        let parent = ReadYakStore::get_yak(&storage, &YakId::from("parent-a1b2")).unwrap();
        assert_eq!(parent.id, YakId::from("parent-a1b2"));

        let child = ReadYakStore::get_yak(&storage, &YakId::from("child-c3d4")).unwrap();
        assert_eq!(child.id, YakId::from("child-c3d4"));
        assert_eq!(child.name, Name::from("parent/child"));
    }

    #[test]
    fn test_get_yak_populates_custom_fields() {
        let (mut storage, _temp) = setup_test_storage();

        storage
            .on_event(&YakEvent::Added(AddedEvent {
                name: Name::from("my yak"),
                id: YakId::from("my-yak-a1b2"),
                parent_id: None,
            }))
            .unwrap();

        // Write a custom field
        storage
            .on_event(&YakEvent::FieldUpdated(FieldUpdatedEvent {
                id: YakId::from("my-yak-a1b2"),
                field_name: "plan".to_string(),
                content: "Step 1\nStep 2".to_string(),
            }))
            .unwrap();

        let yak = ReadYakStore::get_yak(&storage, &YakId::from("my-yak-a1b2")).unwrap();
        assert_eq!(yak.fields.get("plan"), Some(&"Step 1\nStep 2".to_string()));
        // Reserved fields should not appear in custom fields
        assert!(!yak.fields.contains_key("state"));
        assert!(!yak.fields.contains_key("context.md"));
        assert!(!yak.fields.contains_key("name"));
        assert!(!yak.fields.contains_key("id"));
    }

    #[test]
    fn test_get_yak_populates_children() {
        let (mut storage, _temp) = setup_test_storage();

        storage
            .on_event(&YakEvent::Added(AddedEvent {
                name: Name::from("parent"),
                id: YakId::from("parent-a1b2"),
                parent_id: None,
            }))
            .unwrap();

        storage
            .on_event(&YakEvent::Added(AddedEvent {
                name: Name::from("child1"),
                id: YakId::from("child1-c3d4"),
                parent_id: Some(YakId::from("parent-a1b2")),
            }))
            .unwrap();

        storage
            .on_event(&YakEvent::Added(AddedEvent {
                name: Name::from("child2"),
                id: YakId::from("child2-e5f6"),
                parent_id: Some(YakId::from("parent-a1b2")),
            }))
            .unwrap();

        let parent = ReadYakStore::get_yak(&storage, &YakId::from("parent-a1b2")).unwrap();
        assert_eq!(parent.children.len(), 2);
        assert!(parent.children.contains(&YakId::from("child1-c3d4")));
        assert!(parent.children.contains(&YakId::from("child2-e5f6")));

        // Leaf yaks should have no children
        let child = ReadYakStore::get_yak(&storage, &YakId::from("child1-c3d4")).unwrap();
        assert!(child.children.is_empty());
    }

    #[test]
    fn test_list_yaks_populates_fields_and_children() {
        let (mut storage, _temp) = setup_test_storage();

        storage
            .on_event(&YakEvent::Added(AddedEvent {
                name: Name::from("parent"),
                id: YakId::from("parent-a1b2"),
                parent_id: None,
            }))
            .unwrap();

        storage
            .on_event(&YakEvent::Added(AddedEvent {
                name: Name::from("child"),
                id: YakId::from("child-c3d4"),
                parent_id: Some(YakId::from("parent-a1b2")),
            }))
            .unwrap();

        storage
            .on_event(&YakEvent::FieldUpdated(FieldUpdatedEvent {
                id: YakId::from("child-c3d4"),
                field_name: "spec".to_string(),
                content: "some spec".to_string(),
            }))
            .unwrap();

        let yaks = ReadYakStore::list_yaks(&storage).unwrap();
        let parent = yaks
            .iter()
            .find(|y| y.id == YakId::from("parent-a1b2"))
            .unwrap();
        let child = yaks
            .iter()
            .find(|y| y.id == YakId::from("child-c3d4"))
            .unwrap();

        assert_eq!(parent.children.len(), 1);
        assert!(parent.children.contains(&YakId::from("child-c3d4")));

        assert_eq!(child.fields.get("spec"), Some(&"some spec".to_string()));
        assert!(child.children.is_empty());
    }
}
