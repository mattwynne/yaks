// Directory-based storage adapter - implements .yaks/ directory structure

use crate::domain::field::RESERVED_FIELDS;
use crate::domain::ports::{ReadYakStore, WriteYakStore};
use crate::domain::slug::{slugify, Name, YakId};
use crate::domain::event_metadata::{Author, Timestamp};
use crate::domain::{Yak, CONTEXT_FIELD, ID_FIELD, NAME_FIELD, STATE_FIELD};
use crate::infrastructure::check_yaks_gitignored;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Clone)]
pub struct DirectoryStorage {
    base_path: PathBuf,
}

impl DirectoryStorage {
    /// Create a DirectoryStorage using the provided git repo root and yaks path.
    /// Checks that .yaks is gitignored before proceeding.
    pub fn new(repo_root: &Path, yaks_path: &Path) -> Result<Self> {
        check_yaks_gitignored(repo_root)?;
        Ok(Self {
            base_path: yaks_path.to_path_buf(),
        })
    }

    /// Create a DirectoryStorage without any git checks.
    /// Used when YX_SKIP_GIT_CHECKS is set and no git repo is available.
    pub fn without_git(yaks_path: &Path) -> Result<Self> {
        Ok(Self {
            base_path: yaks_path.to_path_buf(),
        })
    }

    /// Creates a DirectoryStorage with an explicit path, bypassing all checks.
    /// This is intended for testing only, where we want to use isolated temp
    /// directories without environment variable pollution.
    #[cfg(test)]
    pub(crate) fn from_path_unchecked(base_path: PathBuf) -> Self {
        Self { base_path }
    }

    /// Resolve a yak's directory by name or id.
    /// Tries: direct path, resolve by id, resolve by name (in that order).
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

    /// Read the parent yak's ID from the filesystem.
    /// If the parent directory is also a yak (has context.md), read its id file.
    fn read_parent_id(&self, dir: &std::path::Path) -> Option<YakId> {
        dir.parent().and_then(|parent| {
            if parent != self.base_path && parent.join(CONTEXT_FIELD).exists() {
                let fallback = parent
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown");
                Some(self.read_id_from_dir(parent, fallback))
            } else {
                None
            }
        })
    }

    /// Read created_by and created_at from .metadata.json in a yak directory.
    /// Returns (Author::unknown(), Timestamp::zero()) if the file is absent or unparseable.
    fn read_metadata(dir: &Path) -> (Author, Timestamp) {
        let metadata_path = dir.join(".metadata.json");
        if let Ok(content) = fs::read_to_string(&metadata_path) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                let author = Author {
                    name: json["created_by"]["name"]
                        .as_str()
                        .unwrap_or("unknown")
                        .to_string(),
                    email: json["created_by"]["email"]
                        .as_str()
                        .unwrap_or("")
                        .to_string(),
                };
                let timestamp = Timestamp(json["created_at"].as_i64().unwrap_or(0));
                return (author, timestamp);
            }
        }
        (Author::unknown(), Timestamp::zero())
    }

    /// Read the leaf name for a yak at the given path.
    /// Returns the content of the name file, or falls back to the directory name.
    fn read_leaf_name(&self, path: &std::path::Path) -> String {
        fs::read_to_string(path.join(NAME_FIELD)).unwrap_or_else(|_| {
            path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string()
        })
    }
}

impl DirectoryStorage {
    /// Remove all yak directories from the base path.
    /// A directory is a yak if it contains a `context.md` file.
    /// Non-yak files (e.g. `.schema-version`) are preserved.
    pub fn clear(&self) -> Result<()> {
        if !self.base_path.exists() {
            fs::create_dir_all(&self.base_path)?;
            return Ok(());
        }
        for entry in fs::read_dir(&self.base_path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() && path.join(CONTEXT_FIELD).exists() {
                fs::remove_dir_all(&path)?;
            }
        }
        Ok(())
    }
}

impl WriteYakStore for DirectoryStorage {
    fn create_yak(&self, name: &Name, id: &YakId, parent_id: Option<&YakId>) -> Result<()> {
        // Use slug (from name) as directory name for human readability.
        // Fall back to name directly for backward compat (empty id = legacy).
        let dir_name = if id.as_str().is_empty() {
            name.as_str().to_string()
        } else {
            slugify(name.as_str()).to_string()
        };

        // Determine parent directory: base_path or parent's directory
        let parent_dir = match parent_id {
            Some(pid) => self
                .resolve_by_id(pid.as_str())
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
        fs::write(dir.join(NAME_FIELD), name.as_str())
            .with_context(|| format!("Failed to write name file for yak: {name}"))?;

        // Write id file so the immutable ID is stored inside the directory
        if !id.as_str().is_empty() {
            fs::write(dir.join(ID_FIELD), id.as_str())
                .with_context(|| format!("Failed to write id file for yak: {name}"))?;
        }

        Ok(())
    }

    fn delete_yak(&self, id: &YakId) -> Result<()> {
        let dir = self.yak_dir(id.as_str());
        if dir.exists() {
            fs::remove_dir_all(&dir).with_context(|| format!("Failed to remove yak '{id}'"))?;
        }
        Ok(())
    }

    fn rename_yak(&self, id: &YakId, new_name: &Name) -> Result<()> {
        let from_dir = self.yak_dir(id.as_str());

        if !from_dir.exists() {
            anyhow::bail!("yak '{}' not found", id);
        }

        // Compute new slug-based directory name
        let new_slug = slugify(new_name.as_str()).to_string();

        // Target directory is in the same parent as the current directory
        let parent_dir = from_dir
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Cannot determine parent directory for '{}'", id))?;
        let to_dir = parent_dir.join(&new_slug);

        if to_dir == from_dir {
            // Slug unchanged - just update the name file
            fs::write(from_dir.join(NAME_FIELD), new_name.as_str())
                .with_context(|| format!("Failed to update name file for '{}'", new_name))?;
            return Ok(());
        }

        if to_dir.exists() {
            anyhow::bail!("Yak '{}' already exists", new_name);
        }

        fs::rename(&from_dir, &to_dir)
            .with_context(|| format!("Failed to rename '{}' to '{}'", id, new_name))?;

        // Update name file to reflect new name
        fs::write(to_dir.join(NAME_FIELD), new_name.as_str())
            .with_context(|| format!("Failed to update name file for '{}'", new_name))?;

        Ok(())
    }

    fn reparent_yak(&self, id: &YakId, new_parent_id: Option<&YakId>) -> Result<()> {
        let current_dir = self
            .resolve_by_id(id.as_str())
            .ok_or_else(|| anyhow::anyhow!("yak '{}' not found", id))?;

        let new_parent_dir = match new_parent_id {
            Some(pid) => self
                .resolve_by_id(pid.as_str())
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

    fn write_field(&self, id: &YakId, field_name: &str, content: &str) -> Result<()> {
        let dir = self.yak_dir(id.as_str());
        if !dir.exists() {
            anyhow::bail!("yak '{}' not found", id);
        }
        let field_path = dir.join(field_name);
        fs::write(&field_path, content)
            .with_context(|| format!("Failed to write field '{field_name}' for '{id}'"))
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

        let display_name = self.read_leaf_name(&dir);

        let context = fs::read_to_string(dir.join(CONTEXT_FIELD))
            .ok()
            .and_then(|c| if c.is_empty() { None } else { Some(c) });

        let state = fs::read_to_string(dir.join(STATE_FIELD))
            .unwrap_or_else(|_| "todo".to_string())
            .trim()
            .to_string();

        let fields = self.read_custom_fields(&dir);
        let children = self.read_children(&dir);
        let parent_id = self.read_parent_id(&dir);
        let (created_by, created_at) = Self::read_metadata(&dir);

        Ok(Yak {
            id: id.clone(),
            name: Name::from(display_name),
            parent_id,
            state,
            context,
            fields,
            children,
            created_by,
            created_at,
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
            let display_name = self.read_leaf_name(path);

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
            let parent_id = self.read_parent_id(path);
            let (created_by, created_at) = Self::read_metadata(path);

            yaks.push(Yak {
                id: YakId::from(id),
                name: Name::from(display_name),
                parent_id,
                state,
                context,
                fields,
                children,
                created_by,
                created_at,
            });
        }

        Ok(yaks)
    }

    fn fuzzy_find_yak_id(&self, query: &str) -> Result<YakId> {
        // First, try exact match via resolution (handles both old and new format)
        let dir = self.yak_dir(query);
        if dir.exists() && dir.join(CONTEXT_FIELD).exists() {
            let id = self.read_id_from_dir(&dir, query);
            return Ok(id);
        }

        // If not found, try fuzzy match on the name
        let yaks = ReadYakStore::list_yaks(self)?;
        let matches: Vec<&Yak> = yaks
            .iter()
            .filter(|yak| {
                yak.name
                    .as_str()
                    .to_lowercase()
                    .contains(&query.to_lowercase())
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
    use crate::domain::event_metadata::EventMetadata;
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
    fn test_without_git_stores_provided_yaks_path() {
        let temp_dir = TempDir::new().unwrap();

        // without_git() should succeed without a git repo
        let result = DirectoryStorage::without_git(temp_dir.path());
        assert!(result.is_ok());
        assert_eq!(result.unwrap().base_path, temp_dir.path());
    }

    #[test]
    fn test_directory_storage_handles_added_event() {
        let (mut storage, _temp) = setup_test_storage();

        let event = YakEvent::Added(
            AddedEvent {
                name: Name::from("test"),
                id: YakId::from(""),
                parent_id: None,
            },
            EventMetadata::default_legacy(),
        );

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
            .on_event(&YakEvent::Added(
                AddedEvent {
                    name: Name::from("test"),
                    id: YakId::from(""),
                    parent_id: None,
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();

        // Then update context
        storage
            .on_event(&YakEvent::FieldUpdated(
                FieldUpdatedEvent {
                    id: YakId::from("test"),
                    field_name: "context.md".to_string(),
                    content: "new context".to_string(),
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();

        let yak = ReadYakStore::get_yak(&storage, &YakId::from("test")).unwrap();
        assert_eq!(yak.context, Some("new context".to_string()));
    }

    #[test]
    fn test_directory_storage_handles_state_updated_event() {
        let (mut storage, _temp) = setup_test_storage();

        storage
            .on_event(&YakEvent::Added(
                AddedEvent {
                    name: Name::from("test"),
                    id: YakId::from(""),
                    parent_id: None,
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();

        storage
            .on_event(&YakEvent::FieldUpdated(
                FieldUpdatedEvent {
                    id: YakId::from("test"),
                    field_name: "state".to_string(),
                    content: "wip".to_string(),
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();

        let yak = ReadYakStore::get_yak(&storage, &YakId::from("test")).unwrap();
        assert_eq!(yak.state, "wip");
    }

    #[test]
    fn test_directory_storage_read_yak_store_get_yak() {
        let (mut storage, _temp) = setup_test_storage();

        storage
            .on_event(&YakEvent::Added(
                AddedEvent {
                    name: Name::from("test"),
                    id: YakId::from(""),
                    parent_id: None,
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();

        storage
            .on_event(&YakEvent::FieldUpdated(
                FieldUpdatedEvent {
                    id: YakId::from("test"),
                    field_name: "context.md".to_string(),
                    content: "context".to_string(),
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();

        let yak = ReadYakStore::get_yak(&storage, &YakId::from("test")).unwrap();
        assert_eq!(yak.name, Name::from("test"));
        assert_eq!(yak.state, "todo");
        assert_eq!(yak.context, Some("context".to_string()));
    }

    #[test]
    fn test_added_event_with_id_creates_slug_based_directory() {
        let (mut storage, _temp) = setup_test_storage();

        let event = YakEvent::Added(
            AddedEvent {
                name: Name::from("my yak"),
                id: YakId::from("my-yak-a1b2"),
                parent_id: None,
            },
            EventMetadata::default_legacy(),
        );

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
            .on_event(&YakEvent::Added(
                AddedEvent {
                    name: Name::from("test1"),
                    id: YakId::from(""),
                    parent_id: None,
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();

        storage
            .on_event(&YakEvent::Added(
                AddedEvent {
                    name: Name::from("test2"),
                    id: YakId::from(""),
                    parent_id: None,
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();

        let yaks = ReadYakStore::list_yaks(&storage).unwrap();
        assert_eq!(yaks.len(), 2);
    }

    #[test]
    fn test_state_update_by_id() {
        let (mut storage, _temp) = setup_test_storage();

        // Add yak with id
        storage
            .on_event(&YakEvent::Added(
                AddedEvent {
                    name: Name::from("my yak"),
                    id: YakId::from("my-yak-a1b2"),
                    parent_id: None,
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();

        // Update state using id
        storage
            .on_event(&YakEvent::FieldUpdated(
                FieldUpdatedEvent {
                    id: YakId::from("my-yak-a1b2"),
                    field_name: "state".to_string(),
                    content: "wip".to_string(),
                },
                EventMetadata::default_legacy(),
            ))
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
            .on_event(&YakEvent::Added(
                AddedEvent {
                    name: Name::from("parent"),
                    id: YakId::from("parent-a1b2"),
                    parent_id: None,
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();

        // Add child under parent
        storage
            .on_event(&YakEvent::Added(
                AddedEvent {
                    name: Name::from("child"),
                    id: YakId::from("child-c3d4"),
                    parent_id: Some(YakId::from("parent-a1b2")),
                },
                EventMetadata::default_legacy(),
            ))
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
        assert_eq!(child.name, Name::from("child"));
    }

    #[test]
    fn test_get_yak_populates_custom_fields() {
        let (mut storage, _temp) = setup_test_storage();

        storage
            .on_event(&YakEvent::Added(
                AddedEvent {
                    name: Name::from("my yak"),
                    id: YakId::from("my-yak-a1b2"),
                    parent_id: None,
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();

        // Write a custom field
        storage
            .on_event(&YakEvent::FieldUpdated(
                FieldUpdatedEvent {
                    id: YakId::from("my-yak-a1b2"),
                    field_name: "plan".to_string(),
                    content: "Step 1\nStep 2".to_string(),
                },
                EventMetadata::default_legacy(),
            ))
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
            .on_event(&YakEvent::Added(
                AddedEvent {
                    name: Name::from("parent"),
                    id: YakId::from("parent-a1b2"),
                    parent_id: None,
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();

        storage
            .on_event(&YakEvent::Added(
                AddedEvent {
                    name: Name::from("child1"),
                    id: YakId::from("child1-c3d4"),
                    parent_id: Some(YakId::from("parent-a1b2")),
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();

        storage
            .on_event(&YakEvent::Added(
                AddedEvent {
                    name: Name::from("child2"),
                    id: YakId::from("child2-e5f6"),
                    parent_id: Some(YakId::from("parent-a1b2")),
                },
                EventMetadata::default_legacy(),
            ))
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
            .on_event(&YakEvent::Added(
                AddedEvent {
                    name: Name::from("parent"),
                    id: YakId::from("parent-a1b2"),
                    parent_id: None,
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();

        storage
            .on_event(&YakEvent::Added(
                AddedEvent {
                    name: Name::from("child"),
                    id: YakId::from("child-c3d4"),
                    parent_id: Some(YakId::from("parent-a1b2")),
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();

        storage
            .on_event(&YakEvent::FieldUpdated(
                FieldUpdatedEvent {
                    id: YakId::from("child-c3d4"),
                    field_name: "spec".to_string(),
                    content: "some spec".to_string(),
                },
                EventMetadata::default_legacy(),
            ))
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

    #[test]
    fn test_clear_removes_yak_directories() {
        let (mut storage, temp) = setup_test_storage();

        // Create two yaks
        storage
            .on_event(&YakEvent::Added(
                AddedEvent {
                    name: Name::from("yak one"),
                    id: YakId::from("yak-one-a1b2"),
                    parent_id: None,
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();
        storage
            .on_event(&YakEvent::Added(
                AddedEvent {
                    name: Name::from("yak two"),
                    id: YakId::from("yak-two-c3d4"),
                    parent_id: None,
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();

        // Add a non-yak file
        std::fs::write(temp.path().join(".schema-version"), "3").unwrap();

        assert_eq!(ReadYakStore::list_yaks(&storage).unwrap().len(), 2);

        storage.clear().unwrap();

        assert_eq!(ReadYakStore::list_yaks(&storage).unwrap().len(), 0);
        assert!(
            temp.path().join(".schema-version").exists(),
            "Non-yak files should be preserved"
        );
    }

    #[test]
    fn test_clear_on_nonexistent_directory() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("nonexistent");
        let storage = DirectoryStorage::from_path_unchecked(path.clone());

        storage.clear().unwrap();

        assert!(path.exists(), "Should create the directory");
    }

    #[test]
    fn test_get_yak_populates_created_by_and_created_at() {
        use crate::domain::event_metadata::{Author, EventMetadata, Timestamp};

        let (mut storage, _temp) = setup_test_storage();

        let metadata = EventMetadata::new(
            Author {
                name: "Creator".to_string(),
                email: "creator@test.com".to_string(),
            },
            Timestamp(1708300800),
        );
        storage
            .on_event(&YakEvent::Added(
                AddedEvent {
                    name: Name::from("my yak"),
                    id: YakId::from("my-yak-a1b2"),
                    parent_id: None,
                },
                metadata,
            ))
            .unwrap();

        let yak = ReadYakStore::get_yak(&storage, &YakId::from("my-yak-a1b2")).unwrap();
        assert_eq!(yak.created_by.name, "Creator");
        assert_eq!(yak.created_by.email, "creator@test.com");
        assert_eq!(yak.created_at, Timestamp(1708300800));
    }

    #[test]
    fn test_added_event_writes_metadata_json() {
        use crate::domain::event_metadata::{Author, EventMetadata, Timestamp};

        let (mut storage, temp) = setup_test_storage();

        let metadata = EventMetadata::new(
            Author {
                name: "Test".to_string(),
                email: "test@test.com".to_string(),
            },
            Timestamp(1708300800),
        );
        let event = YakEvent::Added(
            AddedEvent {
                name: Name::from("my yak"),
                id: YakId::from("my-yak-a1b2"),
                parent_id: None,
            },
            metadata,
        );

        storage.on_event(&event).unwrap();

        // The yak directory is slug-based (from name), not id-based
        let content =
            std::fs::read_to_string(temp.path().join("my-yak/.metadata.json")).unwrap();
        let json: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(json["created_by"]["name"], "Test");
        assert_eq!(json["created_by"]["email"], "test@test.com");
        assert_eq!(json["created_at"], 1708300800);
    }
}
