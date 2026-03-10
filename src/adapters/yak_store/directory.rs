// Directory-based storage adapter - implements .yaks/ directory structure

mod fields;
mod io;
mod query;

use crate::domain::ports::{ReadYakStore, WriteYakStore};
use crate::domain::slug::{Name, YakId};
use anyhow::Result;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct DirectoryStorage {
    base_path: PathBuf,
}

impl DirectoryStorage {
    /// Create a DirectoryStorage using the provided git repo root and yaks path.
    pub fn new(_repo_root: &Path, yaks_path: &Path) -> Result<Self> {
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

    /// Remove all yak directories from the base path.
    /// A directory is a yak if it contains a `context.md` file.
    /// Non-yak files (e.g. `.schema-version`) are preserved.
    pub fn clear(&self) -> Result<()> {
        io::clear(&self.base_path)
    }
}

impl WriteYakStore for DirectoryStorage {
    fn create_yak(&self, name: &Name, id: &YakId, parent_id: Option<&YakId>) -> Result<()> {
        io::create_yak(&self.base_path, name, id, parent_id)
    }

    fn delete_yak(&self, id: &YakId) -> Result<()> {
        io::delete_yak(&self.base_path, id)
    }

    fn rename_yak(&self, id: &YakId, new_name: &Name) -> Result<()> {
        io::rename_yak(&self.base_path, id, new_name)
    }

    fn reparent_yak(&self, id: &YakId, new_parent_id: Option<&YakId>) -> Result<()> {
        io::reparent_yak(&self.base_path, id, new_parent_id)
    }

    fn write_field(&self, id: &YakId, field_name: &str, content: &str) -> Result<()> {
        io::write_field(&self.base_path, id, field_name, content)
    }

    fn clear_all(&self) -> Result<()> {
        self.clear()
    }
}

impl ReadYakStore for DirectoryStorage {
    fn get_yak(&self, id: &YakId) -> Result<crate::domain::Yak> {
        query::get_yak(&self.base_path, id)
    }

    fn list_yaks(&self) -> Result<Vec<crate::domain::Yak>> {
        query::list_yaks(&self.base_path)
    }

    fn fuzzy_find_yak_id(&self, query_str: &str) -> Result<YakId> {
        query::fuzzy_find_yak_id(&self.base_path, query_str)
    }

    fn read_field(&self, id: &YakId, field_name: &str) -> Result<String> {
        io::read_field(&self.base_path, id, field_name)
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

        assert!(query::yak_dir(&storage.base_path, "test").exists());
        let yak = ReadYakStore::get_yak(&storage, &YakId::from("test")).unwrap();
        assert_eq!(yak.state, crate::domain::YakState::Todo);
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
                    field_name: ".context.md".to_string(),
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
                    field_name: ".state".to_string(),
                    content: "wip".to_string(),
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();

        let yak = ReadYakStore::get_yak(&storage, &YakId::from("test")).unwrap();
        assert_eq!(yak.state, crate::domain::YakState::Wip);
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
                    field_name: ".context.md".to_string(),
                    content: "context".to_string(),
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();

        let yak = ReadYakStore::get_yak(&storage, &YakId::from("test")).unwrap();
        assert_eq!(yak.name, Name::from("test"));
        assert_eq!(yak.state, crate::domain::YakState::Todo);
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
                    field_name: ".state".to_string(),
                    content: "wip".to_string(),
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();

        // Verify
        let yak = ReadYakStore::get_yak(&storage, &YakId::from("my-yak-a1b2")).unwrap();
        assert_eq!(yak.state, crate::domain::YakState::Wip);
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
        assert!(!yak.fields.contains_key(".state"));
        assert!(!yak.fields.contains_key(".context.md"));
        assert!(!yak.fields.contains_key(".name"));
        assert!(!yak.fields.contains_key(".id"));
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

        // Verify parent-child relationships via parent_id
        let parent = ReadYakStore::get_yak(&storage, &YakId::from("parent-a1b2")).unwrap();
        assert_eq!(parent.parent_id, None);

        let child1 = ReadYakStore::get_yak(&storage, &YakId::from("child1-c3d4")).unwrap();
        assert_eq!(child1.parent_id, Some(YakId::from("parent-a1b2")));

        let child2 = ReadYakStore::get_yak(&storage, &YakId::from("child2-e5f6")).unwrap();
        assert_eq!(child2.parent_id, Some(YakId::from("parent-a1b2")));
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

        // Verify parent-child relationship
        assert_eq!(parent.parent_id, None);
        assert_eq!(child.parent_id, Some(YakId::from("parent-a1b2")));

        assert_eq!(child.fields.get("spec"), Some(&"some spec".to_string()));
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

    // --- Mutant coverage tests ---

    // Mutant 1: resolve_by_name line 109 `if !self.base_path.exists()`
    // Removing `!` would return None when base_path exists (wrong) and
    // proceed scanning when it doesn't exist (panic). This test verifies
    // that resolve_by_name works correctly when base_path does exist.
    // We look up by display name (not ID) so that resolve_by_id fails
    // and the lookup falls through to resolve_by_name.
    #[test]
    fn test_resolve_by_name_finds_yak_when_base_path_exists() {
        let (mut storage, _temp) = setup_test_storage();

        storage
            .on_event(&YakEvent::Added(
                AddedEvent {
                    name: Name::from("find me"),
                    id: YakId::from("find-me-a1b2"),
                    parent_id: None,
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();

        // Use display name "find me" as YakId — this won't match direct path
        // (directory is "find-me") or resolve_by_id (id is "find-me-a1b2"),
        // forcing the lookup through resolve_by_name.
        let yak = ReadYakStore::get_yak(&storage, &YakId::from("find me")).unwrap();
        assert_eq!(yak.name, Name::from("find me"));
    }

    // Mutant 2: read_parent_id line 197
    // `parent != self.base_path && parent.join(CONTEXT_FIELD).exists()`
    // Changing `&&` to `||` would treat base_path itself as a yak parent
    // and return Some(id) for top-level yaks instead of None.
    //
    // Without context.md in base_path: both && and || give false (same result).
    // With context.md in base_path: && gives false, || gives true (detectable!).
    #[test]
    fn test_read_parent_id_returns_none_for_top_level_yak() {
        let (mut storage, temp) = setup_test_storage();

        // Place a context.md in base_path itself so the || mutant is detectable.
        // With &&: parent == base_path → false && true = false → None (correct).
        // With ||: parent == base_path → false || true = true → Some(id) (wrong!).
        std::fs::write(temp.path().join(crate::domain::CONTEXT_FIELD), "").unwrap();

        storage
            .on_event(&YakEvent::Added(
                AddedEvent {
                    name: Name::from("top level"),
                    id: YakId::from("top-level-a1b2"),
                    parent_id: None,
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();

        let yak = ReadYakStore::get_yak(&storage, &YakId::from("top-level-a1b2")).unwrap();
        // Top-level yak should have no parent
        assert!(
            yak.parent_id.is_none(),
            "Top-level yak should have no parent_id, got {:?}",
            yak.parent_id
        );
    }

    // Also for mutant 2: verify child yak does get its parent_id set.
    #[test]
    fn test_read_parent_id_returns_parent_id_for_child_yak() {
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

        let child = ReadYakStore::get_yak(&storage, &YakId::from("child-c3d4")).unwrap();
        assert_eq!(
            child.parent_id,
            Some(YakId::from("parent-a1b2")),
            "Child yak should have parent_id set"
        );
    }

    // Mutant 3: clear line 256
    // `path.is_dir() && path.join(CONTEXT_FIELD).exists()`
    // Changing `&&` to `||` would also remove non-yak directories (files
    // pass `is_dir()` as false so only dirs-without-context.md get hit).
    // This test verifies that non-yak directories in base_path are preserved.
    #[test]
    fn test_clear_preserves_non_yak_directories() {
        let (mut storage, temp) = setup_test_storage();

        storage
            .on_event(&YakEvent::Added(
                AddedEvent {
                    name: Name::from("a yak"),
                    id: YakId::from("a-yak-a1b2"),
                    parent_id: None,
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();

        // Create a directory that is NOT a yak (no context.md)
        let non_yak_dir = temp.path().join("not-a-yak");
        std::fs::create_dir_all(&non_yak_dir).unwrap();

        assert!(non_yak_dir.exists(), "Setup: non-yak dir should exist");
        assert_eq!(ReadYakStore::list_yaks(&storage).unwrap().len(), 1);

        storage.clear().unwrap();

        // The yak should be gone
        assert_eq!(ReadYakStore::list_yaks(&storage).unwrap().len(), 0);
        // The non-yak directory must still be there
        assert!(
            non_yak_dir.exists(),
            "Non-yak directory should be preserved by clear()"
        );
    }

    // Mutant 4: get_yak line 397
    // `d.exists() && d.join(CONTEXT_FIELD).exists()`
    // Changing `&&` to `||` would allow get_yak to resolve a directory that
    // exists but has no context.md, causing a spurious "found" result.
    #[test]
    fn test_get_yak_returns_error_for_dir_without_context_md() {
        let (storage, temp) = setup_test_storage();

        // Create a directory with no context.md — not a valid yak
        let fake_dir = temp.path().join("fake-yak");
        std::fs::create_dir_all(&fake_dir).unwrap();

        let result = ReadYakStore::get_yak(&storage, &YakId::from("fake-yak"));
        assert!(
            result.is_err(),
            "get_yak should fail for a dir without context.md"
        );
    }

    // Mutant 5: fuzzy_find_yak_id line 502
    // `dir.exists() && dir.join(CONTEXT_FIELD).exists()`
    // Changing `&&` to `||` would match a directory that exists but has no
    // context.md, incorrectly treating it as a valid yak.
    #[test]
    fn test_fuzzy_find_yak_id_ignores_dir_without_context_md() {
        let (storage, temp) = setup_test_storage();

        // Create a directory with no context.md — not a valid yak
        let fake_dir = temp.path().join("ghost");
        std::fs::create_dir_all(&fake_dir).unwrap();

        let result = ReadYakStore::fuzzy_find_yak_id(&storage, "ghost");
        assert!(
            result.is_err(),
            "fuzzy_find_yak_id should not match a dir without context.md"
        );
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
        let content = std::fs::read_to_string(temp.path().join("my-yak/.created.json")).unwrap();
        let json: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(json["created_by"]["name"], "Test");
        assert_eq!(json["created_by"]["email"], "test@test.com");
        assert_eq!(json["created_at"], 1708300800);
    }

    #[test]
    fn test_rescue_children_saves_child_yaks_but_not_plain_directories() {
        let (mut storage, _temp) = setup_test_storage();

        // Add parent yak
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

        // Add child yak nested under parent
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

        // Create a plain (non-yak) subdirectory inside the parent
        let plain_dir = storage.base_path.join("parent").join("not-a-yak");
        std::fs::create_dir_all(&plain_dir).unwrap();
        std::fs::write(plain_dir.join("notes.txt"), "just a plain dir").unwrap();

        // Delete the parent — should rescue the child but not the plain dir
        WriteYakStore::delete_yak(&storage, &YakId::from("parent-a1b2")).unwrap();

        // Parent directory should be gone
        assert!(
            !storage.base_path.join("parent").exists(),
            "Parent directory should be removed after deletion"
        );

        // Child yak should be rescued to root level
        let child = ReadYakStore::get_yak(&storage, &YakId::from("child-c3d4")).unwrap();
        assert_eq!(child.name, Name::from("child"));

        // The plain non-yak directory should NOT exist at root level
        assert!(
            !storage.base_path.join("not-a-yak").exists(),
            "Plain (non-yak) directories should not be rescued"
        );
    }

    #[test]
    fn test_rescue_children_moves_child_to_root_when_target_does_not_exist() {
        let (mut storage, _temp) = setup_test_storage();

        // Add parent yak
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

        // Add child yak nested under parent
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

        // Verify child is nested before deletion
        assert!(
            storage.base_path.join("parent").join("child").exists(),
            "Child should be nested under parent before deletion"
        );
        assert!(
            !storage.base_path.join("child").exists(),
            "Child should NOT exist at root before parent deletion"
        );

        // Delete parent — child should be rescued to root
        WriteYakStore::delete_yak(&storage, &YakId::from("parent-a1b2")).unwrap();

        // Child should now be at root level
        assert!(
            storage.base_path.join("child").exists(),
            "Child should be rescued to root after parent deletion"
        );
        let child = ReadYakStore::get_yak(&storage, &YakId::from("child-c3d4")).unwrap();
        assert_eq!(child.name, Name::from("child"));
    }

    #[test]
    fn test_clear_all_removes_yak_directories() {
        let (mut storage, _temp) = setup_test_storage();

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

        assert_eq!(ReadYakStore::list_yaks(&storage).unwrap().len(), 2);

        // Call clear_all through the WriteYakStore trait (not storage.clear())
        WriteYakStore::clear_all(&storage).unwrap();

        assert_eq!(
            ReadYakStore::list_yaks(&storage).unwrap().len(),
            0,
            "clear_all should remove all yaks"
        );
    }

    #[test]
    fn test_tags_with_empty_lines_are_filtered() {
        let (storage, temp) = setup_test_storage();

        // Create a yak directory with tags file containing empty lines
        let yak_dir = temp.path().join("test-yak");
        std::fs::create_dir_all(&yak_dir).unwrap();
        std::fs::write(yak_dir.join(crate::domain::CONTEXT_FIELD), "").unwrap();
        std::fs::write(yak_dir.join(crate::domain::ID_FIELD), "test-yak-a1b2").unwrap();
        std::fs::write(yak_dir.join(crate::domain::NAME_FIELD), "test yak").unwrap();
        std::fs::write(
            yak_dir.join(crate::domain::TAGS_FIELD),
            "tag1\n\ntag2\n\n\ntag3\n",
        )
        .unwrap();

        // get_yak should filter out empty lines
        let yak = ReadYakStore::get_yak(&storage, &YakId::from("test-yak-a1b2")).unwrap();
        assert_eq!(yak.tags.len(), 3);
        assert_eq!(yak.tags, vec!["tag1", "tag2", "tag3"]);

        // list_yaks should also filter out empty lines
        let yaks = ReadYakStore::list_yaks(&storage).unwrap();
        assert_eq!(yaks.len(), 1);
        assert_eq!(yaks[0].tags.len(), 3);
        assert_eq!(yaks[0].tags, vec!["tag1", "tag2", "tag3"]);
    }
}
